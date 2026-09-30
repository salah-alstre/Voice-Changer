//! Per-application audio sessions on a render endpoint (WASAPI IAudioSessionManager2).
//!
//! `SessionPoller` keeps cached interface pointers per session so meters/volumes can be
//! sampled cheaply each tick; the session list is rebuilt when a new session is announced
//! (IAudioSessionNotification) or at least once per second.

use super::devices;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::{implement, Interface, Ref, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::{CoTaskMemFree, CLSCTX_ALL};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// Stable per-session identifier (WASAPI session instance id).
    pub id: String,
    pub pid: u32,
    pub name: String,
    pub exe_name: String,
    pub exe_path: String,
    pub state: String,
    pub is_system: bool,
    pub volume: f32,
    pub muted: bool,
    pub peak: f32,
    pub device_id: String,
}

struct Entry {
    info: SessionInfo,
    volume: ISimpleAudioVolume,
    meter: IAudioMeterInformation,
}

#[implement(IAudioSessionNotification)]
struct NewSession(Arc<AtomicBool>);

impl IAudioSessionNotification_Impl for NewSession_Impl {
    fn OnSessionCreated(&self, _new: Ref<'_, IAudioSessionControl>) -> windows::core::Result<()> {
        self.0.store(true, Ordering::Release);
        Ok(())
    }
}

pub struct SessionPoller {
    device_id: String,
    manager: IAudioSessionManager2,
    notif: IAudioSessionNotification,
    dirty: Arc<AtomicBool>,
    entries: Vec<Entry>,
    last_refresh: std::time::Instant,
    procs: HashMap<u32, (String, String)>,
}

impl Drop for SessionPoller {
    fn drop(&mut self) {
        unsafe {
            let _ = self.manager.UnregisterSessionNotification(&self.notif);
        }
    }
}

fn process_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let h: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(h);
        ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn take_pwstr(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let s = unsafe { p.to_string().unwrap_or_default() };
    unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
    s
}

fn pretty_exe(stem: &str) -> String {
    let mut c = stem.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

impl SessionPoller {
    pub fn new(device_id: Option<&str>) -> AppResult<Self> {
        let dev = devices::resolve(device_id, devices::Flow::Render)?;
        let id = devices::device_id(&dev)?;
        let manager = unsafe { dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)? };
        let dirty = Arc::new(AtomicBool::new(true));
        let notif: IAudioSessionNotification = NewSession(dirty.clone()).into();
        unsafe { manager.RegisterSessionNotification(&notif)? };
        Ok(Self {
            device_id: id,
            manager,
            notif,
            dirty,
            entries: vec![],
            last_refresh: std::time::Instant::now(),
            procs: HashMap::new(),
        })
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    fn rebuild(&mut self) -> AppResult<()> {
        let en = unsafe { self.manager.GetSessionEnumerator()? };
        let n = unsafe { en.GetCount()? };
        let mut entries = Vec::new();
        let mut seen_pids = Vec::new();
        for i in 0..n {
            let Ok(ctl) = (unsafe { en.GetSession(i) }) else {
                continue;
            };
            let Ok(c2) = ctl.cast::<IAudioSessionControl2>() else {
                continue;
            };
            let state = unsafe { ctl.GetState().unwrap_or(AudioSessionStateInactive) };
            if state == AudioSessionStateExpired {
                continue;
            }
            let Ok(volume) = ctl.cast::<ISimpleAudioVolume>() else {
                continue;
            };
            let Ok(meter) = ctl.cast::<IAudioMeterInformation>() else {
                continue;
            };
            let pid = unsafe { c2.GetProcessId().unwrap_or(0) };
            let is_system = unsafe { c2.IsSystemSoundsSession().0 == 0 };
            let id = unsafe {
                c2.GetSessionInstanceIdentifier()
                    .map(take_pwstr)
                    .unwrap_or_default()
            };
            if id.is_empty() {
                continue;
            }
            seen_pids.push(pid);
            let (exe_path, exe_name) = self
                .procs
                .entry(pid)
                .or_insert_with(|| {
                    let p = process_path(pid).unwrap_or_default();
                    let n = std::path::Path::new(&p)
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    (p, n)
                })
                .clone();
            let display = unsafe { ctl.GetDisplayName().map(take_pwstr).unwrap_or_default() };
            let name = if is_system {
                "System sounds".to_string()
            } else if !display.is_empty() && !display.starts_with('@') {
                display
            } else if !exe_name.is_empty() {
                pretty_exe(exe_name.trim_end_matches(".exe").trim_end_matches(".EXE"))
            } else {
                format!("Process {pid}")
            };
            let info = SessionInfo {
                id,
                pid,
                name,
                exe_name,
                exe_path,
                state: match state {
                    AudioSessionStateActive => "active",
                    AudioSessionStateInactive => "inactive",
                    _ => "expired",
                }
                .into(),
                is_system,
                volume: unsafe { volume.GetMasterVolume().unwrap_or(1.0) },
                muted: unsafe { volume.GetMute().map(|b| b.as_bool()).unwrap_or(false) },
                peak: 0.0,
                device_id: self.device_id.clone(),
            };
            entries.push(Entry {
                info,
                volume,
                meter,
            });
        }
        self.procs.retain(|pid, _| seen_pids.contains(pid));
        entries.sort_by(|a, b| {
            (b.info.state == "active")
                .cmp(&(a.info.state == "active"))
                .then_with(|| a.info.name.to_lowercase().cmp(&b.info.name.to_lowercase()))
                .then_with(|| a.info.id.cmp(&b.info.id))
        });
        self.entries = entries;
        self.last_refresh = std::time::Instant::now();
        Ok(())
    }

    /// Refresh (if needed) and sample current volume/mute/peak/state for every session.
    pub fn poll(&mut self) -> AppResult<Vec<SessionInfo>> {
        if self.dirty.swap(false, Ordering::AcqRel)
            || self.last_refresh.elapsed().as_millis() >= 1000
        {
            self.rebuild()?;
        }
        for e in &mut self.entries {
            e.info.peak = unsafe { e.meter.GetPeakValue().unwrap_or(0.0) };
            e.info.volume = unsafe { e.volume.GetMasterVolume().unwrap_or(e.info.volume) };
            e.info.muted = unsafe {
                e.volume
                    .GetMute()
                    .map(|b| b.as_bool())
                    .unwrap_or(e.info.muted)
            };
        }
        Ok(self.entries.iter().map(|e| e.info.clone()).collect())
    }

    fn find(&mut self, session_id: &str) -> AppResult<&Entry> {
        if !self.entries.iter().any(|e| e.info.id == session_id) {
            self.rebuild()?;
        }
        self.entries
            .iter()
            .find(|e| e.info.id == session_id)
            .ok_or_else(|| AppError::NotFound("audio session no longer exists".into()))
    }

    pub fn set_volume(&mut self, session_id: &str, v: f32) -> AppResult<()> {
        if !v.is_finite() {
            return Err(AppError::Invalid("volume must be finite".into()));
        }
        let e = self.find(session_id)?;
        unsafe {
            e.volume
                .SetMasterVolume(v.clamp(0.0, 1.0), std::ptr::null())?
        };
        Ok(())
    }

    pub fn set_mute(&mut self, session_id: &str, m: bool) -> AppResult<()> {
        let e = self.find(session_id)?;
        unsafe { e.volume.SetMute(m, std::ptr::null())? };
        Ok(())
    }
}

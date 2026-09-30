//! Shared application state, the background pollers that feed the UI, and the high level actions
//! (used by both the IPC commands and the global hotkeys / tray).

use crate::audio::devices::{self, EndpointMeter, Flow};
use crate::audio::engine::{EngineSnapshot, MonitorControls, MonitorEngine};
use crate::audio::recorder::{self, Player, PlayerStatus, Recorder, RecorderStatus};
use crate::audio::sessions::{SessionInfo, SessionPoller};
use crate::dsp::presets::{builtin_presets, preset_by_id};
use crate::dsp::VoiceParams;
use crate::error::{AppError, AppResult};
use crate::persistence;
use crate::profiles::{Profile, ProfileStore};
use crate::settings::Settings;
use crossbeam_channel::{bounded, unbounded, Receiver, RecvTimeoutError, Sender};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Media::Audio::eConsole;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PersistedVoice {
    params: VoiceParams,
    preset_id: Option<String>,
    controls: MonitorControls,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceState {
    pub params: VoiceParams,
    pub preset_id: Option<String>,
    pub controls: MonitorControls,
}

/// A recording kept in RAM for the A/B "record once, try presets" workflow.
pub struct Take {
    pub original: Arc<Vec<f32>>,
    pub processed: Option<(String, Arc<Vec<f32>>)>,
}

pub enum SessCmd {
    Volume(String, f32, Sender<AppResult<()>>),
    Mute(String, bool, Sender<AppResult<()>>),
    ApplyVolumes(BTreeMap<String, f32>),
    Refresh,
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub profiles: Mutex<ProfileStore>,
    pub voice_params: Mutex<VoiceParams>,
    pub preset_id: Mutex<Option<String>>,
    pub controls: Mutex<MonitorControls>,
    pub engine: Mutex<Option<MonitorEngine>>,
    pub recorder: Mutex<Option<Recorder>>,
    pub take: Mutex<Option<Take>>,
    pub player: Mutex<Option<Player>>,
    pub sessions: Mutex<Vec<SessionInfo>>,
    pub sess_tx: Sender<SessCmd>,
    sess_rx: Mutex<Option<Receiver<SessCmd>>>,
    pub device_changes: Arc<AtomicU64>,
    pub visible: AtomicBool,
    pub shutdown: AtomicBool,
    pub started: Instant,
}

impl AppState {
    pub fn load() -> Self {
        let dir = persistence::data_dir();
        let mut settings: Settings =
            persistence::load_json(&dir.join("settings.json")).unwrap_or_default();
        if settings.validate().is_err() {
            settings = Settings::default();
        }
        let mut profiles: ProfileStore =
            persistence::load_json(&dir.join("profiles.json")).unwrap_or_default();
        profiles.profiles.retain_mut(|p| p.sanitize().is_ok());
        let voice: PersistedVoice =
            persistence::load_json(&dir.join("voice.json")).unwrap_or_default();
        let (tx, rx) = unbounded();
        Self {
            settings: Mutex::new(settings),
            profiles: Mutex::new(profiles),
            voice_params: Mutex::new(voice.params.sanitized()),
            preset_id: Mutex::new(voice.preset_id),
            controls: Mutex::new(voice.controls.sanitized()),
            engine: Mutex::new(None),
            recorder: Mutex::new(None),
            take: Mutex::new(None),
            player: Mutex::new(None),
            sessions: Mutex::new(Vec::new()),
            sess_tx: tx,
            sess_rx: Mutex::new(Some(rx)),
            device_changes: Arc::new(AtomicU64::new(0)),
            visible: AtomicBool::new(true),
            shutdown: AtomicBool::new(false),
            started: Instant::now(),
        }
    }

    pub fn save_settings(&self) {
        let s = self.settings.lock().clone();
        if let Err(e) = persistence::save_json(&persistence::data_dir().join("settings.json"), &s) {
            tracing::warn!("could not save settings: {e}");
        }
    }

    pub fn save_profiles(&self) {
        let p = self.profiles.lock().clone();
        if let Err(e) = persistence::save_json(&persistence::data_dir().join("profiles.json"), &p) {
            tracing::warn!("could not save profiles: {e}");
        }
    }

    pub fn save_voice(&self) {
        let v = PersistedVoice {
            params: *self.voice_params.lock(),
            preset_id: self.preset_id.lock().clone(),
            controls: *self.controls.lock(),
        };
        if let Err(e) = persistence::save_json(&persistence::data_dir().join("voice.json"), &v) {
            tracing::warn!("could not save voice state: {e}");
        }
    }

    pub fn voice_state(&self) -> VoiceState {
        VoiceState {
            params: *self.voice_params.lock(),
            preset_id: self.preset_id.lock().clone(),
            controls: *self.controls.lock(),
        }
    }

    pub fn emit_voice(&self, app: &AppHandle) {
        let _ = app.emit("voice-changed", self.voice_state());
    }

    // ---- voice -------------------------------------------------------------------------------

    pub fn set_voice_params(&self, p: VoiceParams, preset: Option<String>) {
        let p = p.sanitized();
        *self.voice_params.lock() = p;
        *self.preset_id.lock() = preset;
        if let Some(e) = self.engine.lock().as_ref() {
            e.set_params(p);
        }
        self.save_voice();
    }

    pub fn apply_preset(&self, id: &str) -> AppResult<()> {
        let preset =
            preset_by_id(id).ok_or_else(|| AppError::NotFound(format!("unknown preset '{id}'")))?;
        self.set_voice_params(preset.params, Some(preset.id.to_string()));
        Ok(())
    }

    pub fn cycle_preset(&self, delta: i32) -> AppResult<String> {
        let presets = builtin_presets();
        let cur = self.preset_id.lock().clone();
        let idx = cur
            .and_then(|c| presets.iter().position(|p| p.id == c))
            .map(|i| i as i32)
            .unwrap_or(if delta >= 0 { -1 } else { 0 });
        let n = presets.len() as i32;
        let next = &presets[((idx + delta).rem_euclid(n)) as usize];
        self.set_voice_params(next.params, Some(next.id.to_string()));
        Ok(next.name.to_string())
    }

    pub fn set_controls(&self, c: MonitorControls) {
        let c = c.sanitized();
        *self.controls.lock() = c;
        if let Some(e) = self.engine.lock().as_ref() {
            e.set_controls(c);
        }
        self.save_voice();
    }

    // ---- monitoring --------------------------------------------------------------------------

    pub fn start_monitor(&self, input: Option<String>, output: Option<String>) -> AppResult<()> {
        let input = input.filter(|s| !s.is_empty());
        let output = output.filter(|s| !s.is_empty());
        let mut eng = self.engine.lock();
        if let Some(mut old) = eng.take() {
            old.stop();
        }
        let e = MonitorEngine::start(
            input.clone(),
            output.clone(),
            *self.voice_params.lock(),
            *self.controls.lock(),
        )?;
        *eng = Some(e);
        drop(eng);
        {
            let mut s = self.settings.lock();
            s.monitor_input_device = input;
            s.monitor_output_device = output;
        }
        self.save_settings();
        tracing::info!("monitoring started");
        Ok(())
    }

    pub fn stop_monitor(&self) {
        if let Some(mut e) = self.engine.lock().take() {
            e.stop();
            tracing::info!("monitoring stopped");
        }
    }

    /// Mutes and stops everything that is producing or capturing audio on the app's behalf.
    pub fn emergency_stop(&self) {
        if let Some(mut e) = self.engine.lock().take() {
            e.emergency_stop();
        }
        if let Some(mut p) = self.player.lock().take() {
            p.stop();
        }
        if let Some(r) = self.recorder.lock().take() {
            r.cancel();
        }
        // Next start comes up muted, so a restart can never blast audio unexpectedly.
        self.controls.lock().monitor_mute = true;
        self.save_voice();
        tracing::warn!("emergency stop");
    }

    // ---- endpoints ---------------------------------------------------------------------------

    pub fn default_id(flow: Flow) -> AppResult<String> {
        devices::default_device_id(flow, eConsole)
            .ok_or_else(|| AppError::NotFound("no default device".into()))
    }

    pub fn toggle_mute(flow: Flow) -> AppResult<bool> {
        let id = Self::default_id(flow)?;
        let next = !devices::get_endpoint_mute(&id)?;
        devices::set_endpoint_mute(&id, next)?;
        Ok(next)
    }

    // ---- profiles ----------------------------------------------------------------------------

    pub fn profile_from_current(&self, name: &str) -> Profile {
        let mut app_volumes = BTreeMap::new();
        for s in self
            .sessions
            .lock()
            .iter()
            .filter(|s| !s.is_system && !s.exe_name.is_empty())
        {
            app_volumes.insert(s.exe_name.to_lowercase(), s.volume);
        }
        let master_volume = Self::default_id(Flow::Render)
            .ok()
            .and_then(|id| endpoint_scalar(&id));
        let mic_muted = Self::default_id(Flow::Capture)
            .ok()
            .and_then(|id| devices::get_endpoint_mute(&id).ok());
        Profile {
            name: name.trim().to_string(),
            voice: *self.voice_params.lock(),
            voice_preset: self.preset_id.lock().clone(),
            master_volume,
            mic_gain_db: self.controls.lock().input_gain_db,
            mic_muted,
            app_volumes,
            ..Default::default()
        }
    }

    pub fn apply_profile(&self, p: &Profile) {
        self.set_voice_params(p.voice, p.voice_preset.clone());
        {
            let mut c = self.controls.lock();
            c.input_gain_db = p.mic_gain_db;
            let c2 = c.sanitized();
            *c = c2;
            if let Some(e) = self.engine.lock().as_ref() {
                e.set_controls(c2);
            }
        }
        self.save_voice();
        if let (Some(v), Ok(id)) = (p.master_volume, Self::default_id(Flow::Render)) {
            if let Err(e) = devices::set_endpoint_volume(&id, v) {
                tracing::warn!("profile: master volume not applied: {e}");
            }
        }
        if let (Some(m), Ok(id)) = (p.mic_muted, Self::default_id(Flow::Capture)) {
            if let Err(e) = devices::set_endpoint_mute(&id, m) {
                tracing::warn!("profile: mic mute not applied: {e}");
            }
        }
        if !p.app_volumes.is_empty() {
            let _ = self
                .sess_tx
                .send(SessCmd::ApplyVolumes(p.app_volumes.clone()));
        }
    }

    // ---- takes -------------------------------------------------------------------------------

    /// Processed version of the current take for `preset` (or the current settings), cached per key.
    pub fn processed_take(&self, preset: Option<&str>) -> AppResult<(String, Arc<Vec<f32>>)> {
        let (key, params) = match preset {
            Some(id) => (
                id.to_string(),
                preset_by_id(id)
                    .ok_or_else(|| AppError::NotFound(format!("unknown preset '{id}'")))?
                    .params,
            ),
            None => ("current".to_string(), *self.voice_params.lock()),
        };
        let original = {
            let t = self.take.lock();
            let t = t
                .as_ref()
                .ok_or_else(|| AppError::NotFound("no recording yet".into()))?;
            if let Some((k, v)) = &t.processed {
                if *k == key && preset.is_some() {
                    return Ok((k.clone(), v.clone()));
                }
            }
            t.original.clone()
        };
        let out = Arc::new(recorder::render_processed(&original, &params));
        if let Some(t) = self.take.lock().as_mut() {
            t.processed = Some((key.clone(), out.clone()));
        }
        Ok((key, out))
    }
}

fn endpoint_scalar(id: &str) -> Option<f32> {
    let dev = devices::open_device(id).ok()?;
    let ev = devices::endpoint_volume(&dev).ok()?;
    unsafe { ev.GetMasterVolumeLevelScalar().ok() }
}

// ---- background pollers ----------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EndpointLevel {
    device_id: Option<String>,
    peak: f32,
    volume: f32,
    muted: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Telemetry {
    master: EndpointLevel,
    mic: EndpointLevel,
    monitor: Option<EngineSnapshot>,
    recorder: Option<RecorderStatus>,
    player: Option<PlayerStatus>,
    has_take: bool,
    uptime_s: u64,
}

struct Tracked {
    flow: Flow,
    id: Option<String>,
    meter: Option<EndpointMeter>,
    volume: f32,
    muted: bool,
}

impl Tracked {
    fn new(flow: Flow) -> Self {
        Self {
            flow,
            id: None,
            meter: None,
            volume: 0.0,
            muted: false,
        }
    }

    /// Re-resolve the default endpoint (cheap) and reopen the meter when it changed.
    fn refresh_default(&mut self) {
        let id = devices::default_device_id(self.flow, eConsole);
        if id != self.id {
            self.meter = id.as_deref().and_then(|i| EndpointMeter::open(i).ok());
            self.id = id;
        }
    }

    fn refresh_volume(&mut self) {
        if let Some(id) = &self.id {
            if let Ok(dev) = devices::open_device(id) {
                if let Ok(ev) = devices::endpoint_volume(&dev) {
                    unsafe {
                        self.volume = ev.GetMasterVolumeLevelScalar().unwrap_or(self.volume);
                        self.muted = ev.GetMute().map(|b| b.as_bool()).unwrap_or(self.muted);
                    }
                }
            }
        }
    }

    fn level(&self) -> EndpointLevel {
        EndpointLevel {
            device_id: self.id.clone(),
            peak: self.meter.as_ref().map(|m| m.peak()).unwrap_or(0.0),
            volume: self.volume,
            muted: self.muted,
        }
    }
}

fn tick(st: &AppState) -> Duration {
    let hz = if st.visible.load(Ordering::Relaxed) {
        st.settings.lock().meter_rate_hz.clamp(10, 60)
    } else {
        2
    };
    Duration::from_millis(1000 / hz as u64)
}

pub fn spawn_workers(app: AppHandle, st: Arc<AppState>) {
    let rx = st.sess_rx.lock().take();
    if let Some(rx) = rx {
        let (a, s) = (app.clone(), st.clone());
        let _ = std::thread::Builder::new()
            .name("auralis-sessions".into())
            .spawn(move || session_loop(a, s, rx));
    }
    let _ = std::thread::Builder::new()
        .name("auralis-telemetry".into())
        .spawn(move || telemetry_loop(app, st));
}

fn session_loop(app: AppHandle, st: Arc<AppState>, rx: Receiver<SessCmd>) {
    devices::ensure_com();
    let mut poller: Option<SessionPoller> = None;
    let mut last_check = Instant::now() - Duration::from_secs(10);
    let mut seen_changes = u64::MAX;
    while !st.shutdown.load(Ordering::Relaxed) {
        let cmd = match rx.recv_timeout(tick(&st)) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        // (Re)create the poller when the default render device changed or the last attempt failed.
        let changes = st.device_changes.load(Ordering::Acquire);
        let retry = poller.is_none() && last_check.elapsed() > Duration::from_secs(2);
        if retry || changes != seen_changes {
            last_check = Instant::now();
            seen_changes = changes;
            poller = match SessionPoller::new(None) {
                Ok(p) => Some(p),
                Err(e) => {
                    tracing::warn!("session poller unavailable: {e}");
                    None
                }
            };
        }
        let Some(p) = poller.as_mut() else {
            if let Some(c) = cmd {
                reply_err(c);
            }
            let _ = app.emit("sessions", Vec::<SessionInfo>::new());
            continue;
        };
        match cmd {
            Some(SessCmd::Volume(id, v, tx)) => {
                let _ = tx.send(p.set_volume(&id, v));
            }
            Some(SessCmd::Mute(id, m, tx)) => {
                let _ = tx.send(p.set_mute(&id, m));
            }
            Some(SessCmd::ApplyVolumes(map)) => {
                if let Ok(list) = p.poll() {
                    for s in list.iter().filter(|s| !s.is_system) {
                        if let Some(v) = map.get(&s.exe_name.to_lowercase()) {
                            let _ = p.set_volume(&s.id, *v);
                        }
                    }
                }
            }
            Some(SessCmd::Refresh) | None => {}
        }
        match p.poll() {
            Ok(list) => {
                *st.sessions.lock() = list.clone();
                let _ = app.emit("sessions", list);
            }
            Err(e) => {
                tracing::warn!("session poll failed: {e}");
                poller = None;
            }
        }
    }
}

fn reply_err(c: SessCmd) {
    let e = || AppError::Audio("audio sessions are unavailable right now".into());
    match c {
        SessCmd::Volume(_, _, tx) | SessCmd::Mute(_, _, tx) => {
            let _ = tx.send(Err(e()));
        }
        _ => {}
    }
}

fn telemetry_loop(app: AppHandle, st: Arc<AppState>) {
    devices::ensure_com();
    let watcher = devices::DeviceWatcher::start(st.device_changes.clone());
    if let Err(e) = &watcher {
        tracing::warn!("device change notifications unavailable: {e}");
    }
    let (mut master, mut mic) = (Tracked::new(Flow::Render), Tracked::new(Flow::Capture));
    let mut seen_changes = st.device_changes.load(Ordering::Acquire);
    let mut n: u32 = 0;
    while !st.shutdown.load(Ordering::Relaxed) {
        let changes = st.device_changes.load(Ordering::Acquire);
        if changes != seen_changes {
            seen_changes = changes;
            let _ = app.emit("devices-changed", changes);
            master.id = None;
            mic.id = None;
        }
        if n % 10 == 0 {
            master.refresh_default();
            mic.refresh_default();
        }
        if n % 4 == 0 {
            master.refresh_volume();
            mic.refresh_volume();
        }
        n = n.wrapping_add(1);
        // A recording that filled its buffer becomes the current take automatically.
        let done = {
            let mut slot = st.recorder.lock();
            if slot
                .as_ref()
                .map(|r| !r.status().recording)
                .unwrap_or(false)
            {
                slot.take()
            } else {
                None
            }
        };
        if let Some(r) = done {
            match r.finish() {
                Ok(data) => {
                    *st.take.lock() = Some(Take {
                        original: Arc::new(data),
                        processed: None,
                    });
                    let _ = app.emit("take-ready", ());
                }
                Err(e) => {
                    tracing::warn!("recording failed: {e}");
                    let _ = app.emit("take-error", e.to_string());
                }
            }
        }
        let t = Telemetry {
            master: master.level(),
            mic: mic.level(),
            monitor: st.engine.lock().as_ref().map(|e| e.snapshot()),
            recorder: st.recorder.lock().as_ref().map(|r| r.status()),
            player: st.player.lock().as_ref().map(|p| p.status()),
            has_take: st.take.lock().is_some(),
            uptime_s: st.started.elapsed().as_secs(),
        };
        let _ = app.emit("telemetry", t);
        std::thread::sleep(tick(&st));
    }
    drop(watcher);
}

/// Wait briefly for a session command's result.
pub fn session_request(
    st: &AppState,
    make: impl FnOnce(Sender<AppResult<()>>) -> SessCmd,
) -> AppResult<()> {
    let (tx, rx) = bounded(1);
    st.sess_tx
        .send(make(tx))
        .map_err(|_| AppError::Internal("session worker stopped".into()))?;
    rx.recv_timeout(Duration::from_secs(3))
        .map_err(|_| AppError::Audio("the audio session did not respond".into()))?
}

pub fn main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
}

pub fn toggle_window(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        if w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_window(app);
        }
    }
}

pub fn show_window(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

//! Honest status reporting: virtual microphone detection and the diagnostics report.

use crate::app_state::AppState;
use crate::audio::devices::{self, DeviceInfo, Flow};
use crate::audio::policy;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMicStatus {
    /// `notDetected` or `detected`. The app never reports "connected": it cannot know whether another
    /// program is actually reading from the cable.
    pub state: &'static str,
    pub render_endpoints: Vec<DeviceInfo>,
    pub capture_endpoints: Vec<DeviceInfo>,
    pub message: String,
}

pub fn virtual_mic_status() -> VirtualMicStatus {
    let pick = |flow| {
        devices::list_devices(flow)
            .map(|l| {
                l.into_iter()
                    .filter(|d| d.is_virtual && d.state == "active")
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let render_endpoints = pick(Flow::Render);
    let capture_endpoints = pick(Flow::Capture);
    let (state, message) = if !render_endpoints.is_empty() && !capture_endpoints.is_empty() {
        ("detected", "A virtual audio cable is installed. Choose its playback side as the Voice Test output, then select its recording side as the microphone in your chat or recording app. Auralis cannot see whether another app is reading from it.".to_string())
    } else {
        ("notDetected", "No virtual audio cable was found. Auralis does not ship a driver. Install a virtual cable of your choice (for example VB-CABLE or a similar signed driver) to send your processed voice to other apps. Everything else works without it.".to_string())
    };
    VirtualMicStatus {
        state,
        render_endpoints,
        capture_endpoints,
        message,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub version: &'static str,
    pub os: String,
    pub uptime_s: u64,
    pub data_dir: String,
    pub log_dir: String,
    pub render_devices: usize,
    pub capture_devices: usize,
    pub default_render: Option<String>,
    pub default_capture: Option<String>,
    pub session_count: usize,
    pub routing_supported: bool,
    pub monitor_running: bool,
    pub monitor_error: Option<String>,
    pub virtual_mic: &'static str,
    pub device_change_events: u64,
    pub notes: Vec<String>,
}

pub fn collect(st: &AppState) -> Diagnostics {
    let render = devices::list_devices(Flow::Render).unwrap_or_default();
    let capture = devices::list_devices(Flow::Capture).unwrap_or_default();
    let snap = st.engine.lock().as_ref().map(|e| e.snapshot());
    let mut notes = vec![
        "Audio is processed locally. Nothing is uploaded and raw audio is never logged."
            .to_string(),
        "Latency shown in the Voice Test is the measured buffer path, not the acoustic round trip."
            .to_string(),
    ];
    if !policy::routing_supported() {
        notes.push(
            "Per-application output routing is not supported by this Windows build.".to_string(),
        );
    }
    Diagnostics {
        version: env!("CARGO_PKG_VERSION"),
        os: os_version(),
        uptime_s: st.started.elapsed().as_secs(),
        data_dir: crate::persistence::data_dir().display().to_string(),
        log_dir: crate::persistence::log_dir().display().to_string(),
        render_devices: render.len(),
        capture_devices: capture.len(),
        default_render: render.iter().find(|d| d.is_default).map(|d| d.name.clone()),
        default_capture: capture
            .iter()
            .find(|d| d.is_default)
            .map(|d| d.name.clone()),
        session_count: st.sessions.lock().len(),
        routing_supported: policy::routing_supported(),
        monitor_running: snap.as_ref().map(|s| s.running).unwrap_or(false),
        monitor_error: snap.and_then(|s| s.error),
        virtual_mic: virtual_mic_status().state,
        device_change_events: st.device_changes.load(std::sync::atomic::Ordering::Relaxed),
        notes,
    }
}

fn os_version() -> String {
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
    // `GetVersionExW` reports 6.2 to binaries without a compatibility manifest; `RtlGetVersion` is exact.
    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut OSVERSIONINFOW) -> i32;
    }
    let mut v = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // SAFETY: `v` is a valid, correctly sized OSVERSIONINFOW.
    if unsafe { RtlGetVersion(&mut v) } == 0 {
        format!(
            "Windows {}.{} build {}",
            v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber
        )
    } else {
        "Windows (version unknown)".into()
    }
}

/// Plain-text report for copy/export.
pub fn report(d: &Diagnostics, logs: &[String]) -> String {
    let mut s = String::new();
    s.push_str(&format!("Auralis {} diagnostics\n{}\n\n", d.version, d.os));
    s.push_str(&format!(
        "Uptime: {} s\nData dir: {}\nLog dir: {}\n",
        d.uptime_s, d.data_dir, d.log_dir
    ));
    s.push_str(&format!(
        "Render devices: {} (default: {})\n",
        d.render_devices,
        d.default_render.as_deref().unwrap_or("none")
    ));
    s.push_str(&format!(
        "Capture devices: {} (default: {})\n",
        d.capture_devices,
        d.default_capture.as_deref().unwrap_or("none")
    ));
    s.push_str(&format!(
        "Audio sessions: {}\nPer-app routing supported: {}\n",
        d.session_count, d.routing_supported
    ));
    s.push_str(&format!(
        "Monitoring running: {} (error: {})\nVirtual mic: {}\n\n",
        d.monitor_running,
        d.monitor_error.as_deref().unwrap_or("none"),
        d.virtual_mic
    ));
    for n in &d.notes {
        s.push_str(&format!("- {n}\n"));
    }
    s.push_str("\n--- recent log lines ---\n");
    for l in logs {
        s.push_str(l);
        s.push('\n');
    }
    s
}

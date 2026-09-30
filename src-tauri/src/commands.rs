//! IPC surface. Every command validates its input, runs off the UI thread and returns `AppResult`.
//! The frontend never gets shell or filesystem access: file dialogs are opened here in Rust.

use crate::app_state::{now_ms, session_request, AppState, SessCmd, Take, VoiceState};
use crate::audio::devices::{self, DeviceInfo, Flow};
use crate::audio::engine::{MonitorControls, MonitorEngine};
use crate::audio::recorder::{self, Player, Recorder};
use crate::audio::{icons, policy};
use crate::diagnostics::{self, Diagnostics, VirtualMicStatus};
use crate::dsp::presets::{builtin_presets, preset_by_id};
use crate::dsp::VoiceParams;
use crate::error::{AppError, AppResult};
use crate::hotkeys;
use crate::profiles::{self, Profile};
use crate::settings::Settings;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

type St<'a> = State<'a, Arc<AppState>>;

pub static START_HIDDEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn flow_of(s: &str) -> AppResult<Flow> {
    match s {
        "render" => Ok(Flow::Render),
        "capture" => Ok(Flow::Capture),
        _ => Err(AppError::Invalid(
            "flow must be 'render' or 'capture'".into(),
        )),
    }
}

fn check_id(id: &str) -> AppResult<()> {
    if id.is_empty() || id.len() > 512 || id.chars().any(|c| c.is_control()) {
        return Err(AppError::Invalid("invalid identifier".into()));
    }
    Ok(())
}

fn opt_id(id: Option<String>) -> AppResult<Option<String>> {
    match id.filter(|s| !s.is_empty()) {
        Some(s) => {
            check_id(&s)?;
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

fn scalar(v: f32) -> AppResult<f32> {
    if v.is_finite() {
        Ok(v.clamp(0.0, 1.0))
    } else {
        Err(AppError::Invalid("volume must be a number".into()))
    }
}

// ---- bootstrap & settings --------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDto {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    params: VoiceParams,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    settings: Settings,
    profiles: Vec<Profile>,
    presets: Vec<PresetDto>,
    default_params: VoiceParams,
    voice: VoiceState,
    hotkey_actions: Vec<(&'static str, &'static str)>,
    version: &'static str,
}

#[tauri::command]
pub async fn get_bootstrap(st: St<'_>) -> AppResult<Bootstrap> {
    Ok(Bootstrap {
        settings: st.settings.lock().clone(),
        profiles: st.profiles.lock().profiles.clone(),
        presets: builtin_presets()
            .into_iter()
            .map(|p| PresetDto {
                id: p.id,
                name: p.name,
                description: p.description,
                params: p.params,
            })
            .collect(),
        default_params: VoiceParams::default(),
        voice: st.voice_state(),
        hotkey_actions: hotkeys::ACTIONS.to_vec(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Called by the frontend once its first frame is ready; avoids showing a blank window.
#[tauri::command]
pub async fn app_ready(app: AppHandle) -> AppResult<()> {
    if !START_HIDDEN.load(Ordering::Relaxed) {
        crate::app_state::show_window(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    st: St<'_>,
    settings: Settings,
) -> AppResult<Settings> {
    let mut new = settings;
    new.validate()?;
    let old = st.settings.lock().clone();
    if new.start_with_windows != old.start_with_windows {
        let al = app.autolaunch();
        let r = if new.start_with_windows {
            al.enable()
        } else {
            al.disable()
        };
        if let Err(e) = r {
            return Err(AppError::Internal(format!(
                "could not change start-with-Windows: {e}"
            )));
        }
    }
    let mut clean = BTreeMap::new();
    for (action, acc) in &new.hotkeys {
        if hotkeys::is_known_action(action) {
            if let Ok(n) = hotkeys::normalize(acc) {
                clean.insert(action.clone(), n);
            }
        }
    }
    new.hotkeys = clean;
    *st.settings.lock() = new.clone();
    st.save_settings();
    Ok(new)
}

// ---- hotkeys ---------------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyResult {
    action: String,
    accelerator: String,
    registered: bool,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyReport {
    results: Vec<HotkeyResult>,
    conflicts: Vec<(String, Vec<String>)>,
}

impl HotkeyReport {
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.registered).count()
    }
}

pub fn register_hotkeys(
    app: &AppHandle,
    st: &Arc<AppState>,
    map: &BTreeMap<String, String>,
) -> HotkeyReport {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let conflicts = hotkeys::conflicts(map);
    let in_conflict: Vec<&String> = conflicts.iter().flat_map(|(_, a)| a.iter()).collect();
    let mut results = Vec::new();
    for (action, acc) in map {
        if !hotkeys::is_known_action(action) {
            continue;
        }
        if in_conflict.contains(&action) {
            results.push(HotkeyResult {
                action: action.clone(),
                accelerator: acc.clone(),
                registered: false,
                error: Some("Conflicts with another action".into()),
            });
            continue;
        }
        let (a, s, act) = (app.clone(), st.clone(), action.clone());
        let r = gs.on_shortcut(acc.as_str(), move |_, _, ev| {
            if ev.state() == ShortcutState::Pressed {
                let (a, s, act) = (a.clone(), s.clone(), act.clone());
                std::thread::spawn(move || {
                    let msg = run_action_inner(&a, &s, &act);
                    let _ = a.emit("hotkey-action", serde_json::json!({ "action": act, "message": msg.unwrap_or_else(|e| e.to_string()) }));
                });
            }
        });
        results.push(match r {
            Ok(()) => HotkeyResult {
                action: action.clone(),
                accelerator: acc.clone(),
                registered: true,
                error: None,
            },
            Err(e) => HotkeyResult {
                action: action.clone(),
                accelerator: acc.clone(),
                registered: false,
                error: Some(format!(
                    "Could not register (already used by another app?): {e}"
                )),
            },
        });
    }
    HotkeyReport { results, conflicts }
}

#[tauri::command]
pub async fn set_hotkeys(
    app: AppHandle,
    st: St<'_>,
    hotkeys_map: BTreeMap<String, String>,
) -> AppResult<HotkeyReport> {
    let mut clean = BTreeMap::new();
    for (action, acc) in hotkeys_map {
        if !hotkeys::is_known_action(&action) {
            return Err(AppError::Invalid(format!("unknown action '{action}'")));
        }
        if acc.trim().is_empty() {
            continue;
        }
        let n = hotkeys::normalize(&acc).map_err(AppError::Invalid)?;
        clean.insert(action, n);
    }
    st.settings.lock().hotkeys = clean.clone();
    st.save_settings();
    Ok(register_hotkeys(&app, st.inner(), &clean))
}

pub fn run_action_inner(app: &AppHandle, st: &Arc<AppState>, action: &str) -> AppResult<String> {
    let msg = match action {
        "toggleMicMute" => {
            let m = AppState::toggle_mute(Flow::Capture)?;
            if m {
                "Microphone muted"
            } else {
                "Microphone unmuted"
            }
            .to_string()
        }
        "masterMute" => {
            let m = AppState::toggle_mute(Flow::Render)?;
            if m { "Output muted" } else { "Output unmuted" }.to_string()
        }
        "toggleMonitor" => {
            if st
                .engine
                .lock()
                .as_ref()
                .map(|e| e.is_running())
                .unwrap_or(false)
            {
                st.stop_monitor();
                "Monitoring stopped".to_string()
            } else {
                let (i, o) = {
                    let s = st.settings.lock();
                    (
                        s.monitor_input_device.clone(),
                        s.monitor_output_device.clone(),
                    )
                };
                st.start_monitor(i, o)?;
                "Monitoring started".to_string()
            }
        }
        "toggleVoiceFx" => {
            let mut c = *st.controls.lock();
            c.processed = !c.processed;
            st.set_controls(c);
            if c.processed {
                "Voice effect on"
            } else {
                "Voice effect bypassed (raw)"
            }
            .to_string()
        }
        "nextPreset" => format!("Preset: {}", st.cycle_preset(1)?),
        "prevPreset" => format!("Preset: {}", st.cycle_preset(-1)?),
        "emergencyStop" => {
            st.emergency_stop();
            "Emergency stop: audio stopped".to_string()
        }
        "showWindow" => {
            crate::app_state::toggle_window(app);
            "Window toggled".to_string()
        }
        _ => return Err(AppError::Invalid(format!("unknown action '{action}'"))),
    };
    st.emit_voice(app);
    Ok(msg)
}

#[tauri::command]
pub async fn run_action(app: AppHandle, st: St<'_>, action: String) -> AppResult<String> {
    run_action_inner(&app, st.inner(), &action)
}

// ---- devices ---------------------------------------------------------------------------------

#[tauri::command]
pub async fn list_devices(flow: String) -> AppResult<Vec<DeviceInfo>> {
    devices::list_devices(flow_of(&flow)?)
}

#[tauri::command]
pub async fn set_default_device(device_id: String) -> AppResult<()> {
    check_id(&device_id)?;
    policy::set_default_device(&device_id)
}

#[tauri::command]
pub async fn set_device_volume(device_id: String, volume: f32) -> AppResult<()> {
    check_id(&device_id)?;
    devices::set_endpoint_volume(&device_id, scalar(volume)?)
}

#[tauri::command]
pub async fn set_device_mute(device_id: String, muted: bool) -> AppResult<()> {
    check_id(&device_id)?;
    devices::set_endpoint_mute(&device_id, muted)
}

#[tauri::command]
pub async fn virtual_mic_status() -> AppResult<VirtualMicStatus> {
    Ok(diagnostics::virtual_mic_status())
}

// ---- sessions & routing ----------------------------------------------------------------------

#[tauri::command]
pub async fn get_sessions(st: St<'_>) -> AppResult<Vec<crate::audio::sessions::SessionInfo>> {
    Ok(st.sessions.lock().clone())
}

#[tauri::command]
pub async fn refresh_sessions(st: St<'_>) -> AppResult<()> {
    let _ = st.sess_tx.send(SessCmd::Refresh);
    Ok(())
}

#[tauri::command]
pub async fn set_session_volume(st: St<'_>, session_id: String, volume: f32) -> AppResult<()> {
    check_id(&session_id)?;
    let v = scalar(volume)?;
    session_request(&st, |tx| SessCmd::Volume(session_id, v, tx))
}

#[tauri::command]
pub async fn set_session_mute(st: St<'_>, session_id: String, muted: bool) -> AppResult<()> {
    check_id(&session_id)?;
    session_request(&st, |tx| SessCmd::Mute(session_id, muted, tx))
}

#[tauri::command]
pub async fn get_app_icon(st: St<'_>, exe_path: String) -> AppResult<Option<String>> {
    // Only serve icons for executables that currently own an audio session.
    if !st.sessions.lock().iter().any(|s| s.exe_path == exe_path) {
        return Err(AppError::Invalid("unknown application".into()));
    }
    Ok(icons::icon_data_url(&exe_path))
}

#[tauri::command]
pub async fn routing_supported() -> AppResult<bool> {
    Ok(policy::routing_supported())
}

#[tauri::command]
pub async fn get_app_route(pid: u32, flow: String) -> AppResult<Option<String>> {
    policy::get_app_device(pid, flow_of(&flow)?)
}

#[tauri::command]
pub async fn set_app_route(
    st: St<'_>,
    pid: u32,
    flow: String,
    device_id: Option<String>,
) -> AppResult<()> {
    if pid == 0 || !st.sessions.lock().iter().any(|s| s.pid == pid) {
        return Err(AppError::Invalid(
            "that application has no active audio session".into(),
        ));
    }
    let id = opt_id(device_id)?;
    policy::set_app_device(pid, flow_of(&flow)?, id.as_deref())
}

// ---- voice / monitor -------------------------------------------------------------------------

#[tauri::command]
pub async fn get_voice(st: St<'_>) -> AppResult<VoiceState> {
    Ok(st.voice_state())
}

#[tauri::command]
pub async fn set_voice_params(
    st: St<'_>,
    params: VoiceParams,
    preset_id: Option<String>,
) -> AppResult<VoiceState> {
    if let Some(id) = &preset_id {
        if preset_by_id(id).is_none() {
            return Err(AppError::Invalid("unknown preset".into()));
        }
    }
    st.set_voice_params(params, preset_id);
    Ok(st.voice_state())
}

#[tauri::command]
pub async fn apply_preset(st: St<'_>, preset_id: String) -> AppResult<VoiceState> {
    st.apply_preset(&preset_id)?;
    Ok(st.voice_state())
}

#[tauri::command]
pub async fn set_monitor_controls(st: St<'_>, controls: MonitorControls) -> AppResult<VoiceState> {
    st.set_controls(controls);
    Ok(st.voice_state())
}

#[tauri::command]
pub async fn start_monitor(
    st: St<'_>,
    input_id: Option<String>,
    output_id: Option<String>,
) -> AppResult<()> {
    let (i, o) = (opt_id(input_id)?, opt_id(output_id)?);
    // Starting always requires an explicit call; the monitor is never started implicitly.
    st.start_monitor(i, o)
}

#[tauri::command]
pub async fn stop_monitor(st: St<'_>) -> AppResult<()> {
    st.stop_monitor();
    Ok(())
}

#[tauri::command]
pub async fn emergency_stop(app: AppHandle, st: St<'_>) -> AppResult<()> {
    st.emergency_stop();
    st.emit_voice(&app);
    Ok(())
}

#[tauri::command]
pub async fn reset_clips(st: St<'_>) -> AppResult<()> {
    if let Some(e) = st.engine.lock().as_ref() {
        e.reset_clips();
    }
    Ok(())
}

#[tauri::command]
pub async fn feedback_risk(
    input_id: Option<String>,
    output_id: Option<String>,
) -> AppResult<Option<String>> {
    let (i, o) = (opt_id(input_id)?, opt_id(output_id)?);
    Ok(MonitorEngine::feedback_risk(i.as_deref(), o.as_deref()))
}

// ---- recording & takes -----------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeInfo {
    duration_ms: u32,
    original: Vec<f32>,
    processed: Option<Vec<f32>>,
    processed_key: Option<String>,
}

fn peaks(s: &[f32], n: usize) -> Vec<f32> {
    if s.is_empty() {
        return vec![];
    }
    let step = (s.len() as f32 / n as f32).max(1.0);
    (0..n.min(s.len()))
        .map(|i| {
            let a = (i as f32 * step) as usize;
            let b = (((i + 1) as f32 * step) as usize).min(s.len()).max(a + 1);
            s[a..b.min(s.len())]
                .iter()
                .fold(0.0f32, |m, v| m.max(v.abs()))
        })
        .collect()
}

pub fn take_info_of(st: &AppState) -> Option<TakeInfo> {
    let t = st.take.lock();
    let t = t.as_ref()?;
    Some(TakeInfo {
        duration_ms: (t.original.len() as u64 * 1000 / crate::audio::stream::SAMPLE_RATE as u64)
            as u32,
        original: peaks(&t.original, 400),
        processed: t.processed.as_ref().map(|(_, v)| peaks(v, 400)),
        processed_key: t.processed.as_ref().map(|(k, _)| k.clone()),
    })
}

#[tauri::command]
pub async fn start_recording(st: St<'_>, seconds: u32, input_id: Option<String>) -> AppResult<()> {
    let input = opt_id(input_id)?;
    let mut slot = st.recorder.lock();
    if slot.is_some() {
        return Err(AppError::Invalid(
            "a recording is already in progress".into(),
        ));
    }
    if let Some(mut p) = st.player.lock().take() {
        p.stop();
    }
    *slot = Some(Recorder::start(input, seconds)?);
    Ok(())
}

/// Finish the current recording early (or collect it once it filled up) and keep it as the take.
#[tauri::command]
pub async fn stop_recording(st: St<'_>) -> AppResult<Option<TakeInfo>> {
    let r = st.recorder.lock().take();
    if let Some(r) = r {
        let data = r.finish()?;
        *st.take.lock() = Some(Take {
            original: Arc::new(data),
            processed: None,
        });
    }
    Ok(take_info_of(&st))
}

#[tauri::command]
pub async fn cancel_recording(st: St<'_>) -> AppResult<()> {
    if let Some(r) = st.recorder.lock().take() {
        r.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn discard_take(st: St<'_>) -> AppResult<()> {
    if let Some(mut p) = st.player.lock().take() {
        p.stop();
    }
    *st.take.lock() = None;
    Ok(())
}

#[tauri::command]
pub async fn take_info(st: St<'_>) -> AppResult<Option<TakeInfo>> {
    Ok(take_info_of(&st))
}

/// Render the take through a preset (or the current settings when `preset_id` is null).
#[tauri::command]
pub async fn process_take(st: St<'_>, preset_id: Option<String>) -> AppResult<Option<TakeInfo>> {
    st.processed_take(preset_id.as_deref())?;
    Ok(take_info_of(&st))
}

#[tauri::command]
pub async fn play_take(
    st: St<'_>,
    which: String,
    output_id: Option<String>,
    gain_db: f32,
) -> AppResult<()> {
    let out = opt_id(output_id)?;
    let samples = match which.as_str() {
        "original" => st
            .take
            .lock()
            .as_ref()
            .map(|t| t.original.clone())
            .ok_or_else(|| AppError::NotFound("no recording yet".into()))?,
        "processed" => {
            let cached = st
                .take
                .lock()
                .as_ref()
                .and_then(|t| t.processed.as_ref().map(|(_, v)| v.clone()));
            match cached {
                Some(v) => v,
                None => st.processed_take(None)?.1,
            }
        }
        _ => {
            return Err(AppError::Invalid(
                "which must be 'original' or 'processed'".into(),
            ))
        }
    };
    if let Some(mut p) = st.player.lock().take() {
        p.stop();
    }
    let player = Player::start(out, samples, gain_db)?;
    *st.player.lock() = Some(player);
    Ok(())
}

#[tauri::command]
pub async fn stop_playback(st: St<'_>) -> AppResult<()> {
    if let Some(mut p) = st.player.lock().take() {
        p.stop();
    }
    Ok(())
}

fn save_dialog(app: &AppHandle, name: &str, filter: (&str, &[&str])) -> Option<PathBuf> {
    app.dialog()
        .file()
        .set_file_name(name)
        .add_filter(filter.0, filter.1)
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
}

fn open_dialog(app: &AppHandle, filter: (&str, &[&str])) -> Option<PathBuf> {
    app.dialog()
        .file()
        .add_filter(filter.0, filter.1)
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
}

/// Saves a take as WAV, but only after the user picks a location. Returns the path, or null if cancelled.
#[tauri::command]
pub async fn export_take_wav(
    app: AppHandle,
    st: St<'_>,
    which: String,
) -> AppResult<Option<String>> {
    let samples = match which.as_str() {
        "original" => st.take.lock().as_ref().map(|t| t.original.clone()),
        "processed" => st
            .take
            .lock()
            .as_ref()
            .and_then(|t| t.processed.as_ref().map(|(_, v)| v.clone())),
        _ => {
            return Err(AppError::Invalid(
                "which must be 'original' or 'processed'".into(),
            ))
        }
    }
    .ok_or_else(|| AppError::NotFound("nothing to export".into()))?;
    let Some(path) = save_dialog(
        &app,
        &format!("auralis-{which}.wav"),
        ("WAV audio", &["wav"]),
    ) else {
        return Ok(None);
    };
    std::fs::write(&path, recorder::encode_wav(&samples))?;
    Ok(Some(path.display().to_string()))
}

// ---- profiles --------------------------------------------------------------------------------

fn new_id() -> String {
    format!("p{:x}{:04x}", now_ms(), rand_u16())
}

fn rand_u16() -> u16 {
    use std::sync::atomic::AtomicU32;
    static N: AtomicU32 = AtomicU32::new(0x9e37);
    let n = N.fetch_add(0x6d2b, Ordering::Relaxed);
    (n ^ (now_ms() as u32 >> 3)) as u16
}

#[tauri::command]
pub async fn list_profiles(st: St<'_>) -> AppResult<Vec<Profile>> {
    Ok(st.profiles.lock().profiles.clone())
}

#[tauri::command]
pub async fn save_profile(st: St<'_>, profile: Profile) -> AppResult<Profile> {
    let p = st.profiles.lock().upsert(profile, new_id, now_ms())?;
    st.save_profiles();
    Ok(p)
}

#[tauri::command]
pub async fn create_profile_from_current(st: St<'_>, name: String) -> AppResult<Profile> {
    let p = st.profile_from_current(&name);
    let p = st.profiles.lock().upsert(p, new_id, now_ms())?;
    st.save_profiles();
    Ok(p)
}

#[tauri::command]
pub async fn delete_profile(st: St<'_>, id: String) -> AppResult<()> {
    st.profiles.lock().delete(&id)?;
    {
        let mut s = st.settings.lock();
        if s.active_profile.as_deref() == Some(id.as_str()) {
            s.active_profile = None;
        }
    }
    st.save_profiles();
    st.save_settings();
    Ok(())
}

#[tauri::command]
pub async fn duplicate_profile(st: St<'_>, id: String) -> AppResult<Profile> {
    let p = st.profiles.lock().duplicate(&id, new_id, now_ms())?;
    st.save_profiles();
    Ok(p)
}

#[tauri::command]
pub async fn activate_profile(app: AppHandle, st: St<'_>, id: String) -> AppResult<Profile> {
    let p = st
        .profiles
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("profile not found".into()))?;
    st.apply_profile(&p);
    st.settings.lock().active_profile = Some(id);
    st.save_settings();
    st.emit_voice(&app);
    Ok(p)
}

#[tauri::command]
pub async fn export_profile(app: AppHandle, st: St<'_>, id: String) -> AppResult<Option<String>> {
    let p = st
        .profiles
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("profile not found".into()))?;
    let text = profiles::export_profile(&p)?;
    let Some(path) = save_dialog(
        &app,
        &format!(
            "{}.auralis-profile.json",
            p.name
                .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        ),
        ("Auralis profile", &["json"]),
    ) else {
        return Ok(None);
    };
    std::fs::write(&path, text)?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub async fn import_profile(app: AppHandle, st: St<'_>) -> AppResult<Option<Profile>> {
    let Some(path) = open_dialog(&app, ("Auralis profile", &["json"])) else {
        return Ok(None);
    };
    let len = std::fs::metadata(&path)?.len();
    if len > 1_000_000 {
        return Err(AppError::Invalid(
            "file is too large to be a profile".into(),
        ));
    }
    let mut p = profiles::import_profile(&std::fs::read_to_string(&path)?)?;
    p.id.clear();
    let p = st.profiles.lock().upsert(p, new_id, now_ms())?;
    st.save_profiles();
    Ok(Some(p))
}

// ---- diagnostics -----------------------------------------------------------------------------

#[tauri::command]
pub async fn get_diagnostics(st: St<'_>) -> AppResult<Diagnostics> {
    Ok(diagnostics::collect(&st))
}

#[tauri::command]
pub async fn get_logs(lines: Option<usize>) -> AppResult<Vec<String>> {
    Ok(crate::logging::recent(lines.unwrap_or(200).min(1000)))
}

#[tauri::command]
pub async fn diagnostics_report(st: St<'_>) -> AppResult<String> {
    Ok(diagnostics::report(
        &diagnostics::collect(&st),
        &crate::logging::recent(200),
    ))
}

#[tauri::command]
pub async fn export_diagnostics(app: AppHandle, st: St<'_>) -> AppResult<Option<String>> {
    let text = diagnostics::report(&diagnostics::collect(&st), &crate::logging::recent(500));
    let Some(path) = save_dialog(&app, "auralis-diagnostics.txt", ("Text", &["txt"])) else {
        return Ok(None);
    };
    std::fs::write(&path, text)?;
    Ok(Some(path.display().to_string()))
}

/// Stops and forgets every stream so the next start opens fresh devices.
#[tauri::command]
pub async fn restart_engine(app: AppHandle, st: St<'_>) -> AppResult<()> {
    let was_running = st.engine.lock().is_some();
    let (i, o) = {
        let s = st.settings.lock();
        (
            s.monitor_input_device.clone(),
            s.monitor_output_device.clone(),
        )
    };
    st.stop_monitor();
    st.device_changes.fetch_add(1, Ordering::AcqRel); // makes the session worker rebuild its poller
    tracing::info!("engine restart requested");
    if was_running {
        st.start_monitor(i, o)?;
    }
    st.emit_voice(&app);
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> AppResult<()> {
    app.exit(0);
    Ok(())
}

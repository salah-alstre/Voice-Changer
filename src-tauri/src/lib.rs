#![allow(clippy::field_reassign_with_default, clippy::items_after_test_module)]
pub mod app_state;
pub mod audio;
pub mod commands;
pub mod diagnostics;
pub mod dsp;
pub mod error;
pub mod hotkeys;
pub mod logging;
pub mod persistence;
pub mod profiles;
pub mod settings;
pub mod tray;

use app_state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{Manager, RunEvent, WindowEvent};

pub fn run() {
    let _log_guard = logging::init();
    let minimized_arg = std::env::args().any(|a| a == "--minimized");
    let state = Arc::new(AppState::load());
    if minimized_arg && state.settings.lock().start_minimized {
        commands::START_HIDDEN.store(true, Ordering::Relaxed);
    }
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Auralis starting");

    let st_setup = state.clone();
    let st_events = state.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app_state::show_window(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(state)
        .setup(move |app| {
            let handle = app.handle().clone();
            if let Err(e) = tray::build(&handle, st_setup.clone()) {
                tracing::warn!("tray unavailable: {e}");
            }
            let map = st_setup.settings.lock().hotkeys.clone();
            let report = commands::register_hotkeys(&handle, &st_setup, &map);
            let failed = report.failed_count();
            if failed > 0 {
                tracing::warn!(failed, "some global hotkeys could not be registered");
            }
            app_state::spawn_workers(handle, st_setup.clone());
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = {
                    let s = st_events.settings.lock();
                    s.close_to_tray || s.minimize_to_tray
                };
                if close_to_tray && window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    st_events.visible.store(false, Ordering::Relaxed);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::app_ready,
            commands::update_settings,
            commands::set_hotkeys,
            commands::run_action,
            commands::list_devices,
            commands::set_default_device,
            commands::set_device_volume,
            commands::set_device_mute,
            commands::virtual_mic_status,
            commands::get_sessions,
            commands::refresh_sessions,
            commands::set_session_volume,
            commands::set_session_mute,
            commands::get_app_icon,
            commands::routing_supported,
            commands::get_app_route,
            commands::set_app_route,
            commands::get_voice,
            commands::set_voice_params,
            commands::apply_preset,
            commands::set_monitor_controls,
            commands::start_monitor,
            commands::stop_monitor,
            commands::emergency_stop,
            commands::reset_clips,
            commands::feedback_risk,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::discard_take,
            commands::take_info,
            commands::process_take,
            commands::play_take,
            commands::stop_playback,
            commands::export_take_wav,
            commands::list_profiles,
            commands::save_profile,
            commands::create_profile_from_current,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::activate_profile,
            commands::export_profile,
            commands::import_profile,
            commands::get_diagnostics,
            commands::get_logs,
            commands::diagnostics_report,
            commands::export_diagnostics,
            commands::restart_engine,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Auralis");

    let st_exit = app.state::<Arc<AppState>>().inner().clone();
    app.run(move |_app, event| {
        if let RunEvent::Exit = event {
            st_exit.shutdown.store(true, Ordering::Relaxed);
            st_exit.stop_monitor();
        }
    });
}

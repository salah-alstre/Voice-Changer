//! System tray: quick actions that mirror the global hotkey actions.

use crate::app_state::{show_window, AppState};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter};

pub fn build(app: &AppHandle, st: Arc<AppState>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Auralis", true, None::<&str>)?;
    let mute = MenuItem::with_id(
        app,
        "toggleMicMute",
        "Mute / unmute microphone",
        true,
        None::<&str>,
    )?;
    let stop = MenuItem::with_id(
        app,
        "emergencyStop",
        "Emergency stop audio",
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &mute, &stop, &sep, &quit])?;

    let mut b = TrayIconBuilder::with_id("main")
        .tooltip("Auralis")
        .menu(&menu)
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    let st_menu = st.clone();
    b.on_menu_event(move |app, ev| match ev.id().as_ref() {
        "show" => show_window(app),
        "quit" => app.exit(0),
        id @ ("toggleMicMute" | "emergencyStop") => {
            let (a, s, id) = (app.clone(), st_menu.clone(), id.to_string());
            std::thread::spawn(move || {
                let msg = crate::commands::run_action_inner(&a, &s, &id)
                    .unwrap_or_else(|e| e.to_string());
                let _ = a.emit(
                    "hotkey-action",
                    serde_json::json!({ "action": id, "message": msg }),
                );
            });
        }
        _ => {}
    })
    .on_tray_icon_event(|tray, ev| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = ev
        {
            show_window(tray.app_handle());
        }
    })
    .build(app)?;
    Ok(())
}

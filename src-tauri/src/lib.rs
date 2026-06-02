mod capture;
mod commands;
mod state;

use state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            // ── Tray icon ──────────────────────────────────────────────────
            let show = MenuItem::with_id(app, "show", "Show pkap", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop Recording", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &stop, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone()) // TODO: handle error
                .menu(&menu)
                .tooltip("pkap — screen recorder")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            win.show().unwrap(); // TODO: handle error
                            win.set_focus().unwrap(); // TODO: handle error
                        }
                    }
                    "stop" => {
                        let state = app.state::<AppState>();
                        let _ = commands::do_stop(app, &state);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // ── Global hotkey: Cmd+Shift+R toggles recording ───────────────
            // Shortcut::new(modifiers, key_code)
            // Modifiers::SUPER = Cmd on macOS, Win key on Windows, Super on Linux.
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyR);

            app.handle().global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
                // Only act on key-down, not key-up.
                if event.state == ShortcutState::Pressed {
                    let state = app.state::<AppState>();
                    if state.is_recording() {
                        let _ = commands::do_stop(app, &state);
                    } else {
                        let _ = commands::do_start(app, &state);
                    }
                }
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_region_select,
            commands::set_region,
            commands::cancel_region_select,
            commands::select_full_screen,
            commands::get_region,
            commands::get_monitors,
            commands::select_monitor,
            commands::get_settings,
            commands::set_fps,
            commands::set_quality,
            commands::open_preview_window,
            commands::get_preview_info,
            commands::discard_recording,
            commands::close_preview,
            commands::set_output_format,
            commands::get_output_format,
            commands::set_save_folder,
            commands::get_save_folder,
            commands::start_recording,
            commands::stop_recording,
            commands::get_recording_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

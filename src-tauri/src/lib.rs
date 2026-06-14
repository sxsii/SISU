// lib.rs — SISU backend library entry point

pub mod db;
pub mod monitor;
pub mod process_ctrl;
pub mod optimizer;
pub mod profiles;
pub mod rules;
pub mod foreground;
pub mod notifications;

use std::sync::{Arc, Mutex};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};
use monitor::SystemState;
use optimizer::OptimizerState;
use rusqlite::Connection;

// ============================================================
// AppState
// ============================================================

pub struct AppState {
    pub system:    Arc<Mutex<SystemState>>,
    pub optimizer: Arc<Mutex<OptimizerState>>,
    pub db:        Arc<Mutex<Connection>>,
}

// ============================================================
// Entry Point
// ============================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_conn = Connection::open(db::db_path())
        .expect("Failed to open SISU database");
    db::init_db(&db_conn)
        .expect("Failed to initialize database schema");
    let db_state = Arc::new(Mutex::new(db_conn));

    let optimizer_state    = Arc::new(Mutex::new(OptimizerState::new()));
    let optimizer_for_loop = optimizer_state.clone();
    let db_for_loop        = db_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            monitor::get_system_snapshot,
            monitor::get_process_list,
            process_ctrl::set_process_priority,
            process_ctrl::suspend_process,
            process_ctrl::resume_process,
            process_ctrl::kill_process,
            process_ctrl::set_process_affinity,
            optimizer::get_optimizer_status,
            optimizer::set_optimizer_enabled,
            profiles::load_profiles,
            profiles::save_profile,
            profiles::delete_profile,
            profiles::activate_profile,
            profiles::deactivate_profiles,
            db::get_event_history,
            db::get_performance_history,
        ])
        // Intercept the window close event so clicking X
        // hides the window instead of quitting the app.
        // The app continues running in the tray.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent the default close behaviour
                api.prevent_close();
                // Hide the window — the tray icon remains visible
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();

            // ---- Build tray menu ----
            // Each MenuItem needs an id, label, enabled state, and
            // an optional keyboard accelerator (None here).
            let open_item  = MenuItem::with_id(app, "open",  "Open SISU",  true, None::<&str>)?;
            let toggle_item = MenuItem::with_id(app, "toggle", "Enable Optimizer", true, None::<&str>)?;
            let quit_item  = MenuItem::with_id(app, "quit",  "Quit",       true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &open_item,
                &toggle_item,
                &quit_item,
            ])?;

            // ---- Build tray icon ----
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("SISU — System Resource Manager")
                // Handle menu item clicks
                .on_menu_event({
                    let handle = handle.clone();
                    move |app, event| {
                        match event.id.as_ref() {
                            "open" => {
                                // Show and focus the main window
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                            "toggle" => {
                                // Toggle the optimizer on/off
                                if let Some(state) = app.try_state::<AppState>() {
                                    let mut opt = state.optimizer.lock().unwrap();
                                    opt.status.enabled = !opt.status.enabled;
                                    let enabled = opt.status.enabled;
                                    drop(opt);
                                    log::info!(
                                        "[tray] Optimizer {}",
                                        if enabled { "enabled" } else { "disabled" }
                                    );
                                }
                            }
                            "quit" => {
                                // Actually quit — exit the process entirely
                                app.exit(0);
                            }
                            _ => {}
                        }
                    }
                })
                // Left-click on the tray icon shows/hides the window
                .on_tray_icon_event({
                    let handle = handle.clone();
                    move |_tray, event| {
                        if let TrayIconEvent::Click {
                            button:       MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event {
                            if let Some(win) = handle.get_webview_window("main") {
                                if win.is_visible().unwrap_or(false) {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            // ---- Spawn background tasks ----
            tauri::async_runtime::spawn(async move {
                let system_state = Arc::new(Mutex::new(SystemState::new()));

                handle.manage(AppState {
                    system:    system_state.clone(),
                    optimizer: optimizer_for_loop,
                    db:        db_for_loop,
                });

                monitor::monitoring_loop(
                    system_state,
                    optimizer_state,
                    db_state,
                    handle,
                ).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SISU");
}
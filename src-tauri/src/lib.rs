// lib.rs — SISU backend library entry point

pub mod platform;
pub mod db;
pub mod monitor;
pub mod process_ctrl;
pub mod optimizer;
pub mod profiles;
pub mod rules;
pub mod foreground;
pub mod notifications;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use monitor::SystemState;

pub struct AppState {
    pub system: Arc<Mutex<SystemState>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        ])
        .setup(|app| {
            // IMPORTANT: SystemState::new() must be called here,
            // inside setup(), NOT before Builder::default().
            //
            // Why: sysinfo 0.33 calls CoInitializeEx(COINIT_MULTITHREADED)
            // on Windows when it first reads system data. Tauri's windowing
            // library (tao) calls CoInitializeEx(COINIT_APARTMENTTHREADED)
            // for drag-and-drop support. Windows does not allow two different
            // COM threading modes on the same thread.
            //
            // By the time setup() runs, Tauri has already initialized COM
            // for its own needs on the main thread. We therefore spawn
            // SystemState::new() on a separate thread via
            // tauri::async_runtime::spawn so it gets its own thread with
            // its own COM context — avoiding the conflict entirely.
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                // Initialize system state on the async thread pool.
                // This thread is separate from the main UI thread so
                // COM mode conflicts cannot occur.
                let system_state = Arc::new(Mutex::new(SystemState::new()));

                // Register AppState with Tauri's state management so
                // command handlers can access it via State<AppState>.
                // manage() can be called after setup completes.
                handle.manage(AppState {
                    system: system_state.clone(),
                });

                // Start the monitoring loop on the same async task.
                // It runs forever, emitting system-update events every
                // 2 seconds until the application exits.
                monitor::monitoring_loop(system_state, handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SISU");
}
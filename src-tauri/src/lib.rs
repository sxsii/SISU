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
use optimizer::OptimizerState;

// ============================================================
// AppState
// ============================================================

pub struct AppState {
    pub system:    Arc<Mutex<SystemState>>,
    pub optimizer: Arc<Mutex<OptimizerState>>,
}

// ============================================================
// Application Entry Point
// ============================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let optimizer_state = Arc::new(Mutex::new(OptimizerState::new()));
    let optimizer_for_loop = optimizer_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            // Monitoring
            monitor::get_system_snapshot,
            monitor::get_process_list,
            // Process control
            process_ctrl::set_process_priority,
            process_ctrl::suspend_process,
            process_ctrl::resume_process,
            process_ctrl::kill_process,
            process_ctrl::set_process_affinity,
            // Optimizer
            optimizer::get_optimizer_status,
            optimizer::set_optimizer_enabled,
            // Profiles
            profiles::load_profiles,
            profiles::save_profile,
            profiles::delete_profile,
            profiles::activate_profile,
            profiles::deactivate_profiles,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let system_state = Arc::new(Mutex::new(SystemState::new()));

                handle.manage(AppState {
                    system:    system_state.clone(),
                    optimizer: optimizer_for_loop,
                });

                monitor::monitoring_loop(system_state, optimizer_state, handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SISU");
}
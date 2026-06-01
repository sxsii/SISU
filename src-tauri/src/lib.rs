// lib.rs — SISU backend library entry point
//
// Every module in the backend is declared here.
// Rust requires explicit module declarations — unlike JavaScript,
// files are not automatically part of the project just by existing.
// Each `mod` statement here tells the Rust compiler to find and
// compile the corresponding .rs file.

// Platform abstraction layer — must be declared before modules
// that depend on it so Rust resolves the dependency order correctly
pub mod platform;

// Core backend modules
pub mod db;           // SQLite database: schema, logging, history
pub mod monitor;      // System monitoring: CPU, RAM, disk, network, processes
pub mod process_ctrl; // Process control: priority, suspend, resume, kill
pub mod optimizer;    // Optimization engine: profile activation, status
pub mod profiles;     // Profile storage: load, save, delete JSON profiles
pub mod rules;        // Rule engine: threshold evaluation, default rules
pub mod foreground;   // Foreground detection: active window polling loop
pub mod notifications;// Notification queue: alerts, warnings, tray messages

// The run() function is the single entry point called by main.rs.
// All Tauri setup — plugins, commands, state, event handlers —
// will be registered here as we build each phase.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        // Commands and state will be added here in Phase 4 onwards
        .run(tauri::generate_context!())
        .expect("error while running SISU");
}
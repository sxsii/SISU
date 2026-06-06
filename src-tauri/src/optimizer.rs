// optimizer.rs — SISU Optimization Engine
//
// The optimizer sits between the rule engine and the process
// control module. It:
// 1. Receives actions from the rule engine
// 2. Executes them safely via process_ctrl
// 3. Tracks optimization state
// 4. Exposes status to the frontend via Tauri commands

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use crate::rules::OptimizationAction;

// ============================================================
// Optimization State
//
// Tracks whether optimization is enabled and what the last
// action taken was. Shared between the background loop and
// the frontend command handlers.
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerStatus {
    pub enabled:        bool,
    pub active_profile: Option<String>,
    pub last_action:    Option<String>,
    pub actions_taken:  u64,
}

impl Default for OptimizerStatus {
    fn default() -> Self {
        OptimizerStatus {
            enabled:        false,
            active_profile: None,
            last_action:    None,
            actions_taken:  0,
        }
    }
}

pub struct OptimizerState {
    pub status: OptimizerStatus,
}

impl OptimizerState {
    pub fn new() -> Self {
        OptimizerState {
            status: OptimizerStatus::default(),
        }
    }
}

// ============================================================
// Action Execution
//
// Takes the list of actions from the rule engine and executes
// them. This is where decisions become reality.
// ============================================================

pub fn execute_actions(
    actions:    &[OptimizationAction],
    sys_state:  &crate::monitor::SystemState,
    opt_state:  &mut OptimizerState,
    handle:     &AppHandle,
) {
    for action in actions {
        match action {
            OptimizationAction::ThrottleBackground { reason } => {
                throttle_background_processes(sys_state, reason, handle);
                opt_state.status.last_action = Some(
                    format!("Throttled background processes: {}", reason)
                );
                opt_state.status.actions_taken += 1;
            }

            OptimizationAction::EmitAlert { message, level } => {
                let _ = handle.emit("alert", serde_json::json!({
                    "id":      format!("opt-{}", chrono_timestamp()),
                    "type":    "optimizer",
                    "message": message,
                    "level":   level,
                }));
            }

            OptimizationAction::ActivateProfile { name } => {
                opt_state.status.active_profile = Some(name.clone());
                opt_state.status.last_action = Some(
                    format!("Activated profile: {}", name)
                );
                let _ = handle.emit("alert", serde_json::json!({
                    "id":      format!("profile-{}", chrono_timestamp()),
                    "type":    "profile_activated",
                    "message": format!("Optimization profile '{}' activated.", name),
                    "level":   "info",
                }));
            }

            OptimizationAction::LogEvent { event_type, detail } => {
                // In Phase 8 we will write these to SQLite.
                // For now we log to the console.
                log::info!("[optimizer] {}: {}", event_type, detail);
            }
        }
    }
}

/// Lower the priority of non-essential background processes.
/// We only touch processes that are clearly background tasks —
/// low CPU users that are not the foreground application.
fn throttle_background_processes(
    sys_state: &crate::monitor::SystemState,
    reason:    &str,
    handle:    &AppHandle,
) {
    let mut throttled = 0u32;

    for process in sys_state.sys.processes().values() {
        let name = process.name().to_string_lossy().to_string();
        let pid  = process.pid().as_u32();
        let cpu  = process.cpu_usage();

        // Skip processes with meaningful CPU usage — they might be
        // doing something the user cares about
        if cpu > 5.0 { continue; }

        // Skip processes using significant memory
        if process.memory() > 500 * 1024 * 1024 { continue; } // 500MB

        // Apply the safety check
        if crate::process_ctrl::is_protected(pid, &name) { continue; }

        // Lower priority to BelowNormal for idle background processes
        let result = crate::process_ctrl::set_process_priority(
            pid,
            name.clone(),
            crate::process_ctrl::ProcessPriority::BelowNormal,
        );

        if result.success {
            throttled += 1;
        }
    }

    if throttled > 0 {
        log::info!(
            "[optimizer] Throttled {} background processes. Reason: {}",
            throttled, reason
        );
        let _ = handle.emit("alert", serde_json::json!({
            "id":      format!("throttle-{}", chrono_timestamp()),
            "type":    "optimization_applied",
            "message": format!(
                "Throttled {} background processes to free CPU resources.",
                throttled
            ),
            "level":   "info",
        }));
    }
}

fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ============================================================
// Tauri Commands
// ============================================================

#[tauri::command]
pub fn get_optimizer_status(
    state: tauri::State<crate::AppState>,
) -> OptimizerStatus {
    state.optimizer.lock().unwrap().status.clone()
}

#[tauri::command]
pub fn set_optimizer_enabled(
    enabled: bool,
    state:   tauri::State<crate::AppState>,
) -> Result<(), String> {
    state.optimizer.lock().unwrap().status.enabled = enabled;
    log::info!("[optimizer] Optimization {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}
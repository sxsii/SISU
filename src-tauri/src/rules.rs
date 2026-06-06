// rules.rs — SISU Rule Engine
//
// The rule engine evaluates system conditions every monitoring
// interval and triggers optimization actions when thresholds
// are crossed.
//
// Design principles:
// - Rules are evaluated in priority order
// - Actions are conservative — we lower priorities, not kill processes
// - The engine never touches protected system processes
// - All actions are logged so the user can see what happened

use crate::monitor::SystemSnapshot;
use crate::profiles::OptimizationProfile;
use serde::{Deserialize, Serialize};

// ============================================================
// Rule Evaluation Context
//
// Everything the rule engine needs to make decisions is
// packaged into this struct. It is built from the latest
// SystemSnapshot and the active profile each evaluation cycle.
// ============================================================

pub struct RuleContext<'a> {
    pub snapshot:         &'a SystemSnapshot,
    pub active_profile:   Option<&'a OptimizationProfile>,
    pub foreground_name:  &'a str,
}

// ============================================================
// Optimization Action
//
// Actions are what the rule engine decides to do.
// They are returned as a Vec and executed by the caller
// (the monitoring loop in lib.rs) rather than executed
// directly here. This keeps the rule engine pure — it only
// decides, it does not act. This makes it easy to test.
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum OptimizationAction {
    /// Lower background process priorities
    ThrottleBackground { reason: String },
    /// Emit an alert to the frontend
    EmitAlert { message: String, level: String },
    /// Activate a named profile
    ActivateProfile { name: String },
    /// Log an optimization event
    LogEvent { event_type: String, detail: String },
}

// ============================================================
// Rule Evaluation
// ============================================================

/// Evaluate all rules against the current context.
/// Returns a list of actions to execute.
/// Rules are evaluated in order — later rules can add to
/// actions already scheduled by earlier rules.
pub fn evaluate(ctx: &RuleContext) -> Vec<OptimizationAction> {
    let mut actions = Vec::new();

    evaluate_cpu_rules(ctx, &mut actions);
    evaluate_memory_rules(ctx, &mut actions);
    evaluate_temperature_rules(ctx, &mut actions);
    evaluate_profile_rules(ctx, &mut actions);

    actions
}

// ---- CPU Rules ----

fn evaluate_cpu_rules(ctx: &RuleContext, actions: &mut Vec<OptimizationAction>) {
    let cpu = ctx.snapshot.cpu.usage_percent;

    // Rule: CPU critical threshold
    // When CPU exceeds the active profile's threshold (or 80% default),
    // throttle background processes to free resources for foreground work.
    let threshold = ctx.active_profile
        .map(|p| p.cpu_threshold)
        .unwrap_or(80.0);

    if cpu > threshold {
        // Only throttle if a profile with restrict_background is active,
        // or if CPU is extremely high (above 90%) regardless of profile
        let should_throttle = ctx.active_profile
            .map(|p| p.restrict_background)
            .unwrap_or(false)
            || cpu > 90.0;

        if should_throttle {
            actions.push(OptimizationAction::ThrottleBackground {
                reason: format!(
                    "CPU at {:.1}% (threshold: {:.0}%)",
                    cpu, threshold
                ),
            });
            actions.push(OptimizationAction::LogEvent {
                event_type: "cpu_throttle".into(),
                detail:     format!("CPU {:.1}% triggered background throttle", cpu),
            });
        }
    }
}

// ---- Memory Rules ----

fn evaluate_memory_rules(ctx: &RuleContext, actions: &mut Vec<OptimizationAction>) {
    let mem = ctx.snapshot.memory.usage_percent;

    let threshold = ctx.active_profile
        .map(|p| p.ram_threshold)
        .unwrap_or(85.0);

    if mem > threshold {
        actions.push(OptimizationAction::EmitAlert {
            message: format!(
                "Memory pressure high: {:.1}% used (threshold: {:.0}%)",
                mem, threshold
            ),
            level: if mem > 92.0 {
                "critical".into()
            } else {
                "warning".into()
            },
        });
        actions.push(OptimizationAction::LogEvent {
            event_type: "memory_pressure".into(),
            detail:     format!("Memory at {:.1}%", mem),
        });
    }
}

// ---- Temperature Rules ----

fn evaluate_temperature_rules(ctx: &RuleContext, actions: &mut Vec<OptimizationAction>) {
    if let Some(temp) = ctx.snapshot.cpu.temperature {
        if temp > 85.0 {
            actions.push(OptimizationAction::EmitAlert {
                message: format!(
                    "CPU temperature elevated: {:.0}°C — consider improving airflow.",
                    temp
                ),
                level: if temp > 95.0 {
                    "critical".into()
                } else {
                    "warning".into()
                },
            });
        }
    }
}

// ---- Profile Rules ----

fn evaluate_profile_rules(ctx: &RuleContext, _actions: &mut Vec<OptimizationAction>) {
    // If no profile is active, check if the foreground app matches
    // any profile's target_apps list and suggest activating it
    if ctx.active_profile.is_none() && !ctx.foreground_name.is_empty() {
        // This information is used by the optimizer — no action here,
        // the foreground detection module handles profile suggestions
    }
}

// ============================================================
// Tests
//
// Rust has a built-in test framework. Functions annotated with
// #[test] are run with `cargo test`. They never run in the
// production binary — only during development testing.
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{CpuInfo, MemoryInfo, SystemSnapshot};

    fn mock_snapshot(cpu: f32, mem: f32) -> SystemSnapshot {
        SystemSnapshot {
            cpu: CpuInfo {
                usage_percent:  cpu,
                per_core_usage: vec![cpu],
                frequency_mhz:  3000,
                temperature:    None,
                core_count:     4,
            },
            memory: MemoryInfo {
                total_kb:      16_000_000,
                used_kb:       (16_000_000.0 * mem / 100.0) as u64,
                available_kb:  (16_000_000.0 * (1.0 - mem / 100.0)) as u64,
                swap_total_kb: 4_000_000,
                swap_used_kb:  0,
                usage_percent: mem,
            },
            disks:       vec![],
            networks:    vec![],
            os_name:     "Test".into(),
            os_version:  "1.0".into(),
            uptime_secs: 3600,
            timestamp:   0,
        }
    }

    #[test]
    fn test_no_actions_when_healthy() {
        let snap = mock_snapshot(40.0, 50.0);
        let ctx  = RuleContext {
            snapshot:        &snap,
            active_profile:  None,
            foreground_name: "",
        };
        let actions = evaluate(&ctx);
        // At 40% CPU and 50% RAM with no active profile,
        // no actions should be triggered
        assert!(actions.is_empty());
    }

    #[test]
    fn test_throttle_fires_above_90_cpu() {
        let snap = mock_snapshot(92.0, 50.0);
        let ctx  = RuleContext {
            snapshot:        &snap,
            active_profile:  None,
            foreground_name: "",
        };
        let actions = evaluate(&ctx);
        // CPU above 90% should always trigger throttle
        let has_throttle = actions.iter().any(|a| matches!(
            a, OptimizationAction::ThrottleBackground { .. }
        ));
        assert!(has_throttle, "Expected ThrottleBackground action at 92% CPU");
    }

    #[test]
    fn test_memory_alert_fires_above_threshold() {
        let snap = mock_snapshot(40.0, 88.0);
        let ctx  = RuleContext {
            snapshot:        &snap,
            active_profile:  None,
            foreground_name: "",
        };
        let actions = evaluate(&ctx);
        let has_alert = actions.iter().any(|a| matches!(
            a, OptimizationAction::EmitAlert { .. }
        ));
        assert!(has_alert, "Expected EmitAlert action at 88% memory");
    }
}
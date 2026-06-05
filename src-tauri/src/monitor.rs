// monitor.rs — SISU System Monitoring Engine
// Written for sysinfo 0.33 API

use serde::{Deserialize, Serialize};
use sysinfo::{
    Components, Disks, Networks, System,
    CpuRefreshKind, MemoryRefreshKind,
    ProcessesToUpdate, RefreshKind,
};
use tauri::Emitter;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;

// ============================================================
// Data Types
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub usage_percent:  f32,
    pub per_core_usage: Vec<f32>,
    pub frequency_mhz:  u64,
    pub temperature:    Option<f32>,
    pub core_count:     usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_kb:      u64,
    pub used_kb:       u64,
    pub available_kb:  u64,
    pub swap_total_kb: u64,
    pub swap_used_kb:  u64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name:            String,
    pub total_bytes:     u64,
    pub available_bytes: u64,
    pub usage_percent:   f32,
    pub file_system:     String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub interface:         String,
    pub bytes_received:    u64,
    pub bytes_transmitted: u64,
    pub recv_per_sec:      u64,
    pub sent_per_sec:      u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid:       u32,
    pub name:      String,
    pub cpu_usage: f32,
    pub memory_kb: u64,
    pub status:    String,
    pub run_time:  u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub cpu:         CpuInfo,
    pub memory:      MemoryInfo,
    pub disks:       Vec<DiskInfo>,
    pub networks:    Vec<NetworkInfo>,
    pub os_name:     String,
    pub os_version:  String,
    pub uptime_secs: u64,
    pub timestamp:   u64,
}

// ============================================================
// SystemState
// ============================================================

pub struct SystemState {
    pub sys:        System,
    pub disks:      Disks,
    pub networks:   Networks,
    pub components: Components,
}

impl SystemState {
    pub fn new() -> Self {
        // In sysinfo 0.33, RefreshKind::new() is replaced by
        // RefreshKind::nothing() as the empty constructor
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        // Two reads needed to establish CPU usage baseline
        std::thread::sleep(Duration::from_millis(200));
        sys.refresh_cpu_all();

        // In sysinfo 0.33, refresh_processes takes ProcessesToUpdate
        // not ProcessRefreshKind as the first argument
        sys.refresh_processes(
            ProcessesToUpdate::All,
            true,
        );

        let mut disks = Disks::new_with_refreshed_list();
        disks.refresh(true);

        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);

        let mut components = Components::new_with_refreshed_list();
        components.refresh(false);

        SystemState { sys, disks, networks, components }
    }

    pub fn refresh_all(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        // sysinfo 0.33: refresh_processes(ProcessesToUpdate, bool)
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.disks.refresh(false);
        self.networks.refresh(true);
        self.components.refresh(false);
    }

    pub fn snapshot(&self) -> SystemSnapshot {
        // ---- CPU ----
        // In sysinfo 0.33 the *Ext traits are gone.
        // Methods like cpu_usage() and frequency() are called
        // directly on the Cpu struct without importing a trait.
        let per_core: Vec<f32> = self.sys
            .cpus()
            .iter()
            .map(|c| c.cpu_usage())
            .collect();

        let core_count = per_core.len();

        let overall_cpu = if core_count > 0 {
            per_core.iter().sum::<f32>() / core_count as f32
        } else {
            0.0
        };

        let freq = self.sys
            .cpus()
            .first()
            .map(|c| c.frequency())
            .unwrap_or(0);

        // In sysinfo 0.33 Component methods are called directly.
        // temperature() returns f32 (not Option<f32>), so we
        // wrap it in Some() ourselves after finding the component.
        let cpu_temp: Option<f32> = self.components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu")
                    || label.contains("core")
                    || label.contains("tctl")
                    || label.contains("package")
            })
            .and_then(|c| c.temperature());

        // ---- Memory ----
        let total_mem = self.sys.total_memory();
        let used_mem  = self.sys.used_memory();
        let avail_mem = self.sys.available_memory();
        let mem_pct   = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };

        // ---- Disks ----
        // In sysinfo 0.33 DiskExt is removed; methods are
        // called directly on the Disk struct
        let disks: Vec<DiskInfo> = self.disks.iter().map(|d| {
            let total = d.total_space();
            let avail = d.available_space();
            let used  = total.saturating_sub(avail);
            let pct   = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            DiskInfo {
                name:            d.name().to_string_lossy().to_string(),
                total_bytes:     total,
                available_bytes: avail,
                usage_percent:   pct,
                // file_system() returns &OsStr in 0.33
                file_system:     d.file_system()
                                  .to_string_lossy()
                                  .to_string(),
            }
        }).collect();

        // ---- Networks ----
        // In sysinfo 0.33 NetworkExt is removed; methods are
        // called directly on NetworkData
        let networks: Vec<NetworkInfo> = self.networks
            .iter()
            .map(|(name, data)| NetworkInfo {
                interface:         name.clone(),
                bytes_received:    data.total_received(),
                bytes_transmitted: data.total_transmitted(),
                recv_per_sec:      data.received(),
                sent_per_sec:      data.transmitted(),
            })
            .collect();

        // ---- Timestamp ----
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SystemSnapshot {
            cpu: CpuInfo {
                usage_percent:  overall_cpu,
                per_core_usage: per_core,
                frequency_mhz:  freq,
                temperature:    cpu_temp,
                core_count,
            },
            memory: MemoryInfo {
                total_kb:      total_mem / 1024,
                used_kb:       used_mem  / 1024,
                available_kb:  avail_mem / 1024,
                swap_total_kb: self.sys.total_swap() / 1024,
                swap_used_kb:  self.sys.used_swap()  / 1024,
                usage_percent: mem_pct,
            },
            disks,
            networks,
            os_name:     System::name()
                             .unwrap_or_else(|| "Unknown".into()),
            os_version:  System::os_version()
                             .unwrap_or_else(|| "Unknown".into()),
            uptime_secs: System::uptime(),
            timestamp,
        }
    }

    pub fn process_list(&self) -> Vec<ProcessInfo> {
        // In sysinfo 0.33, ProcessExt and PidExt are removed.
        // pid() returns Pid which has an as_u32() method directly.
        // name() returns &OsStr, so we use to_string_lossy()
        let mut procs: Vec<ProcessInfo> = self.sys
            .processes()
            .values()
            .map(|p| ProcessInfo {
                pid:       p.pid().as_u32(),
                name:      p.name().to_string_lossy().to_string(),
                cpu_usage: p.cpu_usage(),
                memory_kb: p.memory() / 1024,
                status:    format!("{:?}", p.status()),
                run_time:  p.run_time(),
            })
            .collect();

        procs.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        procs.truncate(200);
        procs
    }
}

// ============================================================
// Tauri Commands
// ============================================================

#[tauri::command]
pub fn get_system_snapshot(
    state: tauri::State<crate::AppState>,
) -> SystemSnapshot {
    let mut sys = state.system.lock().unwrap();
    sys.refresh_all();
    sys.snapshot()
}

#[tauri::command]
pub fn get_process_list(
    state: tauri::State<crate::AppState>,
) -> Vec<ProcessInfo> {
    let mut sys = state.system.lock().unwrap();
    sys.refresh_all();
    sys.process_list()
}

// ============================================================
// Background Monitoring Loop
// ============================================================

pub async fn monitoring_loop(
    state: Arc<Mutex<SystemState>>,
    handle: AppHandle,
) {
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let snapshot = {
            let mut sys = state.lock().unwrap();
            sys.refresh_all();
            sys.snapshot()
        };

        // tauri::Emitter must be in scope (imported at top of file)
        // for .emit() to be available on AppHandle
        let _ = handle.emit("system-update", &snapshot);

        // ---- CPU Alerts ----
        if snapshot.cpu.usage_percent > 90.0 {
            let _ = handle.emit("alert", serde_json::json!({
                "id":      format!("cpu-{}", snapshot.timestamp),
                "type":    "cpu_critical",
                "message": format!("CPU usage critical: {:.1}%", snapshot.cpu.usage_percent),
                "level":   "critical"
            }));
        } else if snapshot.cpu.usage_percent > 75.0 {
            let _ = handle.emit("alert", serde_json::json!({
                "id":      format!("cpu-{}", snapshot.timestamp),
                "type":    "cpu_high",
                "message": format!("CPU usage high: {:.1}%", snapshot.cpu.usage_percent),
                "level":   "warning"
            }));
        }

        // ---- Memory Alerts ----
        if snapshot.memory.usage_percent > 90.0 {
            let _ = handle.emit("alert", serde_json::json!({
                "id":      format!("mem-{}", snapshot.timestamp),
                "type":    "memory_critical",
                "message": format!("Memory usage critical: {:.1}%", snapshot.memory.usage_percent),
                "level":   "critical"
            }));
        } else if snapshot.memory.usage_percent > 80.0 {
            let _ = handle.emit("alert", serde_json::json!({
                "id":      format!("mem-{}", snapshot.timestamp),
                "type":    "memory_high",
                "message": format!("Memory pressure high: {:.1}%", snapshot.memory.usage_percent),
                "level":   "warning"
            }));
        }

        // ---- Temperature Alerts ----
        if let Some(temp) = snapshot.cpu.temperature {
            if temp > 90.0 {
                let _ = handle.emit("alert", serde_json::json!({
                    "id":      format!("temp-{}", snapshot.timestamp),
                    "type":    "temperature_critical",
                    "message": format!("CPU temperature critical: {:.0}°C", temp),
                    "level":   "critical"
                }));
            } else if temp > 75.0 {
                let _ = handle.emit("alert", serde_json::json!({
                    "id":      format!("temp-{}", snapshot.timestamp),
                    "type":    "temperature_high",
                    "message": format!("CPU temperature high: {:.0}°C", temp),
                    "level":   "warning"
                }));
            }
        }
    }
}
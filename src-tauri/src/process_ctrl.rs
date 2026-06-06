// process_ctrl.rs — SISU Process Control Engine
//
// This module provides safe, cross-platform process management.
// All operations include safety checks before execution.
// Critical system processes are never modified regardless of
// what the user requests.
//
// SAFETY PHILOSOPHY:
// - Fail loudly with clear error messages rather than silently
// - Never assume a process is safe to modify without checking
// - Always release OS handles immediately after use
// - Return meaningful errors the frontend can display to the user

use serde::{Deserialize, Serialize};

// ============================================================
// Priority Levels
//
// We define our own priority enum rather than exposing raw OS
// values to the frontend. This gives us a stable API that does
// not change if the underlying OS constants change, and it
// prevents the frontend from sending arbitrary invalid values.
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProcessPriority {
    Low,        // Background work, minimal impact
    BelowNormal,// Slightly below normal
    Normal,     // Default OS priority
    AboveNormal,// Slightly elevated
    High,       // High priority — use sparingly
    // Realtime is intentionally excluded. Realtime priority on
    // Windows can starve the OS scheduler and freeze the system.
    // No user-facing application should set Realtime priority.
}

impl ProcessPriority {
    /// Convert to the Windows PRIORITY_CLASS constant.
    /// These are the actual values passed to SetPriorityClass().
    #[cfg(target_os = "windows")]
    pub fn to_windows_class(&self) -> u32 {
        match self {
            ProcessPriority::Low         => 0x00000040, // IDLE_PRIORITY_CLASS
            ProcessPriority::BelowNormal => 0x00004000, // BELOW_NORMAL_PRIORITY_CLASS
            ProcessPriority::Normal      => 0x00000020, // NORMAL_PRIORITY_CLASS
            ProcessPriority::AboveNormal => 0x00008000, // ABOVE_NORMAL_PRIORITY_CLASS
            ProcessPriority::High        => 0x00000080, // HIGH_PRIORITY_CLASS
        }
    }

    /// Convert to a POSIX nice value for Linux and macOS.
    /// Nice values run from -20 (highest priority) to +19 (lowest).
    /// Normal priority is 0.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn to_nice_value(&self) -> i32 {
        match self {
            ProcessPriority::Low         =>  15,
            ProcessPriority::BelowNormal =>   5,
            ProcessPriority::Normal      =>   0,
            ProcessPriority::AboveNormal =>  -5,
            ProcessPriority::High        => -10,
        }
    }
}

// ============================================================
// Safety Guard
//
// This is the most important part of the entire module.
// Any process whose name appears in PROTECTED_NAMES or whose
// PID appears in PROTECTED_PIDS will be refused unconditionally.
//
// On Windows, PID 0 is the System Idle Process and PID 4 is
// the System process (kernel). Both must never be touched.
// ============================================================

/// Process names that must never be modified.
/// These are critical system processes on Windows, Linux, and macOS.
/// The check is case-insensitive and uses contains() so partial
/// matches also work — e.g. "svchost" matches "svchost.exe".
const PROTECTED_NAMES: &[&str] = &[
    // Windows system processes
    "system",
    "smss",        // Session Manager
    "csrss",       // Client/Server Runtime
    "wininit",     // Windows Initialization
    "winlogon",    // Windows Logon
    "lsass",       // Local Security Authority
    "lsm",         // Local Session Manager
    "services",    // Service Control Manager
    "svchost",     // Service Host (critical instances)
    "dwm",         // Desktop Window Manager
    "explorer",    // Windows Shell — suspending this freezes the desktop
    // Linux system processes
    "init",
    "systemd",
    "kthreadd",
    "kworker",
    "ksoftirqd",
    "migration",
    "rcu_",
    // macOS system processes
    "launchd",
    "kernel_task",
    "WindowServer",
    // SISU itself — never let the app modify its own process
    "sisu",
];

/// PIDs that are always protected regardless of name.
/// On Windows: 0 = Idle, 4 = System kernel
const PROTECTED_PIDS: &[u32] = &[0, 4];

/// Result type returned by all process control operations
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControlResult {
    pub success: bool,
    pub message: String,
}

impl ControlResult {
    fn ok(msg: impl Into<String>) -> Self {
        ControlResult { success: true, message: msg.into() }
    }
    fn err(msg: impl Into<String>) -> Self {
        ControlResult { success: false, message: msg.into() }
    }
}

/// Returns true if a process is protected and must not be modified.
/// Used by the optimizer to skip protected processes during bulk operations.
pub fn is_protected(pid: u32, name: &str) -> bool {
    check_protected(pid, name).is_some()
}

/// Check whether a process is protected.
/// Returns Some(reason) if protected, None if safe to modify.
pub fn check_protected(pid: u32, name: &str) -> Option<String> {
    // Check PID whitelist first — fastest check
    if PROTECTED_PIDS.contains(&pid) {
        return Some(format!(
            "PID {} is a protected system process and cannot be modified.", pid
        ));
    }

    // Check name against protected list (case-insensitive)
    let name_lower = name.to_lowercase();
    for protected in PROTECTED_NAMES {
        if name_lower.contains(protected) {
            return Some(format!(
                "'{}' is a protected system process and cannot be modified.", name
            ));
        }
    }

    None
}



// ============================================================
// Windows Implementation
// ============================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use winapi::um::processthreadsapi::{OpenProcess, SetPriorityClass};
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::winnt::{
        PROCESS_SET_INFORMATION,
        PROCESS_SUSPEND_RESUME,
        PROCESS_TERMINATE,
        PROCESS_QUERY_INFORMATION,
    };
    use winapi::um::processthreadsapi::TerminateProcess;

    /// Open a process handle with the requested access rights.
    /// The handle MUST be closed with CloseHandle() after use.
    /// We use a helper to ensure handles are never leaked.
    unsafe fn open_process(pid: u32, access: u32) -> Result<winapi::um::winnt::HANDLE, String> {
        let handle = OpenProcess(access, 0, pid);
        if handle.is_null() {
            let err = winapi::um::errhandlingapi::GetLastError();
            Err(format!(
                "Failed to open process PID {} (Windows error {}). \
                 Try running SISU as Administrator for this operation.",
                pid, err
            ))
        } else {
            Ok(handle)
        }
    }

    pub fn set_priority(pid: u32, priority: &ProcessPriority) -> ControlResult {
        unsafe {
            match open_process(pid, PROCESS_SET_INFORMATION) {
                Err(e) => ControlResult::err(e),
                Ok(handle) => {
                    let result = SetPriorityClass(handle, priority.to_windows_class());
                    CloseHandle(handle);
                    if result != 0 {
                        ControlResult::ok(format!(
                            "Priority set to {:?} for PID {}.", priority, pid
                        ))
                    } else {
                        let err = winapi::um::errhandlingapi::GetLastError();
                        ControlResult::err(format!(
                            "SetPriorityClass failed for PID {} (error {}).", pid, err
                        ))
                    }
                }
            }
        }
    }

    pub fn suspend_process(pid: u32) -> ControlResult {
        // NtSuspendProcess is an undocumented but widely used Windows API.
        // We load it dynamically from ntdll.dll to avoid a hard dependency.
        // This is the same technique used by Process Explorer and Process Lasso.
        unsafe {
            match open_process(pid, PROCESS_SUSPEND_RESUME) {
                Err(e) => ControlResult::err(e),
                Ok(handle) => {
                    // Load ntdll.dll dynamically
                    let ntdll_name = b"ntdll.dll\0";
                    let ntdll = winapi::um::libloaderapi::GetModuleHandleA(
                        ntdll_name.as_ptr() as *const i8
                    );
                    if ntdll.is_null() {
                        CloseHandle(handle);
                        return ControlResult::err("Failed to load ntdll.dll");
                    }

                    // Get the address of NtSuspendProcess
                    let proc_name = b"NtSuspendProcess\0";
                    let nt_suspend = winapi::um::libloaderapi::GetProcAddress(
                        ntdll,
                        proc_name.as_ptr() as *const i8
                    );
                    if nt_suspend.is_null() {
                        CloseHandle(handle);
                        return ControlResult::err("NtSuspendProcess not found in ntdll.dll");
                    }

                    // Call NtSuspendProcess(handle)
                    type NtSuspendFn = unsafe extern "system" fn(winapi::um::winnt::HANDLE) -> i32;
                    let nt_suspend_fn: NtSuspendFn = std::mem::transmute(nt_suspend);
                    let status = nt_suspend_fn(handle);
                    CloseHandle(handle);

                    if status >= 0 {
                        ControlResult::ok(format!("Process PID {} suspended.", pid))
                    } else {
                        ControlResult::err(format!(
                            "NtSuspendProcess failed for PID {} (NTSTATUS {:#x}).", pid, status
                        ))
                    }
                }
            }
        }
    }

    pub fn resume_process(pid: u32) -> ControlResult {
        unsafe {
            match open_process(pid, PROCESS_SUSPEND_RESUME) {
                Err(e) => ControlResult::err(e),
                Ok(handle) => {
                    let ntdll_name = b"ntdll.dll\0";
                    let ntdll = winapi::um::libloaderapi::GetModuleHandleA(
                        ntdll_name.as_ptr() as *const i8
                    );
                    if ntdll.is_null() {
                        CloseHandle(handle);
                        return ControlResult::err("Failed to load ntdll.dll");
                    }

                    let proc_name = b"NtResumeProcess\0";
                    let nt_resume = winapi::um::libloaderapi::GetProcAddress(
                        ntdll,
                        proc_name.as_ptr() as *const i8
                    );
                    if nt_resume.is_null() {
                        CloseHandle(handle);
                        return ControlResult::err("NtResumeProcess not found in ntdll.dll");
                    }

                    type NtResumeFn = unsafe extern "system" fn(winapi::um::winnt::HANDLE) -> i32;
                    let nt_resume_fn: NtResumeFn = std::mem::transmute(nt_resume);
                    let status = nt_resume_fn(handle);
                    CloseHandle(handle);

                    if status >= 0 {
                        ControlResult::ok(format!("Process PID {} resumed.", pid))
                    } else {
                        ControlResult::err(format!(
                            "NtResumeProcess failed for PID {} (NTSTATUS {:#x}).", pid, status
                        ))
                    }
                }
            }
        }
    }

    pub fn kill_process(pid: u32) -> ControlResult {
        unsafe {
            match open_process(pid, PROCESS_TERMINATE) {
                Err(e) => ControlResult::err(e),
                Ok(handle) => {
                    // Exit code 1 is the conventional "terminated by external request" code
                    let result = TerminateProcess(handle, 1);
                    CloseHandle(handle);
                    if result != 0 {
                        ControlResult::ok(format!("Process PID {} terminated.", pid))
                    } else {
                        let err = winapi::um::errhandlingapi::GetLastError();
                        ControlResult::err(format!(
                            "TerminateProcess failed for PID {} (error {}).", pid, err
                        ))
                    }
                }
            }
        }
    }

    pub fn set_affinity(pid: u32, mask: u64) -> ControlResult {
        unsafe {
            // PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION needed for affinity
            let access = PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION;
            match open_process(pid, access) {
                Err(e) => ControlResult::err(e),
                Ok(handle) => {
                    use winapi::um::winbase::SetProcessAffinityMask;
                    let result = SetProcessAffinityMask(handle, mask as u32);
                    CloseHandle(handle);
                    if result != 0 {
                        ControlResult::ok(format!(
                            "CPU affinity set to mask {:#b} for PID {}.", mask, pid
                        ))
                    } else {
                        let err = winapi::um::errhandlingapi::GetLastError();
                        ControlResult::err(format!(
                            "SetProcessAffinityMask failed for PID {} (error {}).", pid, err
                        ))
                    }
                }
            }
        }
    }
}

// ============================================================
// Linux Implementation
// ============================================================

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;

    pub fn set_priority(pid: u32, priority: &ProcessPriority) -> ControlResult {
        let nice = priority.to_nice_value();
        let ret = unsafe {
            libc::setpriority(libc::PRIO_PROCESS, pid, nice)
        };
        if ret == 0 {
            ControlResult::ok(format!("Nice value set to {} for PID {}.", nice, pid))
        } else {
            ControlResult::err(format!(
                "setpriority failed for PID {}. Are you running as root for high priorities?", pid
            ))
        }
    }

    pub fn suspend_process(pid: u32) -> ControlResult {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGSTOP) };
        if ret == 0 {
            ControlResult::ok(format!("Process PID {} suspended (SIGSTOP).", pid))
        } else {
            ControlResult::err(format!("SIGSTOP failed for PID {}.", pid))
        }
    }

    pub fn resume_process(pid: u32) -> ControlResult {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGCONT) };
        if ret == 0 {
            ControlResult::ok(format!("Process PID {} resumed (SIGCONT).", pid))
        } else {
            ControlResult::err(format!("SIGCONT failed for PID {}.", pid))
        }
    }

    pub fn kill_process(pid: u32) -> ControlResult {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        if ret == 0 {
            ControlResult::ok(format!("Process PID {} killed (SIGKILL).", pid))
        } else {
            ControlResult::err(format!("SIGKILL failed for PID {}.", pid))
        }
    }

    pub fn set_affinity(pid: u32, mask: u64) -> ControlResult {
        unsafe {
            let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
            for i in 0..64u64 {
                if mask & (1 << i) != 0 {
                    libc::CPU_SET(i as usize, &mut cpuset);
                }
            }
            let ret = libc::sched_setaffinity(
                pid as i32,
                std::mem::size_of::<libc::cpu_set_t>(),
                &cpuset,
            );
            if ret == 0 {
                ControlResult::ok(format!("Affinity set for PID {}.", pid))
            } else {
                ControlResult::err(format!("sched_setaffinity failed for PID {}.", pid))
            }
        }
    }
}

// ============================================================
// macOS Implementation
// ============================================================

#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;

    pub fn set_priority(pid: u32, priority: &ProcessPriority) -> ControlResult {
        let nice = priority.to_nice_value();
        let ret = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid, nice) };
        if ret == 0 {
            ControlResult::ok(format!("Nice value set to {} for PID {}.", nice, pid))
        } else {
            ControlResult::err(format!("setpriority failed for PID {}.", pid))
        }
    }

    pub fn suspend_process(pid: u32) -> ControlResult {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGSTOP) };
        if ret == 0 {
            ControlResult::ok(format!("Process PID {} suspended.", pid))
        } else {
            ControlResult::err(format!("SIGSTOP failed for PID {}.", pid))
        }
    }

    pub fn resume_process(pid: u32) -> ControlResult {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGCONT) };
        if ret == 0 {
            ControlResult::ok(format!("Process PID {} resumed.", pid))
        } else {
            ControlResult::err(format!("SIGCONT failed for PID {}.", pid))
        }
    }

    pub fn kill_process(pid: u32) -> ControlResult {
        let ret = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        if ret == 0 {
            ControlResult::ok(format!("Process PID {} killed.", pid))
        } else {
            ControlResult::err(format!("SIGKILL failed for PID {}.", pid))
        }
    }

    pub fn set_affinity(_pid: u32, _mask: u64) -> ControlResult {
        // macOS does not support hard CPU affinity via a public API.
        // The kernel manages core assignment automatically.
        // Thread affinity hints exist via Mach API but are advisory only.
        ControlResult::err(
            "CPU affinity is not supported on macOS. \
             The kernel manages core assignment automatically.".to_string()
        )
    }
}

// ============================================================
// Public Tauri Commands
//
// These are the functions the frontend calls via invoke().
// Each one:
// 1. Runs the safety check first
// 2. Delegates to the platform-specific implementation
// 3. Returns a ControlResult the frontend can display
// ============================================================

#[tauri::command]
pub fn set_process_priority(
    pid:      u32,
    name:     String,
    priority: ProcessPriority,
) -> ControlResult {
    if let Some(reason) = check_protected(pid, &name) {
        return ControlResult::err(reason);
    }

    #[cfg(target_os = "windows")]
    return windows_impl::set_priority(pid, &priority);

    #[cfg(target_os = "linux")]
    return linux_impl::set_priority(pid, &priority);

    #[cfg(target_os = "macos")]
    return macos_impl::set_priority(pid, &priority);

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    ControlResult::err("Process priority control is not supported on this platform.")
}

#[tauri::command]
pub fn suspend_process(pid: u32, name: String) -> ControlResult {
    if let Some(reason) = check_protected(pid, &name) {
        return ControlResult::err(reason);
    }

    #[cfg(target_os = "windows")]
    return windows_impl::suspend_process(pid);

    #[cfg(target_os = "linux")]
    return linux_impl::suspend_process(pid);

    #[cfg(target_os = "macos")]
    return macos_impl::suspend_process(pid);

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    ControlResult::err("Process suspension is not supported on this platform.")
}

#[tauri::command]
pub fn resume_process(pid: u32, name: String) -> ControlResult {
    if let Some(reason) = check_protected(pid, &name) {
        return ControlResult::err(reason);
    }

    #[cfg(target_os = "windows")]
    return windows_impl::resume_process(pid);

    #[cfg(target_os = "linux")]
    return linux_impl::resume_process(pid);

    #[cfg(target_os = "macos")]
    return macos_impl::resume_process(pid);

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    ControlResult::err("Process resume is not supported on this platform.")
}

#[tauri::command]
pub fn kill_process(pid: u32, name: String) -> ControlResult {
    if let Some(reason) = check_protected(pid, &name) {
        return ControlResult::err(reason);
    }

    #[cfg(target_os = "windows")]
    return windows_impl::kill_process(pid);

    #[cfg(target_os = "linux")]
    return linux_impl::kill_process(pid);

    #[cfg(target_os = "macos")]
    return macos_impl::kill_process(pid);

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    ControlResult::err("Process termination is not supported on this platform.")
}

#[tauri::command]
pub fn set_process_affinity(pid: u32, name: String, mask: u64) -> ControlResult {
    if let Some(reason) = check_protected(pid, &name) {
        return ControlResult::err(reason);
    }

    // Validate that the mask is not zero — zero would assign the process
    // to no cores at all, which would hang it indefinitely
    if mask == 0 {
        return ControlResult::err(
            "Affinity mask cannot be zero. \
             At least one CPU core must be assigned.".to_string()
        );
    }

    #[cfg(target_os = "windows")]
    return windows_impl::set_affinity(pid, mask);

    #[cfg(target_os = "linux")]
    return linux_impl::set_affinity(pid, mask);

    #[cfg(target_os = "macos")]
    return macos_impl::set_affinity(pid, mask);

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    ControlResult::err("CPU affinity control is not supported on this platform.")
}
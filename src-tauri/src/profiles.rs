// profiles.rs — SISU Optimization Profile Storage
//
// Profiles are stored as a single JSON file on disk at:
//   Windows: C:\Users\<name>\AppData\Roaming\sisu\profiles.json
//   Linux:   ~/.config/sisu/profiles.json
//   macOS:   ~/Library/Application Support/sisu/profiles.json
//
// The dirs crate gives us the correct path per platform automatically.
// We use a single file for all profiles rather than one file per profile
// because the total data size is small and atomic writes are simpler.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ============================================================
// Data Types
// ============================================================

/// A single optimization profile.
/// All fields are serialized to camelCase for the frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationProfile {
    /// Unique display name — also used as the primary key
    pub name: String,

    /// Human-readable description of what this profile does
    pub description: String,

    /// Process names that trigger this profile when they become active.
    /// Matching is case-insensitive and partial — "chrome" matches "chrome.exe"
    pub target_apps: Vec<String>,

    /// CPU usage % above which background throttling activates (0–100)
    pub cpu_threshold: f32,

    /// RAM usage % above which background suppression activates (0–100)
    pub ram_threshold: f32,

    /// Whether to lower priority of background processes when active
    pub restrict_background: bool,

    /// Whether to apply power-saving optimizations (for laptops)
    pub battery_saver: bool,

    /// Whether this profile is currently enabled/active
    pub active: bool,
}

// ============================================================
// Storage Path
// ============================================================

/// Returns the path to the profiles JSON file.
/// Creates the parent directory if it does not exist.
fn profiles_path() -> Result<PathBuf, String> {
    let mut path = dirs::config_dir()
        .ok_or_else(|| "Could not determine config directory".to_string())?;

    path.push("sisu");

    // Create the directory if it does not exist yet.
    // This runs on first launch when no config exists.
    fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    path.push("profiles.json");
    Ok(path)
}

// ============================================================
// Default Profiles
//
// These are created on first launch when no profiles.json exists.
// They give the user a useful starting point without requiring
// any configuration.
// ============================================================

fn default_profiles() -> Vec<OptimizationProfile> {
    vec![
        OptimizationProfile {
            name:               "Gaming".into(),
            description:        "Maximizes foreground game performance by throttling \
                                 background processes when CPU exceeds 75%.".into(),
            target_apps:        vec![
                "game".into(),
                "steam".into(),
                "epicgames".into(),
                "battle.net".into(),
            ],
            cpu_threshold:      75.0,
            ram_threshold:      80.0,
            restrict_background: true,
            battery_saver:      false,
            active:             false,
        },
        OptimizationProfile {
            name:               "Development".into(),
            description:        "Prioritizes IDE, compiler, and build tool processes \
                                 for faster build times.".into(),
            target_apps:        vec![
                "code".into(),
                "cargo".into(),
                "node".into(),
                "rider".into(),
                "clion".into(),
                "idea".into(),
            ],
            cpu_threshold:      80.0,
            ram_threshold:      85.0,
            restrict_background: false,
            battery_saver:      false,
            active:             false,
        },
        OptimizationProfile {
            name:               "Battery Saver".into(),
            description:        "Reduces background activity to extend battery life \
                                 on laptops and portable devices.".into(),
            target_apps:        vec![],
            cpu_threshold:      60.0,
            ram_threshold:      70.0,
            restrict_background: true,
            battery_saver:      true,
            active:             false,
        },
        OptimizationProfile {
            name:               "Streaming".into(),
            description:        "Balances encoding performance with system \
                                 responsiveness for live streaming workflows.".into(),
            target_apps:        vec![
                "obs".into(),
                "streamlabs".into(),
                "xsplit".into(),
            ],
            cpu_threshold:      70.0,
            ram_threshold:      80.0,
            restrict_background: true,
            battery_saver:      false,
            active:             false,
        },
    ]
}

// ============================================================
// Read / Write Helpers
// ============================================================

/// Load all profiles from disk.
/// Returns default profiles if the file does not exist yet.
fn load_from_disk() -> Result<Vec<OptimizationProfile>, String> {
    let path = profiles_path()?;

    if !path.exists() {
        // First launch — write defaults and return them
        let defaults = default_profiles();
        save_to_disk(&defaults)?;
        return Ok(defaults);
    }

    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read profiles file: {}", e))?;

    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse profiles file: {}", e))
}

/// Write all profiles to disk atomically.
/// We write to a temporary file first then rename, so a crash
/// during the write cannot corrupt the existing profiles file.
fn save_to_disk(profiles: &[OptimizationProfile]) -> Result<(), String> {
    let path = profiles_path()?;

    let json = serde_json::to_string_pretty(profiles)
        .map_err(|e| format!("Failed to serialize profiles: {}", e))?;

    // Write to a temp file alongside the real file
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Atomic rename — on Windows this may fail if the target is locked,
    // but for a config file that risk is negligible
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to finalize profiles file: {}", e))?;

    Ok(())
}

// ============================================================
// Tauri Commands
// ============================================================

/// Load all profiles from disk and return them to the frontend.
#[tauri::command]
pub fn load_profiles() -> Result<Vec<OptimizationProfile>, String> {
    load_from_disk()
}

/// Save a single profile.
/// If a profile with the same name exists it is replaced (upsert).
/// If it is new it is appended.
#[tauri::command]
pub fn save_profile(profile: OptimizationProfile) -> Result<(), String> {
    let mut profiles = load_from_disk()?;

    // Find existing profile with this name and replace it,
    // or push a new one if not found
    if let Some(existing) = profiles.iter_mut().find(|p| p.name == profile.name) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }

    save_to_disk(&profiles)
}

/// Delete a profile by name.
#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    let mut profiles = load_from_disk()?;
    let before = profiles.len();
    profiles.retain(|p| p.name != name);

    if profiles.len() == before {
        return Err(format!("Profile '{}' not found.", name));
    }

    save_to_disk(&profiles)
}

/// Set one profile as active and deactivate all others.
/// Only one profile can be active at a time.
#[tauri::command]
pub fn activate_profile(name: String) -> Result<(), String> {
    let mut profiles = load_from_disk()?;

    let found = profiles.iter().any(|p| p.name == name);
    if !found {
        return Err(format!("Profile '{}' not found.", name));
    }

    for p in profiles.iter_mut() {
        p.active = p.name == name;
    }

    save_to_disk(&profiles)
}

/// Deactivate all profiles.
#[tauri::command]
pub fn deactivate_profiles() -> Result<(), String> {
    let mut profiles = load_from_disk()?;
    for p in profiles.iter_mut() {
        p.active = false;
    }
    save_to_disk(&profiles)
}
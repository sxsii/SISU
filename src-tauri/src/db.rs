// db.rs — SISU SQLite Database Module
//
// Manages all persistent storage beyond profiles.
// Two tables:
//   optimization_events — records every action the optimizer takes
//   performance_snapshots — periodic system state samples for history graphs
//
// The database file lives alongside the profiles JSON:
//   Windows: %APPDATA%\sisu\sisu.db
//   Linux:   ~/.config/sisu/sisu.db
//   macOS:   ~/Library/Application Support/sisu/sisu.db

use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================
// Database Path
// ============================================================

pub fn db_path() -> PathBuf {
    let mut path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    path.push("sisu");
    std::fs::create_dir_all(&path).ok();
    path.push("sisu.db");
    path
}

// ============================================================
// Schema Initialization
//
// Called once at startup. CREATE TABLE IF NOT EXISTS means
// this is safe to run on every launch — it only creates
// tables that do not already exist.
// ============================================================

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        -- Records every action the optimizer takes.
        -- Used for the event history log in the UI.
        CREATE TABLE IF NOT EXISTS optimization_events (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp    INTEGER NOT NULL,
            event_type   TEXT    NOT NULL,
            process_name TEXT,
            action       TEXT    NOT NULL,
            detail       TEXT,
            success      INTEGER NOT NULL DEFAULT 1
        );

        -- Periodic system state samples.
        -- Stored every 30 seconds for historical graphing.
        -- We store less frequently than the 2s monitoring interval
        -- to keep the database size manageable.
        CREATE TABLE IF NOT EXISTS performance_snapshots (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp      INTEGER NOT NULL,
            cpu_usage      REAL    NOT NULL,
            memory_usage   REAL    NOT NULL,
            active_profile TEXT
        );

        -- Index on timestamp for fast range queries
        CREATE INDEX IF NOT EXISTS idx_events_timestamp
            ON optimization_events(timestamp);

        CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp
            ON performance_snapshots(timestamp);
    ")?;
    Ok(())
}

// ============================================================
// Event Logging
// ============================================================

/// Write an optimization event to the database.
/// Called by the optimizer whenever it takes an action.
pub fn log_event(
    conn:         &Connection,
    event_type:   &str,
    process_name: Option<&str>,
    action:       &str,
    detail:       Option<&str>,
    success:      bool,
) -> Result<()> {
    let ts = unix_now();
    conn.execute(
        "INSERT INTO optimization_events
         (timestamp, event_type, process_name, action, detail, success)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            ts,
            event_type,
            process_name,
            action,
            detail,
            success as i32,
        ],
    )?;
    Ok(())
}

/// Write a performance snapshot to the database.
pub fn log_snapshot(
    conn:           &Connection,
    cpu_usage:      f32,
    memory_usage:   f32,
    active_profile: Option<&str>,
) -> Result<()> {
    let ts = unix_now();
    conn.execute(
        "INSERT INTO performance_snapshots
         (timestamp, cpu_usage, memory_usage, active_profile)
         VALUES (?1, ?2, ?3, ?4)",
        params![ts, cpu_usage, memory_usage, active_profile],
    )?;
    Ok(())
}

// ============================================================
// Query Types — returned to the frontend
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventRecord {
    pub id:           i64,
    pub timestamp:    i64,
    pub event_type:   String,
    pub process_name: Option<String>,
    pub action:       String,
    pub detail:       Option<String>,
    pub success:      bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRecord {
    pub timestamp:      i64,
    pub cpu_usage:      f32,
    pub memory_usage:   f32,
    pub active_profile: Option<String>,
}

// ============================================================
// Queries
// ============================================================

/// Fetch the most recent optimization events.
/// `limit` controls how many to return (default: 50).
pub fn get_recent_events(
    conn:  &Connection,
    limit: u32,
) -> Result<Vec<EventRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, event_type, process_name, action, detail, success
         FROM optimization_events
         ORDER BY timestamp DESC
         LIMIT ?1"
    )?;

    let records = stmt.query_map(params![limit], |row| {
        Ok(EventRecord {
            id:           row.get(0)?,
            timestamp:    row.get(1)?,
            event_type:   row.get(2)?,
            process_name: row.get(3)?,
            action:       row.get(4)?,
            detail:       row.get(5)?,
            success:      row.get::<_, i32>(6)? != 0,
        })
    })?
    .collect::<Result<Vec<_>>>()?;

    Ok(records)
}

/// Fetch performance snapshots for a given time range.
/// `since` is a Unix timestamp — only records after this time are returned.
pub fn get_snapshots_since(
    conn:  &Connection,
    since: i64,
) -> Result<Vec<SnapshotRecord>> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, cpu_usage, memory_usage, active_profile
         FROM performance_snapshots
         WHERE timestamp > ?1
         ORDER BY timestamp ASC"
    )?;

    let records = stmt.query_map(params![since], |row| {
        Ok(SnapshotRecord {
            timestamp:      row.get(0)?,
            cpu_usage:      row.get(1)?,
            memory_usage:   row.get(2)?,
            active_profile: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;

    Ok(records)
}

/// Delete events older than `days` days to keep the database small.
pub fn prune_old_events(conn: &Connection, days: u32) -> Result<usize> {
    let cutoff = unix_now() - (days as i64 * 86400);
    let deleted = conn.execute(
        "DELETE FROM optimization_events WHERE timestamp < ?1",
        params![cutoff],
    )?;
    Ok(deleted)
}

/// Delete snapshots older than `days` days.
pub fn prune_old_snapshots(conn: &Connection, days: u32) -> Result<usize> {
    let cutoff = unix_now() - (days as i64 * 86400);
    let deleted = conn.execute(
        "DELETE FROM performance_snapshots WHERE timestamp < ?1",
        params![cutoff],
    )?;
    Ok(deleted)
}

// ============================================================
// Tauri Commands
// ============================================================

/// Get recent optimization events for display in the UI.
#[tauri::command]
pub fn get_event_history(
    limit: Option<u32>,
    state: tauri::State<crate::AppState>,
) -> Result<Vec<EventRecord>, String> {
    let conn = state.db.lock().unwrap();
    get_recent_events(&conn, limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

/// Get performance snapshots from the last N hours.
#[tauri::command]
pub fn get_performance_history(
    hours: Option<u32>,
    state: tauri::State<crate::AppState>,
) -> Result<Vec<SnapshotRecord>, String> {
    let hours = hours.unwrap_or(1);
    let since = unix_now() - (hours as i64 * 3600);
    let conn  = state.db.lock().unwrap();
    get_snapshots_since(&conn, since)
        .map_err(|e| e.to_string())
}

// ============================================================
// Utility
// ============================================================

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
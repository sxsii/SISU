// Hides the console window on Windows in release builds.
// In debug builds the console stays visible so we can read log output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The actual application logic lives in lib.rs.
// main.rs is intentionally kept as thin as possible —
// its only job is to be the binary entry point.
fn main() {
    sisu_lib::run();
}
// platform/mod.rs — Platform abstraction layer
//
// This module routes platform-specific operations to the correct
// implementation based on which OS we are compiling for.
//
// The #[cfg(target_os = "...")] attribute is Rust's compile-time
// conditional compilation system. Only the matching file gets
// compiled into the binary — the others are completely excluded.
// This means a Windows build contains zero Linux or macOS code.

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;
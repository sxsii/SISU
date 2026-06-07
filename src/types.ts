// types.ts — Shared TypeScript type definitions for SISU
//
// These interfaces mirror the Rust structs in monitor.rs exactly.
// The field names use camelCase here because serde's rename_all =
// "camelCase" attribute on the Rust side automatically converts
// snake_case Rust field names to camelCase when serializing to JSON.
//
// If you add a field to a Rust struct, add it here too.
// If the types drift apart, TypeScript will catch the mismatch
// at compile time rather than letting it fail silently at runtime.

// ---- Monitoring Types ----

export interface CpuInfo {
  usagePercent:  number;      // Overall CPU usage 0–100
  perCoreUsage:  number[];    // Per-core usage array, one entry per core
  frequencyMhz:  number;      // Clock speed of first core in MHz
  temperature:   number | null; // CPU temp in Celsius, null if unavailable
  coreCount:     number;      // Total number of logical cores
}

export interface MemoryInfo {
  totalKb:      number;   // Total installed RAM in KB
  usedKb:       number;   // Currently used RAM in KB
  availableKb:  number;   // Free RAM in KB
  swapTotalKb:  number;   // Total swap/page file in KB
  swapUsedKb:   number;   // Used swap in KB
  usagePercent: number;   // Memory usage 0–100
}

export interface DiskInfo {
  name:           string;  // Drive name e.g. "C:" or "/"
  totalBytes:     number;  // Total capacity in bytes
  availableBytes: number;  // Free space in bytes
  usagePercent:   number;  // Usage 0–100
  fileSystem:     string;  // e.g. "NTFS", "ext4", "apfs"
}

export interface NetworkInfo {
  interface:        string;  // Interface name e.g. "Wi-Fi", "Ethernet"
  bytesReceived:    number;  // Total bytes received since boot
  bytesTransmitted: number;  // Total bytes sent since boot
  recvPerSec:       number;  // Bytes received in last interval
  sentPerSec:       number;  // Bytes sent in last interval
}

export interface ProcessInfo {
  pid:      number;   // Process ID
  name:     string;   // Process name e.g. "chrome.exe"
  cpuUsage: number;   // CPU usage 0–100 (can exceed 100 on multi-core)
  memoryKb: number;   // Memory used in KB
  status:   string;   // e.g. "Run", "Sleep", "Idle"
  runTime:  number;   // How long the process has been running in seconds
}

export interface SystemSnapshot {
  cpu:        CpuInfo;
  memory:     MemoryInfo;
  disks:      DiskInfo[];
  networks:   NetworkInfo[];
  osName:     string;   // e.g. "Windows 11"
  osVersion:  string;   // e.g. "22H2"
  uptimeSecs: number;   // System uptime in seconds
  timestamp:  number;   // Unix timestamp of this snapshot
}

// ---- Alert Types ----

export type AlertLevel = "info" | "warning" | "critical";

export interface Alert {
  id:      string;       // Unique ID e.g. "cpu-1234567890"
  type:    string;       // e.g. "cpu_high", "memory_critical"
  message: string;       // Human-readable description
  level:   AlertLevel;   // Severity level
}

// ---- Profile Types ----
// These will be fully used in Phase 6 when we build the profiles system.
// Defined here now so components can reference them without circular imports.

export interface OptimizationProfile {
  name:               string;    // Profile name e.g. "Gaming"
  targetApps:         string[];  // App names that trigger this profile
  cpuThreshold:       number;    // CPU % that triggers optimization
  ramThreshold:       number;    // RAM % that triggers optimization
  restrictBackground: boolean;   // Whether to throttle background apps
  batterySaver:       boolean;   // Whether to enable power saving mode
  description:        string;    // Human-readable profile description
  active:           boolean;   // Whether this profile is currently active
}

// ---- Utility Types ----

// Tab names for the main navigation
export type TabName = "dashboard" | "processes" | "profiles" | "alerts";

// Color levels used for traffic-light colouring throughout the UI
export type HealthLevel = "good" | "warn" | "critical";

// Returns the health level for a given percentage value
export function getHealthLevel(pct: number): HealthLevel {
  if (pct >= 85) return "critical";
  if (pct >= 65) return "warn";
  return "good";
}

// Returns a CSS colour string for a given health level
export function healthColor(level: HealthLevel): string {
  switch (level) {
    case "critical": return "#e24b4a";
    case "warn":     return "#d97706";
    case "good":     return "#16a34a";
  }
}

// Formats a kilobyte value into a human-readable string
// e.g. 1536000 → "1.5 GB", 512000 → "500 MB"
export function fmtKb(kb: number): string {
  if (kb >= 1024 * 1024) return `${(kb / 1024 / 1024).toFixed(1)} GB`;
  if (kb >= 1024)        return `${(kb / 1024).toFixed(0)} MB`;
  return `${kb} KB`;
}

// Formats a byte value into a human-readable string
export function fmtBytes(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
  if (bytes >= 1e3) return `${(bytes / 1e3).toFixed(0)} KB`;
  return `${bytes} B`;
}

// Formats seconds into h m s display
// e.g. 3723 → "1h 2m"
export function fmtUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}
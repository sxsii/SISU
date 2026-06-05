// Dashboard.tsx — SISU main dashboard view
//
// Displays real-time system metrics received from the Rust backend:
// - Stat cards: CPU, Memory, Disk, Uptime
// - Live line graph: CPU and memory over last 60 seconds
// - Per-core usage grid

import { useState, useEffect } from "react";
import {
  LineChart, Line, XAxis, YAxis,
  CartesianGrid, Tooltip, ResponsiveContainer,
} from "recharts";
import {
  SystemSnapshot,
  getHealthLevel, healthColor,
  fmtKb, fmtBytes, fmtUptime,
} from "../types";

// ---- Types ----

interface Props {
  snapshot: SystemSnapshot | null;
}

// One data point in the rolling history graph
interface HistoryPoint {
  time:   string;   // Display label e.g. "14:32:08"
  cpu:    number;   // CPU usage 0–100
  memory: number;   // Memory usage 0–100
}

// ---- StatCard sub-component ----
// Displays a single metric with a label, value, subtitle, and bar

interface StatCardProps {
  label:    string;
  value:    string;
  sub:      string;
  percent?: number;   // If provided, renders a usage bar
  color:    string;
}

function StatCard({ label, value, sub, percent, color }: StatCardProps) {
  return (
    <div className="stat-card">
      <div className="stat-label">{label}</div>
      <div className="stat-value" style={{ color }}>{value}</div>
      <div className="stat-sub">{sub}</div>
      {percent !== undefined && (
        <div className="stat-bar-bg">
          <div
            className="stat-bar-fill"
            style={{ width: `${Math.min(percent, 100)}%`, background: color }}
          />
        </div>
      )}
    </div>
  );
}

// ---- Dashboard component ----

const MAX_HISTORY = 30; // 30 points × 2 seconds = 60 seconds of history

export default function Dashboard({ snapshot }: Props) {
  // Rolling history array for the line graph.
  // We store it in component state because only the Dashboard needs it —
  // keeping it here avoids polluting App.tsx with display-only data.
  const [history, setHistory] = useState<HistoryPoint[]>([]);

  // Every time a new snapshot arrives, append a new history point
  // and drop the oldest one if we exceed MAX_HISTORY entries.
  useEffect(() => {
    if (!snapshot) return;

    const time = new Date().toLocaleTimeString([], {
      hour:   "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    setHistory((prev) => {
      const next: HistoryPoint = {
        time,
        cpu:    parseFloat(snapshot.cpu.usagePercent.toFixed(1)),
        memory: parseFloat(snapshot.memory.usagePercent.toFixed(1)),
      };
      // Keep only the last MAX_HISTORY points
      return [...prev.slice(-(MAX_HISTORY - 1)), next];
    });
  }, [snapshot]);

  // Loading state — shown until the first snapshot arrives
  if (!snapshot) {
    return (
      <div className="dashboard-loading">
        <div className="loading-spinner" />
        <p>Connecting to system monitor...</p>
      </div>
    );
  }

  // ---- Derived display values ----

  const cpuLevel   = getHealthLevel(snapshot.cpu.usagePercent);
  const cpuColor   = healthColor(cpuLevel);
  const memLevel   = getHealthLevel(snapshot.memory.usagePercent);
  const memColor   = healthColor(memLevel);

  const primaryDisk = snapshot.disks[0] ?? null;
  const diskLevel   = primaryDisk
    ? getHealthLevel(primaryDisk.usagePercent)
    : "good";
  const diskColor   = healthColor(diskLevel);

  const cpuTempStr = snapshot.cpu.temperature !== null
    ? ` · ${snapshot.cpu.temperature.toFixed(0)}°C`
    : "";

  const cpuSubStr = `${snapshot.cpu.frequencyMhz} MHz${cpuTempStr}`;

  const memSubStr = `${fmtKb(snapshot.memory.usedKb)} / ${fmtKb(snapshot.memory.totalKb)}`;

  const diskSubStr = primaryDisk
    ? `${fmtBytes(primaryDisk.availableBytes)} free`
    : "No disk data";

  // ---- Network display ----
  // Sum all interfaces for a total throughput figure
  const totalRecv = snapshot.networks.reduce(
    (sum, n) => sum + n.recvPerSec, 0
  );
  const totalSent = snapshot.networks.reduce(
    (sum, n) => sum + n.sentPerSec, 0
  );

  return (
    <div className="dashboard">

      {/* ---- Stat cards row ---- */}
      <div className="stat-row">
        <StatCard
          label="CPU"
          value={`${snapshot.cpu.usagePercent.toFixed(1)}%`}
          sub={cpuSubStr}
          percent={snapshot.cpu.usagePercent}
          color={cpuColor}
        />
        <StatCard
          label="Memory"
          value={`${snapshot.memory.usagePercent.toFixed(1)}%`}
          sub={memSubStr}
          percent={snapshot.memory.usagePercent}
          color={memColor}
        />
        <StatCard
          label={primaryDisk ? `Disk (${primaryDisk.name})` : "Disk"}
          value={primaryDisk ? `${primaryDisk.usagePercent.toFixed(1)}%` : "—"}
          sub={diskSubStr}
          percent={primaryDisk?.usagePercent}
          color={diskColor}
        />
        <StatCard
          label="Uptime"
          value={fmtUptime(snapshot.uptimeSecs)}
          sub={`${snapshot.osName}`}
          color="var(--text-secondary)"
        />
      </div>

      {/* ---- Network row ---- */}
      <div className="network-row">
        <div className="network-card">
          <span className="net-label">↓ Download</span>
          <span className="net-value">{fmtBytes(totalRecv)}/s</span>
        </div>
        <div className="network-card">
          <span className="net-label">↑ Upload</span>
          <span className="net-value">{fmtBytes(totalSent)}/s</span>
        </div>
        <div className="network-card">
          <span className="net-label">Swap</span>
          <span className="net-value">
            {fmtKb(snapshot.memory.swapUsedKb)} / {fmtKb(snapshot.memory.swapTotalKb)}
          </span>
        </div>
        <div className="network-card">
          <span className="net-label">Cores</span>
          <span className="net-value">{snapshot.cpu.coreCount}</span>
        </div>
      </div>

      {/* ---- Live graph ---- */}
      <div className="chart-card">
        <div className="chart-header">
          <span className="chart-title">CPU &amp; Memory — last 60s</span>
          <div className="chart-legend">
            <span className="legend-item">
              <span className="legend-dot" style={{ background: cpuColor }} />
              CPU
            </span>
            <span className="legend-item">
              <span className="legend-dot" style={{ background: memColor }} />
              Memory
            </span>
          </div>
        </div>
        <ResponsiveContainer width="100%" height={180}>
          <LineChart
            data={history}
            margin={{ top: 4, right: 8, left: -20, bottom: 0 }}
          >
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="var(--border)"
              vertical={false}
            />
            <XAxis
              dataKey="time"
              tick={{ fontSize: 10, fill: "var(--text-tertiary)" }}
              tickLine={false}
              axisLine={false}
              // Only show every 5th label to avoid crowding
              interval={4}
            />
            <YAxis
              domain={[0, 100]}
              tick={{ fontSize: 10, fill: "var(--text-tertiary)" }}
              tickLine={false}
              axisLine={false}
              unit="%"
            />
            <Tooltip
              contentStyle={{
                background:   "var(--surface-2)",
                border:       "1px solid var(--border)",
                borderRadius: "8px",
                fontSize:     "12px",
              }}
              labelStyle={{ color: "var(--text-secondary)" }}
              itemStyle={{ color: "var(--text-primary)" }}
              formatter={(val: number) => [`${val.toFixed(1)}%`]}
            />
            <Line
              type="monotone"
              dataKey="cpu"
              stroke={cpuColor}
              strokeWidth={1.5}
              dot={false}
              isAnimationActive={false}
              name="CPU"
            />
            <Line
              type="monotone"
              dataKey="memory"
              stroke={memColor}
              strokeWidth={1.5}
              dot={false}
              isAnimationActive={false}
              name="Memory"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* ---- Per-core grid ---- */}
      <div className="core-card">
        <div className="core-title">Per-core load</div>
        <div className="core-grid">
          {snapshot.cpu.perCoreUsage.map((usage, i) => {
            const level = getHealthLevel(usage);
            const color = healthColor(level);
            return (
              <div key={i} className="core-chip">
                <div className="core-name">C{i}</div>
                <div className="core-bar-bg">
                  <div
                    className="core-bar-fill"
                    style={{
                      height:     `${usage}%`,
                      background: color,
                    }}
                  />
                </div>
                <div className="core-pct">{usage.toFixed(0)}%</div>
              </div>
            );
          })}
        </div>
      </div>

    </div>
  );
}
// App.tsx — SISU root component
//
// This component is responsible for:
// 1. Listening to backend events (system-update, alert)
// 2. Storing the latest system snapshot in React state
// 3. Rendering the tab navigation
// 4. Passing data down to child components as props
//
// It deliberately does no rendering of actual metrics —
// that is the job of the child components. App.tsx is purely
// a data coordinator and layout shell.

import { useState, useEffect, useCallback } from "react";
import { invoke }  from "@tauri-apps/api/core";
import { listen }  from "@tauri-apps/api/event";
import {
  SystemSnapshot, Alert, TabName,
} from "./types";
import Dashboard    from "./components/Dashboard";
import ProcessTable from "./components/ProcessTable";
import "./App.css";
import ProfilesPanel from "./components/ProfilesPanel";

// ---- Tab configuration ----
// Defined outside the component so it is not recreated on every render
const TABS: { key: TabName; label: string }[] = [
  { key: "dashboard",  label: "Dashboard"  },
  { key: "processes",  label: "Processes"  },
  { key: "profiles",   label: "Profiles"   },
  { key: "alerts",     label: "Alerts"     },
];

export default function App() {
  // ---- State ----

  // The latest system snapshot received from the Rust backend.
  // Starts as null — the Dashboard shows a loading state until
  // the first snapshot arrives (within ~2 seconds of launch).
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);

  // Accumulated alerts from the backend threshold checks.
  // New alerts are prepended so the most recent appears first.
  const [alerts, setAlerts] = useState<Alert[]>([]);

  // Which tab is currently active
  const [activeTab, setActiveTab] = useState<TabName>("dashboard");

  // Dark mode toggle — default true (dark)
  const [dark, setDark] = useState(true);

  // ---- Backend communication ----

  // Fetch a snapshot on demand via invoke().
  // invoke() calls a Rust #[tauri::command] function and returns
  // a Promise with the JSON result. We use useCallback so this
  // function reference is stable across renders.
  const fetchSnapshot = useCallback(async () => {
    try {
      const s = await invoke<SystemSnapshot>("get_system_snapshot");
      setSnapshot(s);
    } catch (err) {
      console.error("Failed to fetch snapshot:", err);
    }
  }, []);

  useEffect(() => {
    // Fetch initial snapshot immediately on mount so we do not
    // wait 2 seconds for the first monitoring loop emission
    fetchSnapshot();

    // listen() subscribes to events emitted by the Rust backend
    // via handle.emit(). It returns a Promise<UnlistenFn> —
    // a function we must call on cleanup to prevent memory leaks.
    const unlistenSnapshot = listen<SystemSnapshot>(
      "system-update",
      (event) => {
        // event.payload is the deserialized SystemSnapshot struct
        setSnapshot(event.payload);
      }
    );

    const unlistenAlert = listen<Alert>(
      "alert",
      (event) => {
        setAlerts((prev) => {
          // Avoid duplicate alerts with the same ID.
          // The backend emits the same alert every 2 seconds while
          // the condition persists — we only want to show it once.
          if (prev.some((a) => a.id === event.payload.id)) return prev;
          // Prepend new alert, keep at most 50
          return [event.payload, ...prev].slice(0, 50);
        });
      }
    );

    // Cleanup function — React calls this when the component unmounts.
    // We resolve the Promise and call the unlisten function to
    // deregister the event listeners from Tauri's event system.
    return () => {
      unlistenSnapshot.then((f) => f());
      unlistenAlert.then((f) => f());
    };
  }, [fetchSnapshot]);

  // ---- Render ----

  const unreadAlerts = alerts.filter((a) => a.level === "critical").length;

  return (
    <div className={`app ${dark ? "dark" : "light"}`}>

      {/* ---- Header ---- */}
      <header className="app-header">
        <div className="header-brand">
          <span className="brand-name">SISU</span>
          {snapshot && (
            <span className="os-badge">
              {snapshot.osName} {snapshot.osVersion}
            </span>
          )}
        </div>

        {/* Tab navigation */}
        <nav className="tab-nav" role="tablist">
          {TABS.map((tab) => (
            <button
              key={tab.key}
              role="tab"
              aria-selected={activeTab === tab.key}
              className={`tab-btn ${activeTab === tab.key ? "active" : ""}`}
              onClick={() => setActiveTab(tab.key)}
            >
              {tab.label}
              {/* Show unread critical alert count on Alerts tab */}
              {tab.key === "alerts" && unreadAlerts > 0 && (
                <span className="alert-badge">{unreadAlerts}</span>
              )}
            </button>
          ))}
        </nav>

        <div className="header-actions">
          <button
            className="icon-btn"
            title={dark ? "Switch to light mode" : "Switch to dark mode"}
            onClick={() => setDark((d) => !d)}
          >
            {dark ? "☀" : "☾"}
          </button>
        </div>
      </header>

      {/* ---- Main content area ---- */}
      <main className="app-main">
        {activeTab === "dashboard" && (
          <Dashboard snapshot={snapshot} />
        )}
        {activeTab === "processes" && (
          <ProcessTable />
        )}
        {activeTab === "profiles" && (
          <ProfilesPanel />
        )}
        {activeTab === "alerts" && (
          <div className="placeholder-panel">
            <div className="alerts-list">
              {alerts.length === 0 && (
                <p className="empty-state">No alerts. System is healthy.</p>
              )}
              {alerts.map((a) => (
                <div key={a.id} className={`alert-item level-${a.level}`}>
                  <span className="alert-level">{a.level.toUpperCase()}</span>
                  <span className="alert-msg">{a.message}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
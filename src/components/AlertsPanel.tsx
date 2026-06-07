// AlertsPanel.tsx — Live alerts and optimization event history

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "../types";

// ---- Types ----

interface EventRecord {
  id:          number;
  timestamp:   number;
  eventType:   string;
  processName: string | null;
  action:      string;
  detail:      string | null;
  success:     boolean;
}

interface Props {
  alerts:  Alert[];
  onClear: () => void;
}

// ---- Helpers ----

function fmtTimestamp(unix: number): string {
  return new Date(unix * 1000).toLocaleTimeString([], {
    hour:   "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function fmtEventType(type: string): string {
  return type
    .replace(/_/g, " ")
    .replace(/\b\w/g, c => c.toUpperCase());
}

// ---- Live alert item ----

function AlertItem({ alert }: { alert: Alert }) {
  return (
    <div className={`alert-item level-${alert.level}`}>
      <div className="alert-left">
        <span className={`alert-dot dot-${alert.level}`} />
        <div>
          <div className="alert-msg">{alert.message}</div>
          <div className="alert-type">{alert.type.replace(/_/g, " ")}</div>
        </div>
      </div>
      <span className={`alert-level-badge badge-${alert.level}`}>
        {alert.level.toUpperCase()}
      </span>
    </div>
  );
}

// ---- Event history item ----

function EventItem({ event }: { event: EventRecord }) {
  return (
    <div className={`event-item ${event.success ? "" : "event-failed"}`}>
      <div className="event-time">{fmtTimestamp(event.timestamp)}</div>
      <div className="event-body">
        <div className="event-type">{fmtEventType(event.eventType)}</div>
        {event.processName && (
          <div className="event-process">{event.processName}</div>
        )}
        {event.detail && (
          <div className="event-detail">{event.detail}</div>
        )}
      </div>
      <div className={`event-status ${event.success ? "status-ok" : "status-fail"}`}>
        {event.success ? "OK" : "FAIL"}
      </div>
    </div>
  );
}

// ---- Main AlertsPanel ----

type ViewMode = "alerts" | "history";

export default function AlertsPanel({ alerts, onClear }: Props) {
  const [view,    setView]    = useState<ViewMode>("alerts");
  const [history, setHistory] = useState<EventRecord[]>([]);
  const [loading, setLoading] = useState(false);

  const loadHistory = useCallback(async () => {
    setLoading(true);
    try {
      const records = await invoke<EventRecord[]>("get_event_history", {
        limit: 100,
      });
      setHistory(records);
    } catch (err) {
      console.error("Failed to load event history:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  // Load history when switching to history view
  useEffect(() => {
    if (view === "history") loadHistory();
  }, [view, loadHistory]);

  const criticalCount = alerts.filter(a => a.level === "critical").length;
  const warningCount  = alerts.filter(a => a.level === "warning").length;

  return (
    <div className="alerts-panel">

      {/* Summary bar */}
      <div className="alerts-summary">
        <div className="summary-stat">
          <span className="summary-num summary-critical">{criticalCount}</span>
          <span className="summary-label">Critical</span>
        </div>
        <div className="summary-stat">
          <span className="summary-num summary-warning">{warningCount}</span>
          <span className="summary-label">Warnings</span>
        </div>
        <div className="summary-stat">
          <span className="summary-num">{alerts.length}</span>
          <span className="summary-label">Total alerts</span>
        </div>
        <div className="summary-stat">
          <span className="summary-num">{history.length}</span>
          <span className="summary-label">DB events</span>
        </div>
      </div>

      {/* View toggle + actions */}
      <div className="alerts-toolbar">
        <div className="view-toggle">
          <button
            className={`view-btn ${view === "alerts" ? "view-btn-active" : ""}`}
            onClick={() => setView("alerts")}
          >
            Live Alerts
            {alerts.length > 0 && (
              <span className="view-count">{alerts.length}</span>
            )}
          </button>
          <button
            className={`view-btn ${view === "history" ? "view-btn-active" : ""}`}
            onClick={() => setView("history")}
          >
            Event History
            {history.length > 0 && (
              <span className="view-count">{history.length}</span>
            )}
          </button>
        </div>

        <div className="alerts-actions">
          {view === "alerts" && alerts.length > 0 && (
            <button className="btn-secondary" onClick={onClear}>
              Clear All
            </button>
          )}
          {view === "history" && (
            <button className="btn-secondary" onClick={loadHistory}>
              ↻ Refresh
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      {view === "alerts" && (
        <div className="alerts-list">
          {alerts.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">✓</div>
              <div className="empty-title">No active alerts</div>
              <div className="empty-sub">System is running normally</div>
            </div>
          ) : (
            alerts.map(a => <AlertItem key={a.id} alert={a} />)
          )}
        </div>
      )}

      {view === "history" && (
        <div className="alerts-list">
          {loading ? (
            <div className="dashboard-loading">
              <div className="loading-spinner" />
              <p>Loading history...</p>
            </div>
          ) : history.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">📋</div>
              <div className="empty-title">No events recorded yet</div>
              <div className="empty-sub">
                Events appear here when the optimizer takes action.
                Enable optimization in the Profiles tab to start.
              </div>
            </div>
          ) : (
            history.map(e => <EventItem key={e.id} event={e} />)
          )}
        </div>
      )}
    </div>
  );
}
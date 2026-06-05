// ProcessTable.tsx — Process list with context menu actions

import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ProcessInfo, fmtKb, fmtUptime } from "../types";

type SortKey = "cpuUsage" | "memoryKb" | "name" | "pid";
type SortDir = "asc" | "desc";

// ---- Types ----

interface ControlResult {
  success: boolean;
  message: string;
}

// Priority options shown in the context menu
const PRIORITY_OPTIONS = [
  { label: "Low",          value: "low"         },
  { label: "Below Normal", value: "belowNormal"  },
  { label: "Normal",       value: "normal"       },
  { label: "Above Normal", value: "aboveNormal"  },
  { label: "High",         value: "high"         },
] as const;

// ---- Context menu component ----

interface ContextMenuProps {
  x:       number;
  y:       number;
  process: ProcessInfo;
  onClose: () => void;
  onResult: (msg: string, ok: boolean) => void;
}

function ContextMenu({ x, y, process: proc, onClose, onResult }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  // Close when clicking outside the menu
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  const invoke_action = async (
    command: string,
    extra?: Record<string, unknown>
  ) => {
    onClose();
    try {
      const result = await invoke<ControlResult>(command, {
        pid:  proc.pid,
        name: proc.name,
        ...extra,
      });
      onResult(result.message, result.success);
    } catch (err) {
      onResult(String(err), false);
    }
  };

  // Clamp menu position so it never renders off-screen
  const menuStyle: React.CSSProperties = {
    position: "fixed",
    top:      Math.min(y, window.innerHeight - 280),
    left:     Math.min(x, window.innerWidth  - 200),
    zIndex:   1000,
  };

  return (
    <div ref={ref} className="context-menu" style={menuStyle}>
      {/* Process header */}
      <div className="ctx-header">
        <div className="ctx-proc-name">{proc.name}</div>
        <div className="ctx-proc-pid">PID {proc.pid}</div>
      </div>
      <div className="ctx-divider" />

      {/* Suspend / Resume / Kill */}
      <button
        className="ctx-item"
        onClick={() => invoke_action("suspend_process")}
      >
        ⏸ Suspend
      </button>
      <button
        className="ctx-item"
        onClick={() => invoke_action("resume_process")}
      >
        ▶ Resume
      </button>
      <div className="ctx-divider" />

      {/* Priority submenu */}
      <div className="ctx-section-label">Set Priority</div>
      {PRIORITY_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          className="ctx-item ctx-item-indent"
          onClick={() =>
            invoke_action("set_process_priority", { priority: opt.value })
          }
        >
          {opt.label}
        </button>
      ))}
      <div className="ctx-divider" />

      {/* Kill — destructive action, styled red */}
      <button
        className="ctx-item ctx-item-danger"
        onClick={() => {
          if (window.confirm(
            `Terminate "${proc.name}" (PID ${proc.pid})?\n\nThis cannot be undone.`
          )) {
            invoke_action("kill_process");
          } else {
            onClose();
          }
        }}
      >
        ✕ Terminate Process
      </button>
    </div>
  );
}

// ---- Toast notification ----

interface ToastProps {
  message: string;
  success: boolean;
  onDone:  () => void;
}

function Toast({ message, success, onDone }: ToastProps) {
  useEffect(() => {
    const t = setTimeout(onDone, 3500);
    return () => clearTimeout(t);
  }, [onDone]);

  return (
    <div className={`toast ${success ? "toast-ok" : "toast-err"}`}>
      <span>{success ? "✓" : "✗"}</span>
      <span>{message}</span>
    </div>
  );
}

// ---- Main ProcessTable component ----

export default function ProcessTable() {
  const [processes, setProcesses]   = useState<ProcessInfo[]>([]);
  const [search,    setSearch]      = useState("");
  const [sortKey,   setSortKey]     = useState<SortKey>("cpuUsage");
  const [sortDir,   setSortDir]     = useState<SortDir>("desc");
  const [loading,   setLoading]     = useState(true);
  const [ctxMenu,   setCtxMenu]     = useState<{
    x: number; y: number; proc: ProcessInfo
  } | null>(null);
  const [toast, setToast] = useState<{
    message: string; success: boolean
  } | null>(null);

  const fetchProcesses = useCallback(async () => {
    try {
      const list = await invoke<ProcessInfo[]>("get_process_list");
      setProcesses(list);
      setLoading(false);
    } catch (err) {
      console.error("Failed to fetch processes:", err);
    }
  }, []);

  useEffect(() => {
    fetchProcesses();
    const id = setInterval(fetchProcesses, 3000);
    return () => clearInterval(id);
  }, [fetchProcesses]);

  const handleSort = (key: SortKey) => {
    if (key === sortKey) setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    else { setSortKey(key); setSortDir("desc"); }
  };

  const displayed = useMemo(() => {
    let list = processes;
    if (search.trim()) {
      const term = search.toLowerCase();
      list = list.filter((p) => p.name.toLowerCase().includes(term));
    }
    return [...list].sort((a, b) => {
      const diff = sortKey === "name"
        ? a.name.localeCompare(b.name)
        : (a[sortKey] as number) - (b[sortKey] as number);
      return sortDir === "asc" ? diff : -diff;
    });
  }, [processes, search, sortKey, sortDir]);

  const arrow = (key: SortKey) =>
    key === sortKey
      ? <span className="sort-arrow">{sortDir === "asc" ? "▲" : "▼"}</span>
      : null;

  const handleRowContext = (e: React.MouseEvent, proc: ProcessInfo) => {
    e.preventDefault();
    setCtxMenu({ x: e.clientX, y: e.clientY, proc });
  };

  if (loading) {
    return (
      <div className="dashboard-loading">
        <div className="loading-spinner" />
        <p>Loading processes...</p>
      </div>
    );
  }

  return (
    <div className="process-panel">

      {/* Toolbar */}
      <div className="process-toolbar">
        <input
          className="search-input"
          type="text"
          placeholder="Search processes..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <button className="btn-secondary" onClick={fetchProcesses}>
          ↻ Refresh
        </button>
        <span className="process-hint">Right-click a process for actions</span>
        <span className="process-count">
          {displayed.length} / {processes.length}
        </span>
      </div>

      {/* Table */}
      <div className="table-wrapper">
        <table className="process-table">
          <thead>
            <tr>
              <th onClick={() => handleSort("pid")} className="th-sort">
                PID {arrow("pid")}
              </th>
              <th onClick={() => handleSort("name")} className="th-sort">
                Name {arrow("name")}
              </th>
              <th onClick={() => handleSort("cpuUsage")} className="th-sort">
                CPU % {arrow("cpuUsage")}
              </th>
              <th onClick={() => handleSort("memoryKb")} className="th-sort">
                Memory {arrow("memoryKb")}
              </th>
              <th>Status</th>
              <th>Runtime</th>
            </tr>
          </thead>
          <tbody>
            {displayed.map((p) => (
              <tr
                key={p.pid}
                className={p.cpuUsage > 50 ? "row-active" : ""}
                onContextMenu={(e) => handleRowContext(e, p)}
              >
                <td className="td-pid">{p.pid}</td>
                <td className="td-name" title={p.name}>{p.name}</td>
                <td className={
                  p.cpuUsage > 80 ? "td-critical" :
                  p.cpuUsage > 40 ? "td-warn" : ""
                }>
                  {p.cpuUsage.toFixed(1)}%
                </td>
                <td>{fmtKb(p.memoryKb)}</td>
                <td><span className="status-chip">{p.status}</span></td>
                <td>{fmtUptime(Math.min(p.runTime, 86400 * 365))}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Context menu */}
      {ctxMenu && (
        <ContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          process={ctxMenu.proc}
          onClose={() => setCtxMenu(null)}
          onResult={(msg, ok) => {
            setCtxMenu(null);
            setToast({ message: msg, success: ok });
            fetchProcesses();
          }}
        />
      )}

      {/* Toast feedback */}
      {toast && (
        <Toast
          message={toast.message}
          success={toast.success}
          onDone={() => setToast(null)}
        />
      )}
    </div>
  );
}
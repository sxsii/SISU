// ProfilesPanel.tsx — Optimization profile management UI

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OptimizationProfile } from "../types";

// ---- Empty profile template for the create form ----
const EMPTY_PROFILE: OptimizationProfile = {
  name:               "",
  description:        "",
  targetApps:         [],
  cpuThreshold:       80,
  ramThreshold:       85,
  restrictBackground: false,
  batterySaver:       false,
  active:             false,
};

// ---- Profile card component ----

interface ProfileCardProps {
  profile:    OptimizationProfile;
  onActivate: (name: string) => void;
  onEdit:     (profile: OptimizationProfile) => void;
  onDelete:   (name: string) => void;
}

function ProfileCard({ profile, onActivate, onEdit, onDelete }: ProfileCardProps) {
  return (
    <div className={`profile-card ${profile.active ? "profile-card-active" : ""}`}>
      {/* Active indicator */}
      {profile.active && (
        <div className="profile-active-badge">ACTIVE</div>
      )}

      <div className="profile-card-body">
        <div className="profile-name">{profile.name}</div>
        <div className="profile-desc">{profile.description}</div>

        {/* Threshold tags */}
        <div className="profile-tags">
          <span className="profile-tag">CPU &gt; {profile.cpuThreshold}%</span>
          <span className="profile-tag">RAM &gt; {profile.ramThreshold}%</span>
          {profile.restrictBackground && (
            <span className="profile-tag profile-tag-warn">BG Restrict</span>
          )}
          {profile.batterySaver && (
            <span className="profile-tag profile-tag-info">Battery Saver</span>
          )}
        </div>

        {/* Target apps */}
        {profile.targetApps.length > 0 && (
          <div className="profile-apps">
            Triggers on: {profile.targetApps.join(", ")}
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="profile-card-actions">
        {profile.active ? (
          <button
            className="btn-secondary"
            onClick={() => onActivate("")}   // empty string = deactivate
          >
            Deactivate
          </button>
        ) : (
          <button
            className="btn-primary"
            onClick={() => onActivate(profile.name)}
          >
            ▶ Activate
          </button>
        )}
        <button className="btn-secondary" onClick={() => onEdit(profile)}>
          Edit
        </button>
        <button
          className="btn-danger"
          onClick={() => {
            if (window.confirm(`Delete profile "${profile.name}"?`)) {
              onDelete(profile.name);
            }
          }}
        >
          Delete
        </button>
      </div>
    </div>
  );
}

// ---- Profile form component ----

interface ProfileFormProps {
  initial:  OptimizationProfile;
  onSave:   (profile: OptimizationProfile) => void;
  onCancel: () => void;
  isNew:    boolean;
}

function ProfileForm({ initial, onSave, onCancel, isNew }: ProfileFormProps) {
  const [form, setForm] = useState<OptimizationProfile>(initial);
  const [appsInput, setAppsInput] = useState(initial.targetApps.join(", "));
  const [error, setError] = useState("");

  const update = <K extends keyof OptimizationProfile>(
    key: K,
    value: OptimizationProfile[K]
  ) => setForm(f => ({ ...f, [key]: value }));

  const handleSave = () => {
    if (!form.name.trim()) {
      setError("Profile name is required.");
      return;
    }
    // Parse comma-separated app names into an array
    const apps = appsInput
      .split(",")
      .map(s => s.trim().toLowerCase())
      .filter(Boolean);

    onSave({ ...form, targetApps: apps });
  };

  return (
    <div className="profile-form-overlay">
      <div className="profile-form">
        <div className="form-header">
          <h2>{isNew ? "New Profile" : `Edit: ${initial.name}`}</h2>
          <button className="icon-btn" onClick={onCancel}>✕</button>
        </div>

        {error && <div className="form-error">{error}</div>}

        <div className="form-grid">
          {/* Name */}
          <label className="form-label">Profile Name *</label>
          <input
            className="form-input"
            placeholder="e.g. Gaming, Development"
            value={form.name}
            disabled={!isNew}  // Name is the key — cannot rename existing profiles
            onChange={e => { update("name", e.target.value); setError(""); }}
          />

          {/* Description */}
          <label className="form-label">Description</label>
          <input
            className="form-input"
            placeholder="What does this profile do?"
            value={form.description}
            onChange={e => update("description", e.target.value)}
          />

          {/* Target apps */}
          <label className="form-label">Target Applications</label>
          <input
            className="form-input"
            placeholder="chrome, code, game (comma-separated, partial names)"
            value={appsInput}
            onChange={e => setAppsInput(e.target.value)}
          />
          <div className="form-hint">
            Enter partial process names separated by commas.
            "chrome" will match "chrome.exe".
          </div>

          {/* CPU threshold */}
          <label className="form-label">
            CPU Threshold: <strong>{form.cpuThreshold}%</strong>
          </label>
          <input
            className="form-range"
            type="range"
            min={40} max={95} step={5}
            value={form.cpuThreshold}
            onChange={e => update("cpuThreshold", Number(e.target.value))}
          />

          {/* RAM threshold */}
          <label className="form-label">
            RAM Threshold: <strong>{form.ramThreshold}%</strong>
          </label>
          <input
            className="form-range"
            type="range"
            min={50} max={95} step={5}
            value={form.ramThreshold}
            onChange={e => update("ramThreshold", Number(e.target.value))}
          />

          {/* Toggles */}
          <div className="form-toggles">
            <label className="toggle-label">
              <input
                type="checkbox"
                checked={form.restrictBackground}
                onChange={e => update("restrictBackground", e.target.checked)}
              />
              <span>Restrict background processes</span>
            </label>
            <label className="toggle-label">
              <input
                type="checkbox"
                checked={form.batterySaver}
                onChange={e => update("batterySaver", e.target.checked)}
              />
              <span>Battery saver mode</span>
            </label>
          </div>
        </div>

        <div className="form-actions">
          <button className="btn-primary" onClick={handleSave}>
            {isNew ? "Create Profile" : "Save Changes"}
          </button>
          <button className="btn-secondary" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

// ---- Optimizer toggle component ----

interface OptimizerToggleProps {
  enabled:   boolean;
  onChange:  (val: boolean) => void;
}

function OptimizerToggle({ enabled, onChange }: OptimizerToggleProps) {
  return (
    <div className={`optimizer-toggle ${enabled ? "toggle-on" : "toggle-off"}`}>
      <div className="toggle-info">
        <div className="toggle-title">Optimization Engine</div>
        <div className="toggle-desc">
          {enabled
            ? "Active — rules are being evaluated every 2 seconds"
            : "Inactive — no automatic optimizations are applied"}
        </div>
      </div>
      <button
        className={`toggle-btn ${enabled ? "toggle-btn-on" : "toggle-btn-off"}`}
        onClick={() => onChange(!enabled)}
      >
        {enabled ? "ON" : "OFF"}
      </button>
    </div>
  );
}

// ---- Main ProfilesPanel component ----

interface ProfilesPanelProps {
  optimEnabled:         boolean;
  onOptimEnabledChange: (val: boolean) => void;
}

export default function ProfilesPanel({
  optimEnabled,
  onOptimEnabledChange,
}: ProfilesPanelProps) {
  const [profiles,     setProfiles]     = useState<OptimizationProfile[]>([]);
  const [loading,      setLoading]      = useState(true);
  const [showForm,     setShowForm]     = useState(false);
  const [editProfile,  setEditProfile]  = useState<OptimizationProfile | null>(null);
  const [feedback,     setFeedback]     = useState("");

  const loadProfiles = useCallback(async () => {
    try {
      const list = await invoke<OptimizationProfile[]>("load_profiles");
      setProfiles(list);
      setLoading(false);
    } catch (err) {
      console.error("Failed to load profiles:", err);
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadProfiles(); }, [loadProfiles]);

  const showFeedback = (msg: string) => {
    setFeedback(msg);
    setTimeout(() => setFeedback(""), 3000);
  };

  const handleSave = async (profile: OptimizationProfile) => {
    try {
      await invoke("save_profile", { profile });
      setShowForm(false);
      setEditProfile(null);
      await loadProfiles();
      showFeedback(`Profile "${profile.name}" saved.`);
    } catch (err) {
      showFeedback(`Error: ${err}`);
    }
  };

  const handleActivate = async (name: string) => {
    try {
      if (name === "") {
        await invoke("deactivate_profiles");
        showFeedback("All profiles deactivated.");
      } else {
        await invoke("activate_profile", { name });
        showFeedback(`Profile "${name}" activated.`);
      }
      await loadProfiles();
    } catch (err) {
      showFeedback(`Error: ${err}`);
    }
  };

  const handleDelete = async (name: string) => {
    try {
      await invoke("delete_profile", { name });
      await loadProfiles();
      showFeedback(`Profile "${name}" deleted.`);
    } catch (err) {
      showFeedback(`Error: ${err}`);
    }
  };

  const handleOptimizerToggle = async (enabled: boolean) => {
    try {
      await invoke("set_optimizer_enabled", { enabled });
      onOptimEnabledChange(enabled);
      showFeedback(
        enabled
          ? "Optimization engine enabled."
          : "Optimization engine disabled."
      );
    } catch (err) {
      showFeedback(`Error: ${err}`);
    }
  };

  if (loading) {
    return (
      <div className="dashboard-loading">
        <div className="loading-spinner" />
        <p>Loading profiles...</p>
      </div>
    );
  }

  return (
    <div className="profiles-panel">

      {/* Optimizer engine toggle */}
      <OptimizerToggle
        enabled={optimEnabled}
        onChange={handleOptimizerToggle}
      />

      {/* Panel header */}
      <div className="profiles-header">
        <div>
          <h2 className="profiles-title">Optimization Profiles</h2>
          <p className="profiles-subtitle">
            Profiles define how SISU optimizes your system.
            Activate one to apply its rules automatically.
          </p>
        </div>
        <button
          className="btn-primary"
          onClick={() => { setEditProfile(null); setShowForm(true); }}
        >
          + New Profile
        </button>
      </div>

      {/* Feedback message */}
      {feedback && (
        <div className="profiles-feedback">{feedback}</div>
      )}

      {/* Profile grid */}
      <div className="profiles-grid">
        {profiles.map(p => (
          <ProfileCard
            key={p.name}
            profile={p}
            onActivate={handleActivate}
            onEdit={profile => { setEditProfile(profile); setShowForm(true); }}
            onDelete={handleDelete}
          />
        ))}
      </div>

      {/* Create / Edit form modal */}
      {showForm && (
        <ProfileForm
          initial={editProfile ?? EMPTY_PROFILE}
          isNew={editProfile === null}
          onSave={handleSave}
          onCancel={() => { setShowForm(false); setEditProfile(null); }}
        />
      )}
    </div>
  );
}
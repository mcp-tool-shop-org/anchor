import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectHealth, RecoveryAction } from "../types";

const ACTION_ICONS: Record<string, string> = {
  edit_content: "✏️",
  transition_state: "→",
  add_trace_link: "🔗",
  revalidate: "✓",
  reapprove: "✓✓",
  reconcile_stale: "♻️",
  propose_amendment: "📋",
};

const STATUS_COLORS: Record<string, string> = {
  healthy: "var(--green, #22c55e)",
  needs_attention: "var(--yellow, #eab308)",
  critical: "var(--red, #ef4444)",
};

export function ProjectHealthView({
  onNavigate,
}: {
  onNavigate: (view: string, artifactId?: string) => void;
}) {
  const [health, setHealth] = useState<ProjectHealth | null>(null);

  useEffect(() => {
    refresh();
  }, []);

  async function refresh() {
    const h = await invoke<ProjectHealth>("get_project_health");
    setHealth(h);
  }

  if (!health) return <div className="empty">Loading project health...</div>;

  const statusColor = STATUS_COLORS[health.status] ?? "var(--text-dim)";
  const statusLabel = health.status === "healthy" ? "HEALTHY" : health.status === "critical" ? "CRITICAL" : "NEEDS ATTENTION";

  // Group actions by target artifact
  const byArtifact = new Map<string, RecoveryAction[]>();
  for (const action of health.nextActions) {
    const key = action.targetArtifactId;
    if (!byArtifact.has(key)) byArtifact.set(key, []);
    byArtifact.get(key)!.push(action);
  }

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <h1>Project Health</h1>
        <button className="btn btn-ghost" onClick={refresh}>↻ Refresh</button>
      </div>

      {/* Status Banner */}
      <div
        style={{
          padding: "16px 20px",
          borderRadius: 6,
          background: health.status === "healthy" ? "#052e16" : health.status === "critical" ? "#2a0a0a" : "#2a200a",
          border: `1px solid ${statusColor}`,
          marginBottom: 20,
        }}
      >
        <div style={{ fontSize: 16, fontWeight: 700, color: statusColor }}>
          {statusLabel}
        </div>
        <div style={{ marginTop: 6, fontSize: 12, color: "var(--text-dim)" }}>
          {health.summary}
        </div>
      </div>

      {/* Metrics */}
      <div style={{ display: "flex", gap: 24, marginBottom: 24, fontSize: 12, flexWrap: "wrap" }}>
        <Metric label="Total" value={health.totalArtifacts} />
        <Metric label="Approved" value={health.readyArtifacts} color="var(--green)" />
        <Metric label="Stale" value={health.staleArtifacts} color={health.staleArtifacts > 0 ? "var(--red)" : undefined} />
        <Metric label="Blocked" value={health.blockedArtifacts} color={health.blockedArtifacts > 0 ? "var(--orange)" : undefined} />
        <Metric label="Alarms" value={health.activeAlarms} color={health.activeAlarms > 0 ? "var(--red)" : undefined} />
        <Metric label="Missing Links" value={health.missingLinks} color={health.missingLinks > 0 ? "var(--yellow)" : undefined} />
      </div>

      {/* Next Actions */}
      {health.nextActions.length > 0 ? (
        <>
          <h2>Recovery Actions ({health.nextActions.length})</h2>
          <p style={{ fontSize: 12, color: "var(--text-dim)", marginBottom: 12 }}>
            These are the next lawful actions to reach export readiness. Ordered by priority.
          </p>
          {[...byArtifact.entries()].map(([_artifactId, actions]) => (
            <div key={actions[0].targetArtifactId} style={{ marginBottom: 16 }}>
              <div
                style={{
                  fontSize: 13,
                  fontWeight: 600,
                  marginBottom: 6,
                  color: "var(--accent)",
                  cursor: "pointer",
                }}
                onClick={() => onNavigate("detail", actions[0].targetArtifactId)}
              >
                {actions[0].targetArtifactTitle}
              </div>
              {actions.map((action, i) => (
                <ActionCard
                  key={i}
                  action={action}
                  onExecute={() => onNavigate("detail", action.targetArtifactId)}
                />
              ))}
            </div>
          ))}
        </>
      ) : (
        <div style={{ padding: 20, textAlign: "center", color: "var(--green)", fontSize: 14 }}>
          All clear. No recovery actions needed.
        </div>
      )}

      {/* Quick Links */}
      <div style={{ marginTop: 24, display: "flex", gap: 8, flexWrap: "wrap" }}>
        <button className="btn btn-ghost" onClick={() => onNavigate("gate")}>
          Readiness Gate
        </button>
        <button className="btn btn-ghost" onClick={() => onNavigate("links")}>
          Link Authoring
        </button>
        <button className="btn btn-ghost" onClick={() => onNavigate("timeline")}>
          Audit Timeline
        </button>
      </div>
    </div>
  );
}

function Metric({ label, value, color }: { label: string; value: number; color?: string }) {
  return (
    <div>
      <span style={{ color: "var(--text-dim)" }}>{label}: </span>
      <span style={{ color: color ?? "var(--text)", fontWeight: 600 }}>{value}</span>
    </div>
  );
}

function ActionCard({ action, onExecute }: { action: RecoveryAction; onExecute: () => void }) {
  const icon = ACTION_ICONS[action.actionType] ?? "•";
  const priorityColor =
    action.priority === 1 ? "var(--red)" : action.priority === 2 ? "var(--orange)" : "var(--text-dim)";

  return (
    <div
      className="blocker-card"
      style={{ cursor: "pointer", marginBottom: 4 }}
      onClick={onExecute}
    >
      <div style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
        <span style={{ fontSize: 14 }}>{icon}</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 12, fontWeight: 600 }}>{action.title}</div>
          <div style={{ fontSize: 11, color: "var(--text-dim)", marginTop: 2 }}>
            {action.description}
          </div>
        </div>
        <div
          style={{
            fontSize: 10,
            padding: "2px 6px",
            borderRadius: 3,
            background: "rgba(255,255,255,0.05)",
            color: priorityColor,
            whiteSpace: "nowrap",
          }}
        >
          P{action.priority}
        </div>
      </div>
    </div>
  );
}

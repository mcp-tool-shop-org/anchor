import type { ArtifactRow } from "../types";

const TYPE_LABELS: Record<string, string> = {
  constitution: "Constitution",
  user_fantasy_workflows: "Workflows",
  feature_map: "Features",
  system_architecture: "System Arch",
  ux_state_map: "UX State Map",
  phase_roadmap_contracts: "Phase Roadmap",
  acceptance_checklists: "Checklists",
  drift_alarm_definitions: "Drift Alarms",
  execution_readiness_gate: "Gate",
};

export function ArtifactIndex({
  artifacts,
  onSelect,
}: {
  artifacts: ArtifactRow[];
  onSelect: (id: string) => void;
}) {
  return (
    <div>
      <h1>Artifact Index</h1>
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>Artifact</th>
            <th>State</th>
            <th>Version</th>
            <th>Approved</th>
            <th>Links ↑↓</th>
            <th>Alarms</th>
          </tr>
        </thead>
        <tbody>
          {artifacts.map((a, i) => (
            <tr
              key={a.id}
              className="clickable"
              onClick={() => onSelect(a.id)}
            >
              <td style={{ color: "var(--text-dim)" }}>{i + 1}</td>
              <td>
                <div>{a.title}</div>
                <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
                  {TYPE_LABELS[a.artifactType] ?? a.artifactType}
                </div>
              </td>
              <td>
                <span className={`badge ${a.state}`}>{a.state}</span>
              </td>
              <td>v{a.versionNumber}</td>
              <td>{a.hasApproval ? "✓" : "—"}</td>
              <td>
                {a.upstreamCount}↑ {a.downstreamCount}↓
              </td>
              <td>
                {a.alarmCount > 0 ? (
                  <span style={{ color: "var(--red)" }}>{a.alarmCount}</span>
                ) : (
                  <span style={{ color: "var(--text-dim)" }}>0</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { GateEvaluation } from "../types";

export function ReadinessGate() {
  const [gate, setGate] = useState<GateEvaluation | null>(null);

  useEffect(() => {
    invoke<GateEvaluation>("get_readiness_gate").then(setGate);
  }, []);

  async function refresh() {
    const g = await invoke<GateEvaluation>("get_readiness_gate");
    setGate(g);
  }

  if (!gate) return <div className="empty">Loading gate evaluation...</div>;

  const ready = gate.status === "ready";

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <h1>Execution Readiness Gate</h1>
        <button className="btn btn-ghost" onClick={refresh}>↻ Refresh</button>
      </div>

      <div
        style={{
          padding: "16px 20px",
          borderRadius: 6,
          background: ready ? "#052e16" : "#2a0a0a",
          border: `1px solid ${ready ? "var(--green)" : "var(--red)"}`,
          marginBottom: 20,
        }}
      >
        <div style={{ fontSize: 16, fontWeight: 700, color: ready ? "var(--green)" : "var(--red)" }}>
          {ready ? "READY FOR EXPORT" : "BLOCKED"}
        </div>
        <div style={{ marginTop: 6, fontSize: 12, color: "var(--text-dim)" }}>
          {gate.readinessSummary}
        </div>
      </div>

      {/* Stats Row */}
      <div style={{ display: "flex", gap: 24, marginBottom: 20, fontSize: 12 }}>
        <div>
          <span style={{ color: "var(--text-dim)" }}>Blocking reasons: </span>
          <span style={{ color: gate.blockingReasons.length > 0 ? "var(--red)" : "var(--green)" }}>
            {gate.blockingReasons.length}
          </span>
        </div>
        <div>
          <span style={{ color: "var(--text-dim)" }}>Stale artifacts: </span>
          <span style={{ color: gate.staleSummary.count > 0 ? "var(--orange)" : "var(--green)" }}>
            {gate.staleSummary.count}
          </span>
        </div>
        <div>
          <span style={{ color: "var(--text-dim)" }}>Outdated approvals: </span>
          <span style={{ color: gate.outdatedApprovals.length > 0 ? "var(--yellow)" : "var(--green)" }}>
            {gate.outdatedApprovals.length}
          </span>
        </div>
        <div>
          <span style={{ color: "var(--text-dim)" }}>Active blocking alarms: </span>
          <span style={{ color: gate.activeBlockingAlarms.length > 0 ? "var(--red)" : "var(--green)" }}>
            {gate.activeBlockingAlarms.length}
          </span>
        </div>
        <div>
          <span style={{ color: "var(--text-dim)" }}>Traceability failures: </span>
          <span style={{ color: gate.traceabilityFailures > 0 ? "var(--red)" : "var(--green)" }}>
            {gate.traceabilityFailures}
          </span>
        </div>
      </div>

      {/* Blocking Reasons */}
      {gate.blockingReasons.length > 0 && (
        <>
          <h2>Why Blocked?</h2>
          {gate.blockingReasons.map((r, i) => (
            <div key={i} className="blocker-card">
              <div className="code">{r.code}</div>
              <div className="message">{r.message}</div>
              <div className="provenance">
                {r.ruleProvenance.sourceClause} — {r.ruleProvenance.humanLabel}
              </div>
              <ol className="remediation">
                {r.remediationSteps.map((step, j) => (
                  <li key={j}>{step}</li>
                ))}
              </ol>
            </div>
          ))}
        </>
      )}

      {/* Stale Artifacts */}
      {gate.staleSummary.count > 0 && (
        <>
          <h2>Stale Artifacts</h2>
          <ul style={{ paddingLeft: 20, fontSize: 12 }}>
            {gate.staleSummary.artifactIds.map((id) => (
              <li key={id}>{id}</li>
            ))}
          </ul>
        </>
      )}

      {/* Outdated Approvals */}
      {gate.outdatedApprovals.length > 0 && (
        <>
          <h2>Outdated Approvals</h2>
          <table>
            <thead>
              <tr>
                <th>Artifact</th>
                <th>Approved Against</th>
                <th>Current</th>
              </tr>
            </thead>
            <tbody>
              {gate.outdatedApprovals.map((oa) => (
                <tr key={oa.approvalId}>
                  <td>{oa.artifactId}</td>
                  <td>{oa.approvedAgainstVersion}</td>
                  <td>{oa.currentConstitutionVersion}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

      {/* Export Manifest Preview */}
      <h2>Export Manifest ({gate.exportManifestPreview.fileCount} files)</h2>
      <table>
        <thead>
          <tr>
            <th>Path</th>
            <th>Kind</th>
          </tr>
        </thead>
        <tbody>
          {gate.exportManifestPreview.files.map((f) => (
            <tr key={f.path}>
              <td style={{ color: "var(--accent)" }}>{f.path}</td>
              <td>{f.kind}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

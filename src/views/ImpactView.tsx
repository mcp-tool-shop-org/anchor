import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ImpactReport, AffectedArtifact, RecoveryStep } from "../types";

export function ImpactView({
  artifactId,
  mode,
  onBack,
}: {
  artifactId?: string;
  mode: "edit" | "amendment";
  onBack: () => void;
}) {
  const [report, setReport] = useState<ImpactReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadImpact();
  }, [artifactId, mode]);

  async function loadImpact() {
    try {
      if (mode === "amendment") {
        const r = await invoke<ImpactReport>("get_amendment_impact");
        setReport(r);
      } else if (artifactId) {
        const r = await invoke<ImpactReport>("get_edit_impact", { artifactId });
        setReport(r);
      }
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  if (error) return <div style={{ color: "var(--red)" }}>{error}</div>;
  if (!report) return <div className="empty">Loading impact analysis...</div>;

  const severityColor: Record<string, string> = {
    none: "var(--green)",
    low: "var(--text)",
    medium: "var(--orange)",
    high: "var(--red)",
    nuclear: "#ff0044",
  };

  return (
    <div>
      <button className="btn btn-ghost" onClick={onBack} style={{ marginBottom: 12 }}>
        ← Back
      </button>

      <h1>Blast Radius</h1>

      {/* Trigger */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
          {report.trigger.description}
        </div>
      </div>

      {/* Severity badge */}
      <div
        style={{
          display: "inline-block",
          padding: "6px 16px",
          borderRadius: 3,
          fontWeight: 700,
          fontSize: 13,
          background: severityColor[report.severity] ?? "var(--text-dim)",
          color: report.severity === "none" ? "#000" : "#fff",
          marginBottom: 16,
        }}
      >
        {report.severity.toUpperCase()} — {report.totalAffected} artifact(s), {report.totalApprovalsLost} approval(s) lost
      </div>

      {/* Affected Artifacts */}
      <h2>Affected Artifacts ({report.affectedArtifacts.length})</h2>
      {report.affectedArtifacts.length === 0 ? (
        <div className="empty">No artifacts affected by this change.</div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {report.affectedArtifacts.map((aa) => (
            <AffectedCard key={aa.artifactId} artifact={aa} />
          ))}
        </div>
      )}

      {/* Invalidated Approvals */}
      {report.invalidatedApprovals.length > 0 && (
        <>
          <h2>Invalidated Approvals ({report.invalidatedApprovals.length})</h2>
          {report.invalidatedApprovals.map((ia) => (
            <div
              key={ia.approvalId}
              style={{
                padding: "8px 12px",
                borderLeft: "3px solid var(--red)",
                background: "var(--bg-card, #1a1a2e)",
                marginBottom: 4,
                fontSize: 12,
              }}
            >
              <strong>{ia.artifactTitle}</strong> — {ia.reason}
            </div>
          ))}
        </>
      )}

      {/* Recovery Plan */}
      <h2>Recovery Plan ({report.recoveryPlan.length} steps)</h2>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {report.recoveryPlan.map((step) => (
          <RecoveryStepCard key={step.order} step={step} />
        ))}
      </div>
    </div>
  );
}

function AffectedCard({ artifact }: { artifact: AffectedArtifact }) {
  return (
    <div
      className="blocker-card"
      style={{ borderLeft: "3px solid var(--orange)" }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
        <span style={{ fontWeight: 600, fontSize: 13 }}>{artifact.title}</span>
        <span style={{ fontSize: 10, color: "var(--text-dim)" }}>
          {artifact.artifactType}
        </span>
        <span style={{ marginLeft: "auto", fontSize: 10 }}>
          depth: {artifact.distanceFromSource}
        </span>
      </div>
      <div style={{ fontSize: 12, marginBottom: 4 }}>
        <span className={`badge ${artifact.currentState}`}>{artifact.currentState}</span>
        {" → "}
        <span className={`badge ${artifact.willBecome}`}>{artifact.willBecome}</span>
      </div>
      <div style={{ fontSize: 11, color: "var(--text-dim)" }}>{artifact.reason}</div>
      {artifact.propagationPath.length > 1 && (
        <div style={{ fontSize: 10, color: "var(--text-dim)", marginTop: 4 }}>
          Path: {artifact.propagationPath.join(" → ")}
        </div>
      )}
    </div>
  );
}

function RecoveryStepCard({ step }: { step: RecoveryStep }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "30px 1fr",
        gap: 8,
        padding: "6px 0",
        borderBottom: "1px solid var(--border)",
        fontSize: 12,
      }}
    >
      <span
        style={{
          fontWeight: 700,
          color: "var(--text-dim)",
          textAlign: "center",
        }}
      >
        {step.order}
      </span>
      <div>
        <div>{step.action}</div>
        <div style={{ fontSize: 11, color: "var(--text-dim)" }}>{step.reason}</div>
      </div>
    </div>
  );
}

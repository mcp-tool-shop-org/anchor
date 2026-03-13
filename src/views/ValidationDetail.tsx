import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ValidationReport, ValidationCheck } from "../types";

export function ValidationDetail({
  artifactId,
  onBack,
}: {
  artifactId: string;
  onBack: () => void;
}) {
  const [report, setReport] = useState<ValidationReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadReport();
  }, [artifactId]);

  async function loadReport() {
    try {
      const r = await invoke<ValidationReport>("get_validation_report", {
        artifactId,
      });
      setReport(r);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  if (error) return <div style={{ color: "var(--red)" }}>{error}</div>;
  if (!report) return <div className="empty">Loading validation...</div>;

  const verdictColor =
    report.overallStatus === "all_clear"
      ? "var(--green)"
      : report.overallStatus === "has_warnings"
        ? "var(--orange)"
        : "var(--red)";

  const verdictLabel =
    report.overallStatus === "all_clear"
      ? "ALL CLEAR"
      : report.overallStatus === "has_warnings"
        ? "WARNINGS"
        : "BLOCKED";

  const layers = ["structural", "relational", "intent"] as const;

  return (
    <div>
      <button className="btn btn-ghost" onClick={onBack} style={{ marginBottom: 12 }}>
        ← Back
      </button>

      <h1>Validation Report</h1>
      <div style={{ marginBottom: 16 }}>
        <span style={{ fontSize: 11, color: "var(--text-dim)" }}>
          {report.artifactType} · {report.artifactState}
        </span>
      </div>

      {/* Verdict */}
      <div
        style={{
          display: "inline-block",
          padding: "6px 16px",
          borderRadius: 3,
          fontWeight: 700,
          fontSize: 13,
          background: verdictColor,
          color: report.overallStatus === "all_clear" ? "#000" : "#fff",
          marginBottom: 16,
        }}
      >
        {verdictLabel}
      </div>

      {report.resolutionSummary && (
        <div style={{ fontSize: 12, color: "var(--text-dim)", marginBottom: 16 }}>
          {report.resolutionSummary}
        </div>
      )}

      {/* Checks by layer */}
      {layers.map((layer) => {
        const layerChecks = report.checks.filter((c) => c.layer === layer);
        if (layerChecks.length === 0) return null;
        return (
          <div key={layer} style={{ marginBottom: 20 }}>
            <h2 style={{ textTransform: "capitalize" }}>{layer} Checks</h2>
            {layerChecks.map((check) => (
              <CheckCard key={check.checkId} check={check} />
            ))}
          </div>
        );
      })}
    </div>
  );
}

function CheckCard({ check }: { check: ValidationCheck }) {
  const statusIcon =
    check.status === "pass"
      ? "✓"
      : check.status === "fail"
        ? "✗"
        : check.status === "warning"
          ? "⚠"
          : "—";

  const statusColor =
    check.status === "pass"
      ? "var(--green)"
      : check.status === "fail"
        ? "var(--red)"
        : check.status === "warning"
          ? "var(--orange)"
          : "var(--text-dim)";

  return (
    <div
      className="blocker-card"
      style={{
        borderLeft: `3px solid ${statusColor}`,
        marginBottom: 8,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
        <span style={{ color: statusColor, fontWeight: 700, fontSize: 14 }}>
          {statusIcon}
        </span>
        <span style={{ fontWeight: 600, fontSize: 13 }}>{check.title}</span>
        <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-dim)" }}>
          {check.ruleClause}
        </span>
      </div>
      <div style={{ fontSize: 12, color: "var(--text)", marginBottom: 6 }}>
        {check.explanation}
      </div>
      {check.resolutionSteps.length > 0 && (
        <ol
          style={{
            fontSize: 11,
            color: "var(--text-dim)",
            paddingLeft: 20,
            margin: 0,
          }}
        >
          {check.resolutionSteps.map((step, i) => (
            <li key={i} style={{ marginBottom: 2 }}>
              {step}
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

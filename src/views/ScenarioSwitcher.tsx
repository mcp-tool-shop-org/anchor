import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ScenarioInfo,
  SwitchScenarioResponse,
  ImportDiagnostic,
  ImportWithRepairResponse,
} from "../types";

export function ScenarioSwitcher({ onSwitch }: { onSwitch: () => void }) {
  const [scenarios, setScenarios] = useState<ScenarioInfo[]>([]);
  const [importing, setImporting] = useState(false);
  const [diagnostic, setDiagnostic] = useState<ImportDiagnostic | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    loadScenarios();
  }, []);

  async function loadScenarios() {
    try {
      const s = await invoke<ScenarioInfo[]>("list_demo_scenarios");
      setScenarios(s);
    } catch {
      // ignore
    }
  }

  async function switchTo(id: string) {
    try {
      const res = await invoke<SwitchScenarioResponse>("switch_demo_scenario", {
        scenarioName: id,
      });
      if (res.success) {
        setMessage(`Loaded "${res.projectName}" (${res.artifactCount} artifacts)`);
        onSwitch();
      } else {
        setMessage(res.error ?? "Switch failed");
      }
    } catch (e) {
      setMessage(String(e));
    }
  }

  async function doDryRun() {
    const path = prompt("File path to analyze:");
    if (!path) return;
    setImporting(true);
    try {
      const diag = await invoke<ImportDiagnostic>("dry_run_import", { filePath: path });
      setDiagnostic(diag);
    } catch (e) {
      setMessage(String(e));
    }
    setImporting(false);
  }

  async function doRepairLoad() {
    const path = prompt("File path to load with repair:");
    if (!path) return;
    try {
      const res = await invoke<ImportWithRepairResponse>("load_project_with_repair", {
        filePath: path,
      });
      if (res.success) {
        const issueCount = res.issues.length;
        setMessage(
          `Loaded with ${issueCount} issue(s) repaired. Path: ${res.filePath}`
        );
        setDiagnostic(null);
        onSwitch();
      } else {
        setMessage(res.error ?? "Repair load failed");
      }
    } catch (e) {
      setMessage(String(e));
    }
  }

  return (
    <div>
      <h1>Demo Scenarios</h1>
      <p style={{ fontSize: 12, color: "var(--text-dim)", marginBottom: 16 }}>
        Switch between pre-built project states to explore different governance situations.
      </p>

      {message && (
        <div
          style={{
            padding: "8px 12px",
            background: "var(--bg-card, #1a1a2e)",
            borderRadius: 3,
            marginBottom: 12,
            fontSize: 12,
          }}
        >
          {message}
        </div>
      )}

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 24 }}>
        {scenarios.map((s) => (
          <div
            key={s.id}
            className="blocker-card"
            style={{ cursor: "pointer" }}
            onClick={() => switchTo(s.id)}
          >
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 4 }}>
              {s.name}
            </div>
            <div style={{ fontSize: 12, marginBottom: 4 }}>{s.description}</div>
            <div style={{ fontSize: 11, color: "var(--text-dim)", fontStyle: "italic" }}>
              {s.flavor}
            </div>
          </div>
        ))}
      </div>

      {/* Import Tools */}
      <h2>Import Tools</h2>
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <button className="btn btn-ghost" onClick={doDryRun} disabled={importing}>
          Dry-Run Import
        </button>
        <button className="btn btn-ghost" onClick={doRepairLoad}>
          Load with Repair
        </button>
      </div>

      {/* Diagnostic Result */}
      {diagnostic && (
        <div style={{ marginBottom: 16 }}>
          <h3>Import Diagnostic</h3>

          <div
            style={{
              display: "inline-block",
              padding: "4px 12px",
              borderRadius: 3,
              fontWeight: 700,
              fontSize: 12,
              background: diagnostic.loadable ? "var(--green)" : "var(--red)",
              color: diagnostic.loadable ? "#000" : "#fff",
              marginBottom: 8,
            }}
          >
            {diagnostic.loadable ? "LOADABLE" : "NOT LOADABLE"}
            {diagnostic.repairable && !diagnostic.loadable && " (REPAIRABLE)"}
          </div>

          {diagnostic.summary && (
            <div className="meta-grid" style={{ marginBottom: 12 }}>
              <span className="label">Project</span>
              <span>{diagnostic.summary.projectName}</span>
              <span className="label">File Version</span>
              <span>{diagnostic.summary.fileVersion}</span>
              <span className="label">Schema Version</span>
              <span>{diagnostic.summary.schemaVersion}</span>
              <span className="label">Artifacts</span>
              <span>{diagnostic.summary.artifactCount}</span>
              <span className="label">Versions</span>
              <span>{diagnostic.summary.versionCount}</span>
              <span className="label">Links</span>
              <span>{diagnostic.summary.linkCount}</span>
            </div>
          )}

          {diagnostic.issues.length > 0 && (
            <>
              <h3>Issues ({diagnostic.issues.length})</h3>
              {diagnostic.issues.map((issue, i) => (
                <div
                  key={i}
                  className="blocker-card"
                  style={{
                    borderLeft: `3px solid ${
                      issue.severity === "fatal"
                        ? "var(--red)"
                        : issue.severity === "error"
                          ? "var(--orange)"
                          : "var(--text-dim)"
                    }`,
                    marginBottom: 4,
                  }}
                >
                  <div style={{ fontSize: 10, color: "var(--text-dim)", marginBottom: 2 }}>
                    {issue.severity.toUpperCase()} — {issue.code}
                  </div>
                  <div style={{ fontSize: 12 }}>{issue.message}</div>
                  {issue.detail && (
                    <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
                      {issue.detail}
                    </div>
                  )}
                </div>
              ))}
            </>
          )}

          {diagnostic.repairDescription && (
            <div style={{ fontSize: 12, marginTop: 8, color: "var(--text-dim)" }}>
              <strong>Repair available:</strong> {diagnostic.repairDescription}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

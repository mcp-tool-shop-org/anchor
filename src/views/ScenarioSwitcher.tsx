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
  const [expandedStory, setExpandedStory] = useState<string | null>(null);

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
        {scenarios.map((s) => {
          const story = SCENARIO_STORIES[s.id];
          const isExpanded = expandedStory === s.id;
          return (
            <div key={s.id} className="blocker-card">
              <div
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
              {story && (
                <>
                  <button
                    className="btn btn-ghost"
                    style={{ fontSize: 10, marginTop: 8, padding: "2px 8px" }}
                    onClick={(e) => { e.stopPropagation(); setExpandedStory(isExpanded ? null : s.id); }}
                  >
                    {isExpanded ? "Hide Story" : "Show Story"}
                  </button>
                  {isExpanded && (
                    <div style={{ marginTop: 8, padding: "8px 12px", background: "rgba(255,255,255,0.03)", borderRadius: 4, fontSize: 11 }}>
                      <div style={{ fontWeight: 600, marginBottom: 4, color: "var(--accent)" }}>
                        The Situation
                      </div>
                      <div style={{ color: "var(--text-dim)", marginBottom: 8 }}>{story.situation}</div>

                      <div style={{ fontWeight: 600, marginBottom: 4, color: "var(--orange)" }}>
                        What's Wrong
                      </div>
                      <div style={{ color: "var(--text-dim)", marginBottom: 8 }}>{story.diagnosis}</div>

                      <div style={{ fontWeight: 600, marginBottom: 4, color: "var(--green)" }}>
                        Recovery Path
                      </div>
                      <ol style={{ paddingLeft: 20, color: "var(--text-dim)", marginBottom: 8 }}>
                        {story.recovery.map((step, i) => (
                          <li key={i} style={{ marginBottom: 2 }}>{step}</li>
                        ))}
                      </ol>

                      <div style={{ fontWeight: 600, marginBottom: 4, color: "var(--yellow)" }}>
                        Teaching Point
                      </div>
                      <div style={{ color: "var(--text-dim)" }}>{story.teaching}</div>
                    </div>
                  )}
                </>
              )}
            </div>
          );
        })}
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

// ─── Scenario Stories ───────────────────────────────────────

interface ScenarioStory {
  situation: string;
  diagnosis: string;
  recovery: string[];
  teaching: string;
}

const SCENARIO_STORIES: Record<string, ScenarioStory> = {
  "forge-quest": {
    situation:
      "Forge Quest is mid-flight. The constitution is locked. Some artifacts are approved, others are still in draft or partway through validation. The gate is blocked.",
    diagnosis:
      "Several artifacts haven't reached Approved state. Some are missing trace links. The readiness gate correctly identifies these gaps and blocks export.",
    recovery: [
      "Open Project Health to see all recovery actions",
      "Complete draft artifacts with content",
      "Add missing trace links via Link Authoring",
      "Advance each artifact through Complete → Valid → Approved",
      "Check the Readiness Gate — it should turn green",
    ],
    teaching:
      "This is the normal workflow. Anchor doesn't rush you — it shows you exactly what's left and in what order. The gate is a checkpoint, not a punishment.",
  },
  "crystal-sanctum": {
    situation:
      "Crystal Sanctum is a finished project. Every artifact is Approved. All trace links are present. The gate is green and ready for export.",
    diagnosis: "Nothing is wrong. This is what success looks like.",
    recovery: [
      "Examine the Readiness Gate — notice everything is green",
      "Open the Export Panel — the manifest is complete",
      "Try the Traceability Graph — every node is connected",
      "Check Project Health — zero recovery actions",
    ],
    teaching:
      "A healthy project is boring in the best way. Every artifact traceable, every approval current, every gate check passing. This is the standard Anchor holds you to.",
  },
  "shadow-protocol": {
    situation:
      "Shadow Protocol was built fast and loose. Artifacts exist but the traceability graph has gaps. Required links are missing. Drift alarms are firing.",
    diagnosis:
      "Missing required trace links mean the system can't verify that artifacts are justified by the constitution. Drift alarms indicate structural violations.",
    recovery: [
      "Open Link Authoring — see all missing required links",
      "Add the suggested links (one click each)",
      "Open Project Health — watch the missing link count drop",
      "Resolve any remaining drift alarms",
      "Validate and approve stalled artifacts",
    ],
    teaching:
      "Skipping traceability doesn't save time — it creates governance debt. Anchor makes the debt visible so you can pay it down systematically instead of discovering it at export time.",
  },
  "ember-saga": {
    situation:
      "Ember Saga had a constitutional amendment. The core promise changed. Stale propagation has cascaded through the entire project. Most artifacts are now Stale.",
    diagnosis:
      "A constitutional change invalidates every artifact that traces back to it. This is by design — Anchor forces you to re-examine everything when the foundation shifts.",
    recovery: [
      "Open Project Health — see the mass stale propagation",
      "Start with the constitution and work downstream",
      "For each stale artifact: review content, update if needed, revalidate",
      "Re-approve each artifact against the new constitution",
      "Use the Impact view to understand the blast radius",
    ],
    teaching:
      "Constitutional amendments are nuclear by design. If you change the promise, everything built on that promise must be re-examined. There's no shortcut — and that's the point.",
  },
};

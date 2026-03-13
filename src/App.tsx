import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectSnapshot, GateEvaluation } from "./types";
import { ArtifactIndex } from "./views/ArtifactIndex";
import { ArtifactDetail } from "./views/ArtifactDetail";
import { ReadinessGate } from "./views/ReadinessGate";
import { ExportPanel } from "./views/ExportPanel";
import { GraphView } from "./views/GraphView";

type View = "index" | "detail" | "gate" | "export" | "graph";

export default function App() {
  const [view, setView] = useState<View>("index");
  const [snapshot, setSnapshot] = useState<ProjectSnapshot | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [gate, setGate] = useState<GateEvaluation | null>(null);

  useEffect(() => {
    loadSnapshot();
    loadGate();
  }, []);

  async function loadSnapshot() {
    const snap = await invoke<ProjectSnapshot>("get_project_snapshot");
    setSnapshot(snap);
  }

  async function loadGate() {
    const g = await invoke<GateEvaluation>("get_readiness_gate");
    setGate(g);
  }

  function selectArtifact(id: string) {
    setSelectedId(id);
    setView("detail");
  }

  function refreshAll() {
    loadSnapshot();
    loadGate();
  }

  const navItems: { key: View; label: string }[] = [
    { key: "index", label: "Artifact Index" },
    { key: "gate", label: "Readiness Gate" },
    { key: "export", label: "Export Package" },
    { key: "graph", label: "Graph View" },
  ];

  return (
    <div className="shell">
      {/* ─── Sidebar ──────────────────────────────── */}
      <nav className="sidebar">
        <h2>Anchor</h2>
        {navItems.map((item) => (
          <button
            key={item.key}
            className={view === item.key ? "active" : ""}
            onClick={() => setView(item.key)}
          >
            {item.label}
          </button>
        ))}
        {snapshot && (
          <div
            style={{
              marginTop: "auto",
              padding: "12px 16px",
              fontSize: 11,
              color: "var(--text-dim)",
              borderTop: "1px solid var(--border)",
            }}
          >
            <div style={{ fontWeight: 600, marginBottom: 4 }}>
              {snapshot.project.name}
            </div>
            <div>{snapshot.artifacts.length} artifacts</div>
          </div>
        )}
      </nav>

      {/* ─── Center Pane ──────────────────────────── */}
      <main className="center">
        {view === "index" && snapshot && (
          <ArtifactIndex
            artifacts={snapshot.artifacts}
            onSelect={selectArtifact}
          />
        )}
        {view === "detail" && selectedId && (
          <ArtifactDetail
            artifactId={selectedId}
            onBack={() => setView("index")}
            onRefresh={refreshAll}
          />
        )}
        {view === "gate" && <ReadinessGate />}
        {view === "export" && <ExportPanel />}
        {view === "graph" && snapshot && (
          <GraphView
            artifacts={snapshot.artifacts}
            onSelect={selectArtifact}
          />
        )}
      </main>

      {/* ─── Inspector ────────────────────────────── */}
      <aside className="inspector">
        <h3>Constitution</h3>
        {snapshot && (
          <div style={{ fontSize: 12 }}>
            <div style={{ marginBottom: 8, color: "var(--text)" }}>
              {snapshot.project.name}
            </div>
            <div style={{ color: "var(--text-dim)", marginBottom: 12 }}>
              {snapshot.project.description}
            </div>
          </div>
        )}

        <h3>Gate Status</h3>
        {gate && (
          <div style={{ marginBottom: 16 }}>
            <span
              className={`gate-badge ${gate.status}`}
              style={{
                display: "inline-block",
                padding: "3px 10px",
                borderRadius: 3,
                fontWeight: 700,
                fontSize: 11,
                background:
                  gate.status === "ready" ? "var(--green)" : "var(--red)",
                color: gate.status === "ready" ? "#000" : "#fff",
              }}
            >
              {gate.status.toUpperCase()}
            </span>
            {gate.blockingReasons.length > 0 && (
              <div
                style={{
                  marginTop: 8,
                  fontSize: 11,
                  color: "var(--text-dim)",
                }}
              >
                {gate.blockingReasons.length} blocker(s)
                <ul style={{ paddingLeft: 16, marginTop: 4 }}>
                  {gate.blockingReasons.slice(0, 5).map((r, i) => (
                    <li key={i} style={{ marginBottom: 2 }}>
                      {r.code}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}

        <h3>Quick Stats</h3>
        {snapshot && (
          <div className="meta-grid" style={{ gridTemplateColumns: "auto 1fr" }}>
            <span className="label">Artifacts</span>
            <span>{snapshot.artifacts.length}</span>
            <span className="label">Stale</span>
            <span style={{ color: snapshot.staleCount > 0 ? "var(--orange)" : "var(--green)" }}>
              {snapshot.staleCount}
            </span>
            <span className="label">Alarms</span>
            <span style={{ color: snapshot.activeAlarmCount > 0 ? "var(--red)" : "var(--green)" }}>
              {snapshot.activeAlarmCount}
            </span>
          </div>
        )}
      </aside>

      {/* ─── Status Bar ───────────────────────────── */}
      <footer className="status-bar">
        {snapshot && (
          <>
            <span
              className={`gate-badge ${snapshot.gateStatus}`}
            >
              Gate: {snapshot.gateStatus.toUpperCase()}
            </span>
            <span>
              {snapshot.artifacts.filter((a) => a.state === "approved").length}/
              {snapshot.artifacts.length} approved
            </span>
            <span>Alarms: {snapshot.activeAlarmCount}</span>
            <span>Stale: {snapshot.staleCount}</span>
          </>
        )}
      </footer>
    </div>
  );
}

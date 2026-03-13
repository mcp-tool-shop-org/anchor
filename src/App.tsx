import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectSnapshot, GateEvaluation, SaveLoadResponse } from "./types";
import { ArtifactIndex } from "./views/ArtifactIndex";
import { ArtifactDetail } from "./views/ArtifactDetail";
import { ReadinessGate } from "./views/ReadinessGate";
import { ExportPanel } from "./views/ExportPanel";
import { GraphView } from "./views/GraphView";
import { AuditTimeline } from "./views/AuditTimeline";
import { AmendmentPanel } from "./views/AmendmentPanel";
import { ValidationDetail } from "./views/ValidationDetail";
import { ImpactView } from "./views/ImpactView";
import { ScenarioSwitcher } from "./views/ScenarioSwitcher";
import { CommandPalette } from "./views/CommandPalette";
import { ProjectHealthView } from "./views/ProjectHealthView";
import { LinkAuthoringView } from "./views/LinkAuthoringView";

type View = "index" | "detail" | "gate" | "export" | "graph" | "timeline" | "amend" | "validate" | "impact" | "scenarios" | "health" | "links";

export default function App() {
  const [view, setView] = useState<View>("index");
  const [snapshot, setSnapshot] = useState<ProjectSnapshot | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [gate, setGate] = useState<GateEvaluation | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    loadSnapshot();
    loadGate();
  }, []);

  // Global keyboard shortcut: Ctrl+K opens command palette
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setPaletteOpen((prev) => !prev);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
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

  function navigate(targetView: string, artifactId?: string) {
    if (artifactId) setSelectedId(artifactId);
    setView(targetView as View);
  }

  function refreshAll() {
    loadSnapshot();
    loadGate();
  }

  async function handleSave() {
    const path = prompt("Save project to:", "forge-quest.anchor.json");
    if (!path) return;
    const res = await invoke<SaveLoadResponse>("save_project", { filePath: path });
    if (res.success) {
      alert("Saved to " + res.filePath);
    } else {
      alert("Save failed: " + res.error);
    }
  }

  async function handleLoad() {
    const path = prompt("Load project from:");
    if (!path) return;
    const res = await invoke<SaveLoadResponse>("load_project", { filePath: path });
    if (res.success) {
      refreshAll();
      setView("index");
    } else {
      alert("Load failed: " + res.error);
    }
  }

  const navItems: { key: View; label: string }[] = [
    { key: "health", label: "Project Health" },
    { key: "index", label: "Artifact Index" },
    { key: "gate", label: "Readiness Gate" },
    { key: "links", label: "Link Authoring" },
    { key: "export", label: "Export Package" },
    { key: "amend", label: "Amendments" },
    { key: "timeline", label: "Audit Timeline" },
    { key: "graph", label: "Graph View" },
    { key: "scenarios", label: "Demo Scenarios" },
  ];

  return (
    <div className="shell">
      {/* ─── Sidebar ──────────────────────────────── */}
      <nav className="sidebar">
        <h2>Anchor</h2>
        <button
          style={{ fontSize: 10, padding: "2px 8px", marginBottom: 8, width: "100%", textAlign: "left", color: "var(--text-dim)" }}
          onClick={() => setPaletteOpen(true)}
        >
          Ctrl+K — Command Palette
        </button>
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
            <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
              <button
                onClick={handleSave}
                style={{ fontSize: 10, padding: "2px 8px" }}
              >
                Save
              </button>
              <button
                onClick={handleLoad}
                style={{ fontSize: 10, padding: "2px 8px" }}
              >
                Load
              </button>
            </div>
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
            onValidate={(id) => { setSelectedId(id); setView("validate"); }}
            onImpact={(id) => { setSelectedId(id); setView("impact"); }}
          />
        )}
        {view === "gate" && <ReadinessGate />}
        {view === "export" && <ExportPanel />}
        {view === "amend" && <AmendmentPanel onRefresh={refreshAll} />}
        {view === "timeline" && <AuditTimeline />}
        {view === "graph" && snapshot && (
          <GraphView
            artifacts={snapshot.artifacts}
            onSelect={selectArtifact}
          />
        )}
        {view === "validate" && selectedId && (
          <ValidationDetail
            artifactId={selectedId}
            onBack={() => setView("detail")}
          />
        )}
        {view === "impact" && selectedId && (
          <ImpactView
            artifactId={selectedId}
            mode="edit"
            onBack={() => setView("detail")}
          />
        )}
        {view === "scenarios" && (
          <ScenarioSwitcher
            onSwitch={() => { refreshAll(); setView("index"); }}
          />
        )}
        {view === "health" && (
          <ProjectHealthView onNavigate={navigate} />
        )}
        {view === "links" && (
          <LinkAuthoringView onNavigate={navigate} />
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

      {/* ─── Command Palette ──────────────────────── */}
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        snapshot={snapshot}
        onNavigate={navigate}
        onRefresh={refreshAll}
      />
    </div>
  );
}

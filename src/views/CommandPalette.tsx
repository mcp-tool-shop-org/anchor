import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectSnapshot, ScenarioInfo } from "../types";

interface PaletteCommand {
  id: string;
  label: string;
  category: "artifact" | "action" | "scenario" | "view";
  description?: string;
  action: () => void;
}

export function CommandPalette({
  open,
  onClose,
  snapshot,
  onNavigate,
  onRefresh,
}: {
  open: boolean;
  onClose: () => void;
  snapshot: ProjectSnapshot | null;
  onNavigate: (view: string, artifactId?: string) => void;
  onRefresh: () => void;
}) {
  const [query, setQuery] = useState("");
  const [commands, setCommands] = useState<PaletteCommand[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Build command list from current state
  useEffect(() => {
    if (!open) return;
    buildCommands();
  }, [open, snapshot]);

  // Focus input when opened
  useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.focus();
      setQuery("");
      setSelectedIndex(0);
    }
  }, [open]);

  async function buildCommands() {
    const cmds: PaletteCommand[] = [];

    // View navigation commands
    cmds.push({ id: "v-index", label: "Go to Artifact Index", category: "view", action: () => { onNavigate("index"); onClose(); } });
    cmds.push({ id: "v-gate", label: "Go to Readiness Gate", category: "view", action: () => { onNavigate("gate"); onClose(); } });
    cmds.push({ id: "v-export", label: "Go to Export Panel", category: "view", action: () => { onNavigate("export"); onClose(); } });
    cmds.push({ id: "v-graph", label: "Go to Traceability Graph", category: "view", action: () => { onNavigate("graph"); onClose(); } });
    cmds.push({ id: "v-timeline", label: "Go to Audit Timeline", category: "view", action: () => { onNavigate("timeline"); onClose(); } });
    cmds.push({ id: "v-amend", label: "Go to Amendments", category: "view", action: () => { onNavigate("amend"); onClose(); } });
    cmds.push({ id: "v-health", label: "Go to Project Health", category: "view", action: () => { onNavigate("health"); onClose(); } });
    cmds.push({ id: "v-links", label: "Go to Link Authoring", category: "view", action: () => { onNavigate("links"); onClose(); } });
    cmds.push({ id: "v-scenarios", label: "Go to Demo Scenarios", category: "view", action: () => { onNavigate("scenarios"); onClose(); } });

    // Artifact navigation
    if (snapshot) {
      for (const art of snapshot.artifacts) {
        cmds.push({
          id: `a-${art.id}`,
          label: art.title,
          category: "artifact",
          description: `${art.state} · v${art.versionNumber}`,
          action: () => { onNavigate("detail", art.id); onClose(); },
        });
      }
    }

    // Action commands
    cmds.push({
      id: "act-validate-all",
      label: "Validate All Artifacts",
      category: "action",
      description: "Run validation on every artifact",
      action: () => { onNavigate("health"); onClose(); },
    });
    cmds.push({
      id: "act-save",
      label: "Save Project",
      category: "action",
      action: async () => { await invoke("save_project", { filePath: "anchor-project.json" }); onRefresh(); onClose(); },
    });
    cmds.push({
      id: "act-refresh",
      label: "Refresh All Data",
      category: "action",
      action: () => { onRefresh(); onClose(); },
    });

    // Scenario switching
    try {
      const scenarios = await invoke<ScenarioInfo[]>("list_demo_scenarios");
      for (const s of scenarios) {
        cmds.push({
          id: `s-${s.id}`,
          label: `Switch to ${s.name}`,
          category: "scenario",
          description: s.description,
          action: async () => { await invoke("switch_demo_scenario", { scenarioName: s.id }); onRefresh(); onClose(); },
        });
      }
    } catch { /* scenarios unavailable */ }

    setCommands(cmds);
  }

  const filtered = query.trim()
    ? commands.filter((c) => {
        const q = query.toLowerCase();
        return c.label.toLowerCase().includes(q) || (c.description?.toLowerCase().includes(q));
      })
    : commands;

  // Reset selection on query change
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const handleKey = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && filtered[selectedIndex]) {
        e.preventDefault();
        filtered[selectedIndex].action();
      } else if (e.key === "Escape") {
        onClose();
      }
    },
    [filtered, selectedIndex, onClose],
  );

  if (!open) return null;

  const categoryLabel = (c: string) => {
    switch (c) {
      case "artifact": return "Artifacts";
      case "action": return "Actions";
      case "scenario": return "Scenarios";
      case "view": return "Views";
      default: return c;
    }
  };

  // Group by category
  const groups = new Map<string, typeof filtered>();
  for (const cmd of filtered) {
    if (!groups.has(cmd.category)) groups.set(cmd.category, []);
    groups.get(cmd.category)!.push(cmd);
  }

  let flatIndex = 0;

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 1000,
        background: "rgba(0,0,0,0.6)",
        display: "flex",
        justifyContent: "center",
        paddingTop: 80,
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: 520,
          maxHeight: "60vh",
          background: "var(--bg-surface, #1a1a2e)",
          border: "1px solid var(--border, #333)",
          borderRadius: 8,
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border, #333)" }}>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKey}
            placeholder="Type a command, artifact, or scenario..."
            style={{
              width: "100%",
              background: "transparent",
              border: "none",
              outline: "none",
              color: "var(--text, #e0e0e0)",
              fontSize: 14,
            }}
          />
        </div>
        <div style={{ overflowY: "auto", padding: "8px 0" }}>
          {filtered.length === 0 && (
            <div style={{ padding: "16px", textAlign: "center", color: "var(--text-dim, #888)", fontSize: 12 }}>
              No matching commands
            </div>
          )}
          {[...groups.entries()].map(([category, cmds]) => (
            <div key={category}>
              <div style={{ padding: "4px 16px", fontSize: 10, color: "var(--text-dim, #888)", textTransform: "uppercase", letterSpacing: 1 }}>
                {categoryLabel(category)}
              </div>
              {cmds.map((cmd) => {
                const idx = flatIndex++;
                return (
                  <div
                    key={cmd.id}
                    style={{
                      padding: "6px 16px",
                      cursor: "pointer",
                      background: idx === selectedIndex ? "rgba(255,255,255,0.08)" : "transparent",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                    }}
                    onClick={cmd.action}
                    onMouseEnter={() => setSelectedIndex(idx)}
                  >
                    <div>
                      <div style={{ fontSize: 13, color: "var(--text, #e0e0e0)" }}>{cmd.label}</div>
                      {cmd.description && (
                        <div style={{ fontSize: 11, color: "var(--text-dim, #888)" }}>{cmd.description}</div>
                      )}
                    </div>
                    <div style={{ fontSize: 10, color: "var(--text-dim, #888)", textTransform: "uppercase" }}>
                      {category}
                    </div>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

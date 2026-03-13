import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LinkSuggestion, AllowedLinks, AddLinkResult } from "../types";

const LINK_TYPE_LABELS: Record<string, string> = {
  justifies: "Justifies",
  derives_from: "Derives From",
  implements: "Implements",
  depends_on: "Depends On",
  validated_by: "Validated By",
  invalidated_by: "Invalidated By",
};

export function LinkAuthoringView({
  onNavigate,
}: {
  onNavigate: (view: string, artifactId?: string) => void;
}) {
  const [missing, setMissing] = useState<LinkSuggestion[]>([]);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    refresh();
  }, []);

  async function refresh() {
    const m = await invoke<LinkSuggestion[]>("get_missing_links");
    setMissing(m);
    setStatus(null);
  }

  async function quickAddLink(suggestion: LinkSuggestion) {
    if (suggestion.candidateTargets.length === 0) {
      setStatus("No valid targets available for this link.");
      return;
    }
    // Auto-pick the first candidate
    const target = suggestion.candidateTargets[0];
    const result = await invoke<AddLinkResult>("add_trace_link", {
      sourceId: suggestion.sourceArtifactId,
      targetId: target.artifactId,
      linkType: suggestion.suggestedLinkType,
      rationale: `Required link: ${suggestion.ruleDescription}`,
    });
    if (result.success) {
      setStatus(`Added ${LINK_TYPE_LABELS[suggestion.suggestedLinkType] ?? suggestion.suggestedLinkType} link from "${suggestion.sourceTitle}" to "${target.title}"`);
      await refresh();
    } else {
      setStatus(result.error ?? "Failed to add link");
    }
  }

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <h1>Link Authoring</h1>
        <button className="btn btn-ghost" onClick={refresh}>↻ Refresh</button>
      </div>

      {status && (
        <div style={{
          padding: "8px 12px",
          marginBottom: 16,
          borderRadius: 4,
          background: "rgba(255,255,255,0.05)",
          fontSize: 12,
          color: "var(--accent)",
        }}>
          {status}
        </div>
      )}

      {/* Missing Links — Suggestions */}
      {missing.length > 0 ? (
        <>
          <h2>Missing Required Links ({missing.length})</h2>
          <p style={{ fontSize: 12, color: "var(--text-dim)", marginBottom: 12 }}>
            These trace links are required by the traceability rules. Add them to pass validation and clear the readiness gate.
          </p>
          {missing.map((s, i) => (
            <div key={i} className="blocker-card" style={{ marginBottom: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 600 }}>
                    <span
                      style={{ color: "var(--accent)", cursor: "pointer" }}
                      onClick={() => onNavigate("detail", s.sourceArtifactId)}
                    >
                      {s.sourceTitle}
                    </span>
                    {" → "}
                    <span style={{ color: "var(--text-dim)" }}>
                      {LINK_TYPE_LABELS[s.suggestedLinkType] ?? s.suggestedLinkType}
                    </span>
                    {" → "}
                    <span style={{ color: "var(--text-dim)" }}>
                      {s.suggestedTargetType}
                    </span>
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-dim)", marginTop: 4 }}>
                    {s.ruleDescription}
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-dim)", marginTop: 2 }}>
                    Candidates: {s.candidateTargets.map((t) => t.title).join(", ") || "none"}
                  </div>
                </div>
                <button
                  className="btn"
                  style={{ fontSize: 11, whiteSpace: "nowrap" }}
                  onClick={() => quickAddLink(s)}
                  disabled={s.candidateTargets.length === 0}
                >
                  + Add Link
                </button>
              </div>
            </div>
          ))}
        </>
      ) : (
        <div style={{ padding: 20, textAlign: "center", color: "var(--green)", fontSize: 14, marginBottom: 20 }}>
          All required trace links are present.
        </div>
      )}

      {/* Inline Link Creator */}
      <InlineLinkCreator onAdded={refresh} />
    </div>
  );
}

/** Inline form for creating arbitrary trace links. */
function InlineLinkCreator({ onAdded }: { onAdded: () => void }) {
  const [artifactId, setArtifactId] = useState("");
  const [allowed, setAllowed] = useState<AllowedLinks | null>(null);
  const [selectedType, setSelectedType] = useState("");
  const [selectedTarget, setSelectedTarget] = useState("");
  const [rationale, setRationale] = useState("");
  const [result, setResult] = useState<string | null>(null);

  async function loadAllowed() {
    if (!artifactId.trim()) return;
    try {
      const a = await invoke<AllowedLinks>("get_allowed_links", { artifactId: artifactId.trim() });
      setAllowed(a);
      setResult(null);
    } catch (e) {
      setAllowed(null);
      setResult(String(e));
    }
  }

  async function doAdd() {
    if (!artifactId || !selectedType || !selectedTarget) return;
    const r = await invoke<AddLinkResult>("add_trace_link", {
      sourceId: artifactId.trim(),
      targetId: selectedTarget,
      linkType: selectedType,
      rationale: rationale || "Manually authored link",
    });
    if (r.success) {
      setResult("Link added successfully.");
      setSelectedType("");
      setSelectedTarget("");
      setRationale("");
      setAllowed(null);
      onAdded();
    } else {
      setResult(r.error ?? "Failed");
    }
  }

  const activeOption = allowed?.allowed.find((a) => a.linkType === selectedType);

  return (
    <div style={{ marginTop: 24 }}>
      <h2>Create Custom Link</h2>
      <div style={{ display: "flex", gap: 8, alignItems: "flex-end", marginBottom: 8, flexWrap: "wrap" }}>
        <div>
          <label style={{ fontSize: 11, color: "var(--text-dim)" }}>Source Artifact ID</label>
          <input
            type="text"
            value={artifactId}
            onChange={(e) => setArtifactId(e.target.value)}
            placeholder="art-..."
            style={{ display: "block", width: 180 }}
          />
        </div>
        <button className="btn btn-ghost" onClick={loadAllowed}>Load Options</button>
      </div>

      {allowed && (
        <div style={{ fontSize: 12, marginBottom: 8 }}>
          <div style={{ marginBottom: 8 }}>
            <label style={{ fontSize: 11, color: "var(--text-dim)" }}>Link Type</label>
            <select
              value={selectedType}
              onChange={(e) => { setSelectedType(e.target.value); setSelectedTarget(""); }}
              style={{ display: "block", width: 200 }}
            >
              <option value="">Select...</option>
              {allowed.allowed.map((opt) => (
                <option key={opt.linkType} value={opt.linkType}>
                  {LINK_TYPE_LABELS[opt.linkType] ?? opt.linkType}
                  {opt.required ? (opt.alreadySatisfied ? " ✓" : " (required)") : ""}
                </option>
              ))}
            </select>
          </div>

          {activeOption && (
            <div style={{ marginBottom: 8 }}>
              <label style={{ fontSize: 11, color: "var(--text-dim)" }}>Target</label>
              <select
                value={selectedTarget}
                onChange={(e) => setSelectedTarget(e.target.value)}
                style={{ display: "block", width: 300 }}
              >
                <option value="">Select target...</option>
                {activeOption.candidates.map((c) => (
                  <option key={c.artifactId} value={c.artifactId}>
                    {c.title} ({c.artifactType})
                  </option>
                ))}
              </select>
            </div>
          )}

          <div style={{ marginBottom: 8 }}>
            <label style={{ fontSize: 11, color: "var(--text-dim)" }}>Rationale</label>
            <input
              type="text"
              value={rationale}
              onChange={(e) => setRationale(e.target.value)}
              placeholder="Why this link exists..."
              style={{ display: "block", width: 300 }}
            />
          </div>

          <button className="btn" onClick={doAdd} disabled={!selectedType || !selectedTarget}>
            Create Link
          </button>
        </div>
      )}

      {result && (
        <div style={{ marginTop: 8, fontSize: 12, color: "var(--accent)" }}>{result}</div>
      )}
    </div>
  );
}

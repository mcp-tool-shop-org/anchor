import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ArtifactDetailResponse } from "../types";

/**
 * Graph view: focused on one artifact + one hop in/out.
 * Not a hairball. Start with a selected node and expand.
 */
export function GraphView({
  artifacts,
  onSelect,
}: {
  artifacts: { id: string; title: string; state: string; artifactType: string }[];
  onSelect: (id: string) => void;
}) {
  const [selectedId, setSelectedId] = useState<string | null>(
    artifacts.length > 0 ? artifacts[0].id : null
  );
  const [detail, setDetail] = useState<ArtifactDetailResponse | null>(null);

  useEffect(() => {
    if (selectedId) {
      invoke<ArtifactDetailResponse>("get_artifact_detail", {
        artifactId: selectedId,
      }).then(setDetail);
    }
  }, [selectedId]);

  // Collect neighbors from detail
  const neighbors = new Set<string>();
  if (detail) {
    for (const l of detail.outgoingLinks) neighbors.add(l.targetId);
    for (const l of detail.incomingLinks) neighbors.add(l.sourceId);
  }

  return (
    <div>
      <h1>Dependency Graph</h1>
      <p style={{ color: "var(--text-dim)", fontSize: 12, marginBottom: 16 }}>
        Select a node to see one-hop neighborhood. Click a neighbor to navigate.
      </p>

      {/* Node Selector */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 20 }}>
        {artifacts.map((a) => (
          <button
            key={a.id}
            className={`graph-node ${a.id === selectedId ? "selected" : ""}`}
            onClick={() => setSelectedId(a.id)}
            style={{
              borderColor: neighbors.has(a.id)
                ? "var(--yellow)"
                : a.id === selectedId
                  ? "var(--accent)"
                  : "var(--border)",
            }}
          >
            <span className={`badge ${a.state}`} style={{ marginRight: 6 }}>
              {a.state.charAt(0).toUpperCase()}
            </span>
            {a.title}
          </button>
        ))}
      </div>

      {/* Edges from selected */}
      {detail && (
        <>
          <h2>Outgoing from {detail.artifact.title}</h2>
          {detail.outgoingLinks.length === 0 ? (
            <div className="empty">No outgoing links</div>
          ) : (
            detail.outgoingLinks.map((l) => (
              <div key={l.id} className="link-row">
                <span style={{ color: "var(--accent)", cursor: "pointer" }} onClick={() => setSelectedId(l.targetId)}>
                  {l.targetTitle}
                </span>
                <span className="link-type">{l.linkType}</span>
                <span className="graph-edge">"{l.rationale}"</span>
              </div>
            ))
          )}

          <h2>Incoming to {detail.artifact.title}</h2>
          {detail.incomingLinks.length === 0 ? (
            <div className="empty">No incoming links</div>
          ) : (
            detail.incomingLinks.map((l) => (
              <div key={l.id} className="link-row">
                <span style={{ color: "var(--accent)", cursor: "pointer" }} onClick={() => setSelectedId(l.sourceId)}>
                  {l.sourceTitle}
                </span>
                <span className="link-type">{l.linkType}</span>
                <span className="graph-edge">"{l.rationale}"</span>
              </div>
            ))
          )}

          <div style={{ marginTop: 16 }}>
            <button
              className="btn btn-ghost"
              onClick={() => { if (selectedId) onSelect(selectedId); }}
            >
              Open detail →
            </button>
          </div>
        </>
      )}
    </div>
  );
}

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ExportPreviewResponse } from "../types";

export function ExportPanel() {
  const [preview, setPreview] = useState<ExportPreviewResponse | null>(null);

  useEffect(() => {
    loadPreview();
  }, []);

  async function loadPreview() {
    const p = await invoke<ExportPreviewResponse>("get_export_preview");
    setPreview(p);
  }

  if (!preview) return <div className="empty">Loading export preview...</div>;

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <h1>Export Package</h1>
        <button className="btn btn-ghost" onClick={loadPreview}>↻ Refresh</button>
      </div>

      {/* Status Banner */}
      <div
        style={{
          padding: "16px 20px",
          borderRadius: 6,
          background: preview.ready ? "#052e16" : "#2a0a0a",
          border: `1px solid ${preview.ready ? "var(--green)" : "var(--red)"}`,
          marginBottom: 20,
        }}
      >
        <div style={{ fontSize: 14, fontWeight: 700, color: preview.ready ? "var(--green)" : "var(--red)" }}>
          {preview.ready ? "EXPORT READY" : "EXPORT BLOCKED"}
        </div>
        {preview.blockedReason && (
          <div style={{ marginTop: 6, fontSize: 12, color: "var(--text-dim)" }}>
            {preview.blockedReason}
          </div>
        )}
      </div>

      {/* Blockers */}
      {preview.blockingReasons.length > 0 && (
        <>
          <h2>Why Blocked?</h2>
          {preview.blockingReasons.map((r, i) => (
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

      {/* File List */}
      {preview.files.length > 0 && (
        <>
          <h2>Package Contents ({preview.files.length} files)</h2>
          {preview.files.map((f) => (
            <div key={f.path} className="export-file">
              <span className="path">{f.path}</span>
              <span className="size">{formatBytes(f.sizeBytes)}</span>
            </div>
          ))}
        </>
      )}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

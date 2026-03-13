import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ArtifactDetailResponse, TransitionResponse } from "../types";

export function ArtifactDetail({
  artifactId,
  onBack,
  onRefresh,
}: {
  artifactId: string;
  onBack: () => void;
  onRefresh: () => void;
}) {
  const [detail, setDetail] = useState<ArtifactDetailResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadDetail();
  }, [artifactId]);

  async function loadDetail() {
    try {
      const d = await invoke<ArtifactDetailResponse>("get_artifact_detail", {
        artifactId,
      });
      setDetail(d);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function doTransition(targetState: string) {
    try {
      const res = await invoke<TransitionResponse>("transition_artifact", {
        artifactId,
        targetState,
      });
      if (res.success) {
        await loadDetail();
        onRefresh();
      } else {
        setError(res.error ?? "Transition failed");
      }
    } catch (e) {
      setError(String(e));
    }
  }

  async function doApprove() {
    try {
      const res = await invoke<TransitionResponse>("approve_artifact", {
        artifactId,
      });
      if (res.success) {
        await loadDetail();
        onRefresh();
      } else {
        setError(res.error ?? "Approval failed");
      }
    } catch (e) {
      setError(String(e));
    }
  }

  if (!detail) return <div className="empty">Loading...</div>;

  const { artifact, version, approval, outgoingLinks, incomingLinks, activeAlarms, legalTransitions } = detail;

  return (
    <div>
      <button className="btn btn-ghost" onClick={onBack} style={{ marginBottom: 12 }}>
        ← Back to index
      </button>

      <h1>{artifact.title}</h1>

      {error && (
        <div style={{ color: "var(--red)", marginBottom: 12 }}>{error}</div>
      )}

      {/* Metadata */}
      <h2>Metadata</h2>
      <div className="meta-grid">
        <span className="label">State</span>
        <span><span className={`badge ${artifact.state}`}>{artifact.state}</span></span>
        <span className="label">Type</span>
        <span>{artifact.artifactType}</span>
        <span className="label">Version</span>
        <span>{version ? `v${version.versionNumber} (${version.contentHash})` : "—"}</span>
        <span className="label">Constitution</span>
        <span>{version?.constitutionVersionId ?? "—"}</span>
        <span className="label">Validation</span>
        <span>
          S:{artifact.validationSummary.structural} R:{artifact.validationSummary.relational} I:{artifact.validationSummary.intent}
        </span>
        <span className="label">Stale Reason</span>
        <span>{artifact.staleReason ?? "—"}</span>
        <span className="label">Approval</span>
        <span>
          {approval
            ? `${approval.approver.displayName} (${approval.approvalType})`
            : "Not approved"}
        </span>
      </div>

      {/* Actions */}
      <h2>Actions</h2>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {legalTransitions.map((t) => (
          <button
            key={t}
            className="btn btn-ghost"
            onClick={() => doTransition(t)}
          >
            → {t}
          </button>
        ))}
        {artifact.state === "valid" && (
          <button className="btn btn-primary" onClick={doApprove}>
            Approve
          </button>
        )}
        {legalTransitions.length === 0 && artifact.state !== "valid" && (
          <span className="empty">No legal transitions from {artifact.state}</span>
        )}
      </div>

      {/* Outgoing Links */}
      <h2>Outgoing Trace Links ({outgoingLinks.length})</h2>
      {outgoingLinks.length === 0 ? (
        <div className="empty">No outgoing links</div>
      ) : (
        outgoingLinks.map((l) => (
          <div key={l.id} className="link-row">
            <span>{l.sourceTitle}</span>
            <span className="link-type">{l.linkType}</span>
            <span>→ {l.targetTitle}</span>
          </div>
        ))
      )}

      {/* Incoming Links */}
      <h2>Incoming Trace Links ({incomingLinks.length})</h2>
      {incomingLinks.length === 0 ? (
        <div className="empty">No incoming links</div>
      ) : (
        incomingLinks.map((l) => (
          <div key={l.id} className="link-row">
            <span>{l.sourceTitle}</span>
            <span className="link-type">{l.linkType}</span>
            <span>→ {l.targetTitle}</span>
          </div>
        ))
      )}

      {/* Active Alarms */}
      {activeAlarms.length > 0 && (
        <>
          <h2>Active Alarms ({activeAlarms.length})</h2>
          {activeAlarms.map((a) => (
            <div key={a.id} className="blocker-card">
              <div className="code">{a.alarmType} — {a.severity}</div>
              <div className="message">{a.explanation}</div>
              {a.remediationPath.length > 0 && (
                <ol className="remediation">
                  {a.remediationPath.map((step, i) => (
                    <li key={i}>{step}</li>
                  ))}
                </ol>
              )}
            </div>
          ))}
        </>
      )}
    </div>
  );
}

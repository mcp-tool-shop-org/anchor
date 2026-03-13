import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ArtifactDetailResponse,
  TransitionResponse,
  EditResponse,
  AuditTimelineResponse,
  AuditEventRow,
  VersionDiff,
} from "../types";

export function ArtifactDetail({
  artifactId,
  onBack,
  onRefresh,
  onValidate,
  onImpact,
}: {
  artifactId: string;
  onBack: () => void;
  onRefresh: () => void;
  onValidate?: (id: string) => void;
  onImpact?: (id: string) => void;
}) {
  const [detail, setDetail] = useState<ArtifactDetailResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [editContent, setEditContent] = useState("");
  const [history, setHistory] = useState<AuditEventRow[]>([]);
  const [latestDiff, setLatestDiff] = useState<VersionDiff | null>(null);

  useEffect(() => {
    loadDetail();
    loadHistory();
    loadDiff();
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

  async function loadHistory() {
    try {
      const h = await invoke<AuditTimelineResponse>("get_artifact_history", {
        artifactId,
      });
      setHistory(h.events);
    } catch {
      // History may be empty for new artifacts
    }
  }

  async function loadDiff() {
    try {
      const d = await invoke<VersionDiff | null>("get_latest_diff", { artifactId });
      setLatestDiff(d);
    } catch {
      setLatestDiff(null);
    }
  }

  async function doEdit() {
    try {
      const content = JSON.parse(editContent);
      const hash = "hash-" + Date.now();
      const res = await invoke<EditResponse>("edit_artifact_content", {
        artifactId,
        content,
        contentHash: hash,
      });
      if (res.success) {
        setEditing(false);
        setEditContent("");
        await loadDetail();
        await loadHistory();
        onRefresh();
      } else {
        setError(res.error ?? "Edit failed");
      }
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

      {/* Explainability buttons */}
      <h2>Explainability</h2>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {onValidate && (
          <button className="btn btn-ghost" onClick={() => onValidate(artifactId)}>
            Why not valid?
          </button>
        )}
        {onImpact && (
          <button className="btn btn-ghost" onClick={() => onImpact(artifactId)}>
            Blast Radius
          </button>
        )}
      </div>

      {/* Latest Diff */}
      {latestDiff && (
        <>
          <h2>Latest Changes (v{latestDiff.fromVersion.versionNumber} → v{latestDiff.toVersion.versionNumber})</h2>
          {latestDiff.approvalImpact.approvalInvalidated && (
            <div style={{
              padding: "6px 12px",
              background: "rgba(255, 60, 60, 0.15)",
              borderLeft: "3px solid var(--red)",
              fontSize: 12,
              marginBottom: 8,
            }}>
              Approval invalidated: {latestDiff.approvalImpact.reason}
            </div>
          )}
          {latestDiff.contentChanges.length > 0 && (
            <div style={{ fontSize: 12, marginBottom: 8 }}>
              {latestDiff.contentChanges.map((c, i) => (
                <div
                  key={i}
                  style={{
                    display: "grid",
                    gridTemplateColumns: "80px 1fr 1fr",
                    gap: 8,
                    padding: "3px 0",
                    borderBottom: "1px solid var(--border)",
                  }}
                >
                  <span style={{
                    color: c.changeType === "added" ? "var(--green)" : c.changeType === "removed" ? "var(--red)" : "var(--orange)",
                    fontWeight: 600,
                    fontSize: 10,
                    textTransform: "uppercase",
                  }}>
                    {c.changeType}
                  </span>
                  <span style={{ fontFamily: "monospace", fontSize: 11 }}>{c.fieldPath}</span>
                  <span style={{ fontSize: 11, color: "var(--text-dim)" }}>
                    {c.changeType === "modified" && `${c.oldValue ?? ""} → ${c.newValue ?? ""}`}
                    {c.changeType === "added" && (c.newValue ?? "")}
                    {c.changeType === "removed" && (c.oldValue ?? "")}
                  </span>
                </div>
              ))}
            </div>
          )}
          {latestDiff.metadataChanges.length > 0 && (
            <div style={{ fontSize: 11, color: "var(--text-dim)" }}>
              {latestDiff.metadataChanges.map((m, i) => (
                <div key={i}>{m.field}: {m.oldValue} → {m.newValue}</div>
              ))}
            </div>
          )}
        </>
      )}

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

      {/* Content Editor */}
      {artifact.artifactType !== "execution_readiness_gate" &&
        artifact.artifactType !== "constitution" && (
        <>
          <h2>Content</h2>
          {!editing ? (
            <div>
              <pre
                style={{
                  background: "var(--bg-deeper, #1a1a2e)",
                  padding: 12,
                  borderRadius: 3,
                  fontSize: 11,
                  maxHeight: 200,
                  overflow: "auto",
                  marginBottom: 8,
                }}
              >
                {version?.content
                  ? JSON.stringify(version.content, null, 2)
                  : "No content"}
              </pre>
              <button
                className="btn btn-ghost"
                onClick={() => {
                  setEditContent(
                    version?.content
                      ? JSON.stringify(version.content, null, 2)
                      : "{}"
                  );
                  setEditing(true);
                }}
              >
                Edit Content
              </button>
            </div>
          ) : (
            <div>
              <textarea
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                rows={10}
                style={{
                  width: "100%",
                  background: "var(--bg-deeper, #1a1a2e)",
                  color: "var(--text)",
                  border: "1px solid var(--border)",
                  padding: 8,
                  fontFamily: "monospace",
                  fontSize: 11,
                  marginBottom: 8,
                }}
              />
              <div style={{ display: "flex", gap: 8 }}>
                <button className="btn btn-primary" onClick={doEdit}>
                  Save Edit
                </button>
                <button
                  className="btn btn-ghost"
                  onClick={() => setEditing(false)}
                >
                  Cancel
                </button>
              </div>
              <p style={{ fontSize: 11, color: "var(--text-dim)", marginTop: 4 }}>
                Editing an Approved artifact will mark it Stale. Editing a Valid
                artifact will require revalidation. Downstream dependents may be
                marked stale.
              </p>
            </div>
          )}
        </>
      )}

      {/* Artifact History */}
      <h2>History ({history.length})</h2>
      {history.length === 0 ? (
        <div className="empty">No audit events for this artifact yet.</div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 2,
            fontSize: 12,
          }}
        >
          {history.map((evt: AuditEventRow) => (
            <div
              key={evt.id}
              style={{
                display: "grid",
                gridTemplateColumns: "120px 70px 1fr",
                gap: 8,
                padding: "4px 0",
                borderBottom: "1px solid var(--border)",
              }}
            >
              <span style={{ color: "var(--text-dim)", fontFamily: "monospace" }}>
                {evt.occurredAt.replace("T", " ").replace("Z", "")}
              </span>
              <span style={{ color: "var(--text-dim)" }}>{evt.actorName}</span>
              <span>{evt.summary}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

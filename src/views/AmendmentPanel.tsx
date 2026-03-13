import { useState} from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AmendmentResponse } from "../types";

interface Props {
  onRefresh: () => void;
}

export function AmendmentPanel({ onRefresh }: Props) {
  const [reason, setReason] = useState("");
  const [promise, setPromise] = useState("");
  const [qualityBar, setQualityBar] = useState("");
  const [failureCondition, setFailureCondition] = useState("");

  const [activeAmendment, setActiveAmendment] = useState<AmendmentResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handlePropose() {
    setError(null);
    const resp = await invoke<AmendmentResponse>("propose_amendment", {
      reason,
      oneSentencePromise: promise || null,
      userFantasy: null,
      qualityBar: qualityBar || null,
      failureCondition: failureCondition || null,
    });
    if (resp.success) {
      setActiveAmendment(resp);
    } else {
      setError(resp.error);
    }
  }

  async function handleAssess() {
    if (!activeAmendment?.amendmentId) return;
    setError(null);
    const resp = await invoke<AmendmentResponse>("assess_amendment_impact", {
      amendmentId: activeAmendment.amendmentId,
    });
    if (resp.success) {
      setActiveAmendment(resp);
    } else {
      setError(resp.error);
    }
  }

  async function handleApply() {
    if (!activeAmendment?.amendmentId) return;
    setError(null);
    const resp = await invoke<AmendmentResponse>("apply_amendment", {
      amendmentId: activeAmendment.amendmentId,
    });
    if (resp.success) {
      setActiveAmendment(resp);
      onRefresh();
    } else {
      setError(resp.error);
    }
  }

  async function handleAbandon() {
    if (!activeAmendment?.amendmentId) return;
    setError(null);
    const resp = await invoke<AmendmentResponse>("abandon_amendment", {
      amendmentId: activeAmendment.amendmentId,
    });
    if (resp.success) {
      setActiveAmendment(null);
      setReason("");
      setPromise("");
      setQualityBar("");
      setFailureCondition("");
    } else {
      setError(resp.error);
    }
  }

  const isProposed = activeAmendment?.status === "proposed";
  const isAssessed = activeAmendment?.status === "impact_assessed";
  const isApplied = activeAmendment?.status === "applied";

  return (
    <div>
      <h2>Amendment Protocol</h2>
      <p style={{ color: "var(--text-dim)", marginBottom: 16, fontSize: 12 }}>
        The Constitution is lockable, not sacred. Change is allowed but must be
        visible, formal, and force downstream reconciliation.
      </p>

      {error && (
        <div
          style={{
            background: "var(--red)",
            color: "#fff",
            padding: "8px 12px",
            borderRadius: 3,
            marginBottom: 12,
            fontSize: 12,
          }}
        >
          {error}
        </div>
      )}

      {!activeAmendment ? (
        <div>
          <h3>Propose Amendment</h3>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <label style={{ fontSize: 12, color: "var(--text-dim)" }}>
              Reason for amendment *
              <textarea
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                rows={2}
                style={{
                  width: "100%",
                  marginTop: 4,
                  background: "var(--bg-deeper, #1a1a2e)",
                  color: "var(--text)",
                  border: "1px solid var(--border)",
                  padding: 8,
                  fontFamily: "inherit",
                  fontSize: 12,
                }}
              />
            </label>
            <label style={{ fontSize: 12, color: "var(--text-dim)" }}>
              New one-sentence promise (leave blank to keep current)
              <input
                value={promise}
                onChange={(e) => setPromise(e.target.value)}
                style={{
                  width: "100%",
                  marginTop: 4,
                  background: "var(--bg-deeper, #1a1a2e)",
                  color: "var(--text)",
                  border: "1px solid var(--border)",
                  padding: 8,
                  fontFamily: "inherit",
                  fontSize: 12,
                }}
              />
            </label>
            <label style={{ fontSize: 12, color: "var(--text-dim)" }}>
              New quality bar (optional)
              <input
                value={qualityBar}
                onChange={(e) => setQualityBar(e.target.value)}
                style={{
                  width: "100%",
                  marginTop: 4,
                  background: "var(--bg-deeper, #1a1a2e)",
                  color: "var(--text)",
                  border: "1px solid var(--border)",
                  padding: 8,
                  fontFamily: "inherit",
                  fontSize: 12,
                }}
              />
            </label>
            <label style={{ fontSize: 12, color: "var(--text-dim)" }}>
              New failure condition (optional)
              <input
                value={failureCondition}
                onChange={(e) => setFailureCondition(e.target.value)}
                style={{
                  width: "100%",
                  marginTop: 4,
                  background: "var(--bg-deeper, #1a1a2e)",
                  color: "var(--text)",
                  border: "1px solid var(--border)",
                  padding: 8,
                  fontFamily: "inherit",
                  fontSize: 12,
                }}
              />
            </label>
            <button
              onClick={handlePropose}
              disabled={!reason.trim()}
              style={{ marginTop: 8, width: "fit-content" }}
            >
              Propose Amendment
            </button>
          </div>
        </div>
      ) : (
        <div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              marginBottom: 16,
            }}
          >
            <h3 style={{ margin: 0 }}>Amendment {activeAmendment.amendmentId}</h3>
            <span
              style={{
                padding: "2px 8px",
                borderRadius: 3,
                fontSize: 11,
                fontWeight: 700,
                background: isApplied
                  ? "var(--orange)"
                  : isAssessed
                    ? "var(--blue, #5b9bd5)"
                    : "var(--text-dim)",
                color: "#000",
              }}
            >
              {activeAmendment.status?.toUpperCase().replace("_", " ")}
            </span>
          </div>

          {activeAmendment.impactSummary && (
            <div
              style={{
                background: "var(--bg-deeper, #1a1a2e)",
                padding: 12,
                borderRadius: 3,
                marginBottom: 12,
                fontSize: 12,
              }}
            >
              <strong>Impact:</strong> {activeAmendment.impactSummary}
            </div>
          )}

          {activeAmendment.affectedArtifactIds.length > 0 && (
            <div style={{ marginBottom: 12, fontSize: 12 }}>
              <strong>Affected artifacts:</strong>
              <ul style={{ paddingLeft: 16, marginTop: 4 }}>
                {activeAmendment.affectedArtifactIds.map((id) => (
                  <li key={id}>{id}</li>
                ))}
              </ul>
            </div>
          )}

          <div style={{ display: "flex", gap: 8 }}>
            {isProposed && (
              <button onClick={handleAssess}>Assess Impact</button>
            )}
            {isAssessed && (
              <button
                onClick={handleApply}
                style={{ background: "var(--orange)", color: "#000" }}
              >
                Apply Amendment
              </button>
            )}
            {!isApplied && (
              <button onClick={handleAbandon} style={{ opacity: 0.7 }}>
                Abandon
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

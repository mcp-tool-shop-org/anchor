import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AuditTimelineResponse, AuditEventRow } from "../types";

export function AuditTimeline() {
  const [timeline, setTimeline] = useState<AuditTimelineResponse | null>(null);

  useEffect(() => {
    invoke<AuditTimelineResponse>("get_audit_timeline").then(setTimeline);
  }, []);

  if (!timeline) return <div className="loading">Loading timeline...</div>;

  const eventIcon = (eventType: string) => {
    if (eventType.includes("approved")) return "\u2713";
    if (eventType.includes("stale")) return "\u26A0";
    if (eventType.includes("amendment")) return "\u270E";
    if (eventType.includes("created")) return "+";
    if (eventType.includes("alarm")) return "\u2022";
    if (eventType.includes("exported")) return "\u21E5";
    return "\u2022";
  };

  const eventColor = (eventType: string) => {
    if (eventType.includes("approved")) return "var(--green)";
    if (eventType.includes("stale")) return "var(--orange)";
    if (eventType.includes("alarm")) return "var(--red)";
    if (eventType.includes("amendment")) return "var(--blue, #5b9bd5)";
    return "var(--text-dim)";
  };

  return (
    <div>
      <h2>Audit Timeline</h2>
      <p style={{ color: "var(--text-dim)", marginBottom: 16, fontSize: 12 }}>
        {timeline.totalCount} event(s) recorded. Append-only. Immutable.
      </p>

      {timeline.events.length === 0 ? (
        <div
          style={{
            padding: 24,
            textAlign: "center",
            color: "var(--text-dim)",
          }}
        >
          No audit events yet. Events are recorded as you interact with
          artifacts.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          {timeline.events.map((evt: AuditEventRow) => (
            <div
              key={evt.id}
              style={{
                display: "grid",
                gridTemplateColumns: "24px 140px 80px 1fr",
                gap: 8,
                padding: "6px 8px",
                fontSize: 12,
                borderBottom: "1px solid var(--border)",
                alignItems: "center",
              }}
            >
              <span
                style={{
                  color: eventColor(evt.eventType),
                  fontWeight: 700,
                  textAlign: "center",
                }}
              >
                {eventIcon(evt.eventType)}
              </span>
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

//! Anchor Law Engine — Audit Log
//!
//! Every meaningful state change produces an immutable AuditEvent.
//! The audit log is append-only. No event can be deleted or modified.
//!
//! This module provides:
//! - Event construction with proper typing and timestamps
//! - Chronological query by artifact, by type, or full project
//! - Timeline rendering for the UI

use crate::domain::*;

// ─── Event Construction ─────────────────────────────────────

/// A completed audit log entry. The store appends these; nothing removes them.
pub fn emit(
    project_id: &str,
    event_type: AuditEventType,
    actor: AuditActor,
    payload: serde_json::Value,
    timestamp: &str,
    id_sequence: usize,
) -> AuditEvent {
    AuditEvent {
        id: format!("evt-{}-{}", project_id, id_sequence),
        project_id: project_id.to_string(),
        event_type,
        actor,
        occurred_at: timestamp.to_string(),
        payload,
    }
}

// ─── Convenience Constructors ───────────────────────────────

pub fn artifact_created(
    project_id: &str,
    artifact: &Artifact,
    actor: &LocalIdentity,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::ArtifactCreated,
        AuditActor::User(actor.clone()),
        serde_json::json!({
            "artifactId": artifact.id,
            "artifactType": artifact.artifact_type,
            "title": artifact.title,
        }),
        timestamp,
        seq,
    )
}

pub fn artifact_updated(
    project_id: &str,
    artifact_id: &str,
    version_number: u32,
    content_hash: &str,
    actor: &LocalIdentity,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::ArtifactUpdated,
        AuditActor::User(actor.clone()),
        serde_json::json!({
            "artifactId": artifact_id,
            "versionNumber": version_number,
            "contentHash": content_hash,
        }),
        timestamp,
        seq,
    )
}

pub fn artifact_transitioned(
    project_id: &str,
    artifact_id: &str,
    from_state: ArtifactState,
    to_state: ArtifactState,
    actor: AuditActor,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    let event_type = match to_state {
        ArtifactState::Complete => AuditEventType::ArtifactCompleted,
        ArtifactState::Valid => AuditEventType::ArtifactValidated,
        ArtifactState::Approved => AuditEventType::ArtifactApproved,
        ArtifactState::Stale => AuditEventType::ArtifactMarkedStale,
        ArtifactState::Draft => AuditEventType::ArtifactUpdated, // regression
    };
    emit(
        project_id,
        event_type,
        actor,
        serde_json::json!({
            "artifactId": artifact_id,
            "fromState": from_state,
            "toState": to_state,
        }),
        timestamp,
        seq,
    )
}

pub fn artifact_approved(
    project_id: &str,
    artifact_id: &str,
    approval_id: &str,
    actor: &LocalIdentity,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::ArtifactApproved,
        AuditActor::User(actor.clone()),
        serde_json::json!({
            "artifactId": artifact_id,
            "approvalId": approval_id,
        }),
        timestamp,
        seq,
    )
}

pub fn artifact_marked_stale(
    project_id: &str,
    artifact_id: &str,
    reason: &str,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::ArtifactMarkedStale,
        AuditActor::System,
        serde_json::json!({
            "artifactId": artifact_id,
            "reason": reason,
        }),
        timestamp,
        seq,
    )
}

pub fn amendment_started(
    project_id: &str,
    amendment_id: &str,
    reason: &str,
    actor: &LocalIdentity,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::AmendmentStarted,
        AuditActor::User(actor.clone()),
        serde_json::json!({
            "amendmentId": amendment_id,
            "reason": reason,
        }),
        timestamp,
        seq,
    )
}

pub fn amendment_applied(
    project_id: &str,
    amendment_id: &str,
    new_constitution_version_id: &str,
    invalidated_count: usize,
    actor: &LocalIdentity,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::AmendmentApplied,
        AuditActor::User(actor.clone()),
        serde_json::json!({
            "amendmentId": amendment_id,
            "newConstitutionVersionId": new_constitution_version_id,
            "invalidatedArtifactCount": invalidated_count,
        }),
        timestamp,
        seq,
    )
}

pub fn drift_alarm_raised(
    project_id: &str,
    alarm: &DriftAlarm,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::DriftAlarmRaised,
        AuditActor::System,
        serde_json::json!({
            "alarmId": alarm.id,
            "alarmType": alarm.alarm_type,
            "severity": alarm.severity,
            "explanation": alarm.explanation,
        }),
        timestamp,
        seq,
    )
}

pub fn project_saved(
    project_id: &str,
    file_path: &str,
    actor: &LocalIdentity,
    timestamp: &str,
    seq: usize,
) -> AuditEvent {
    emit(
        project_id,
        AuditEventType::ProjectExported, // reuse: save is a form of export
        AuditActor::User(actor.clone()),
        serde_json::json!({
            "action": "project_saved",
            "filePath": file_path,
        }),
        timestamp,
        seq,
    )
}

// ─── Query Functions ────────────────────────────────────────

/// All events for a specific artifact, chronologically.
pub fn events_for_artifact<'a>(
    artifact_id: &str,
    events: &'a [AuditEvent],
) -> Vec<&'a AuditEvent> {
    events
        .iter()
        .filter(|e| {
            e.payload
                .get("artifactId")
                .and_then(|v| v.as_str())
                .map(|id| id == artifact_id)
                .unwrap_or(false)
        })
        .collect()
}

/// All events of a specific type.
pub fn events_by_type<'a>(
    event_type: AuditEventType,
    events: &'a [AuditEvent],
) -> Vec<&'a AuditEvent> {
    events.iter().filter(|e| e.event_type == event_type).collect()
}

/// Full project timeline, already chronological (append-only guarantees this).
pub fn full_timeline(events: &[AuditEvent]) -> &[AuditEvent] {
    events
}

/// Count events by type — useful for the audit summary panel.
pub fn event_type_counts(events: &[AuditEvent]) -> Vec<(AuditEventType, usize)> {
    let mut counts: std::collections::HashMap<AuditEventType, usize> =
        std::collections::HashMap::new();
    for event in events {
        *counts.entry(event.event_type).or_insert(0) += 1;
    }
    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by_key(|(t, _)| format!("{:?}", t));
    result
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> LocalIdentity {
        LocalIdentity {
            id: "user-1".into(),
            display_name: "Tester".into(),
        }
    }

    fn test_artifact() -> Artifact {
        Artifact {
            id: "art-1".into(),
            project_id: "proj-1".into(),
            artifact_type: ArtifactType::FeatureMap,
            title: "Feature Map".into(),
            current_version_id: "art-1-v1".into(),
            state: ArtifactState::Draft,
            validation_summary: ValidationSummary::default(),
            latest_approval_id: None,
            stale_reason: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    #[test]
    fn emit_produces_correct_event() {
        let evt = emit(
            "proj-1",
            AuditEventType::ProjectCreated,
            AuditActor::System,
            serde_json::json!({}),
            "2026-03-13T00:00:00Z",
            1,
        );
        assert_eq!(evt.project_id, "proj-1");
        assert_eq!(evt.event_type, AuditEventType::ProjectCreated);
        assert_eq!(evt.id, "evt-proj-1-1");
    }

    #[test]
    fn artifact_created_event() {
        let art = test_artifact();
        let evt = artifact_created("proj-1", &art, &test_identity(), "2026-03-13T00:00:00Z", 1);
        assert_eq!(evt.event_type, AuditEventType::ArtifactCreated);
        assert_eq!(evt.payload["artifactId"], "art-1");
    }

    #[test]
    fn transition_events_map_correctly() {
        let cases = vec![
            (ArtifactState::Draft, ArtifactState::Complete, AuditEventType::ArtifactCompleted),
            (ArtifactState::Complete, ArtifactState::Valid, AuditEventType::ArtifactValidated),
            (ArtifactState::Valid, ArtifactState::Approved, AuditEventType::ArtifactApproved),
            (ArtifactState::Approved, ArtifactState::Stale, AuditEventType::ArtifactMarkedStale),
        ];
        for (from, to, expected_type) in cases {
            let evt = artifact_transitioned(
                "proj-1",
                "art-1",
                from,
                to,
                AuditActor::System,
                "2026-03-13T00:00:00Z",
                1,
            );
            assert_eq!(evt.event_type, expected_type);
        }
    }

    #[test]
    fn events_for_artifact_filters_correctly() {
        let events = vec![
            artifact_created("proj-1", &test_artifact(), &test_identity(), "2026-03-13T00:00:00Z", 1),
            emit(
                "proj-1",
                AuditEventType::ProjectCreated,
                AuditActor::System,
                serde_json::json!({}),
                "2026-03-13T00:00:00Z",
                2,
            ),
            artifact_updated("proj-1", "art-1", 2, "hash2", &test_identity(), "2026-03-13T00:01:00Z", 3),
        ];
        let filtered = events_for_artifact("art-1", &events);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn events_by_type_works() {
        let events = vec![
            emit("proj-1", AuditEventType::ProjectCreated, AuditActor::System, serde_json::json!({}), "t1", 1),
            emit("proj-1", AuditEventType::ArtifactCreated, AuditActor::System, serde_json::json!({}), "t2", 2),
            emit("proj-1", AuditEventType::ArtifactCreated, AuditActor::System, serde_json::json!({}), "t3", 3),
        ];
        assert_eq!(events_by_type(AuditEventType::ArtifactCreated, &events).len(), 2);
        assert_eq!(events_by_type(AuditEventType::ProjectCreated, &events).len(), 1);
    }

    #[test]
    fn event_type_counts_works() {
        let events = vec![
            emit("proj-1", AuditEventType::ArtifactCreated, AuditActor::System, serde_json::json!({}), "t1", 1),
            emit("proj-1", AuditEventType::ArtifactCreated, AuditActor::System, serde_json::json!({}), "t2", 2),
            emit("proj-1", AuditEventType::ArtifactApproved, AuditActor::System, serde_json::json!({}), "t3", 3),
        ];
        let counts = event_type_counts(&events);
        assert!(counts.iter().any(|(t, c)| *t == AuditEventType::ArtifactCreated && *c == 2));
        assert!(counts.iter().any(|(t, c)| *t == AuditEventType::ArtifactApproved && *c == 1));
    }

    #[test]
    fn amendment_events_carry_correct_payloads() {
        let evt = amendment_started("proj-1", "amend-1", "Fix promise", &test_identity(), "t1", 1);
        assert_eq!(evt.payload["amendmentId"], "amend-1");
        assert_eq!(evt.payload["reason"], "Fix promise");

        let evt2 = amendment_applied("proj-1", "amend-1", "cv2", 5, &test_identity(), "t2", 2);
        assert_eq!(evt2.payload["newConstitutionVersionId"], "cv2");
        assert_eq!(evt2.payload["invalidatedArtifactCount"], 5);
    }

    #[test]
    fn stale_event_includes_reason() {
        let evt = artifact_marked_stale("proj-1", "art-3", "upstream changed", "t1", 1);
        assert_eq!(evt.payload["reason"], "upstream changed");
    }
}

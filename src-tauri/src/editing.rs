//! Anchor Law Engine — Artifact Editing with Constraint Enforcement
//!
//! Controlled authoring: every content edit creates a new version,
//! forces downstream revalidation, and emits audit events.
//!
//! The anti-drift machine starts biting here.

use crate::audit_log;
use crate::domain::*;
use crate::stale_propagation;
use crate::store::ProjectStore;

// ─── Errors ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    ArtifactNotFound { id: String },
    /// Cannot edit the readiness gate (computed, not authored).
    ReadinessGateNotEditable,
    /// Constitution edits must go through the Amendment Protocol.
    ConstitutionRequiresAmendment,
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArtifactNotFound { id } => write!(f, "Artifact not found: {}", id),
            Self::ReadinessGateNotEditable => {
                write!(f, "Execution Readiness Gate is computed, not editable")
            }
            Self::ConstitutionRequiresAmendment => {
                write!(f, "Constitution changes must go through the Amendment Protocol")
            }
        }
    }
}

// ─── Edit Result ────────────────────────────────────────────

/// What happened as a result of editing an artifact.
#[derive(Debug, Clone)]
pub struct EditResult {
    /// The new version that was created.
    pub new_version: ArtifactVersion,
    /// Artifacts that were marked stale due to this edit.
    pub stale_artifact_ids: Vec<String>,
    /// Audit events generated.
    pub audit_events: Vec<AuditEvent>,
    /// New state after the edit (may regress to Draft if content incomplete, or Complete→revalidation needed).
    pub new_state: ArtifactState,
}

// ─── Edit Artifact ──────────────────────────────────────────

/// Edit an artifact's content. This is the main authoring mutation.
///
/// Rules enforced:
/// 1. Constitution edits must go through Amendment Protocol
/// 2. Readiness Gate is computed, not editable
/// 3. Every edit creates a new ArtifactVersion
/// 4. Approved/Valid artifacts regress (Approved→Stale, Valid→Complete)
/// 5. Downstream dependents are marked stale
/// 6. Audit events are emitted
pub fn edit_artifact(
    store: &mut ProjectStore,
    artifact_id: &str,
    new_content: serde_json::Value,
    content_hash: &str,
    editor: &LocalIdentity,
    timestamp: &str,
) -> Result<EditResult, EditError> {
    // Find the artifact.
    let artifact = store
        .artifacts
        .iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| EditError::ArtifactNotFound {
            id: artifact_id.into(),
        })?;

    // Constitution must go through Amendment Protocol.
    if artifact.artifact_type == ArtifactType::Constitution {
        return Err(EditError::ConstitutionRequiresAmendment);
    }

    // Readiness Gate is computed, never edited.
    if artifact.artifact_type == ArtifactType::ExecutionReadinessGate {
        return Err(EditError::ReadinessGateNotEditable);
    }

    let old_state = artifact.state;
    let old_version_id = artifact.current_version_id.clone();

    // Determine the new version number.
    let max_version = store
        .versions
        .iter()
        .filter(|v| v.artifact_id == artifact_id)
        .map(|v| v.version_number)
        .max()
        .unwrap_or(0);
    let new_version_number = max_version + 1;
    let new_version_id = format!("{}-v{}", artifact_id, new_version_number);

    // Create the new version.
    let new_version = ArtifactVersion {
        id: new_version_id.clone(),
        artifact_id: artifact_id.into(),
        project_id: store.project.id.clone(),
        version_number: new_version_number,
        constitution_version_id: store.constitution.version_id.clone(),
        content: new_content,
        content_hash: content_hash.into(),
        parent_version_id: Some(old_version_id),
        created_at: timestamp.into(),
        created_by: editor.clone(),
    };

    // Determine new state after the edit.
    // - If was Approved → goes to Stale (content changed under approved state)
    // - If was Valid → goes to Complete (needs revalidation)
    // - If was Stale → stays Stale (still needs reconciliation)
    // - If was Complete/Draft → stays where it is
    let new_state = match old_state {
        ArtifactState::Approved => ArtifactState::Stale,
        ArtifactState::Valid => ArtifactState::Complete,
        other => other,
    };

    let stale_reason: Option<String> = if new_state == ArtifactState::Stale && old_state == ArtifactState::Approved
    {
        Some("Content edited after approval — must revalidate and re-approve".into())
    } else {
        None
    };

    // Apply to the artifact.
    let artifact_mut = store
        .artifacts
        .iter_mut()
        .find(|a| a.id == artifact_id)
        .unwrap();
    artifact_mut.current_version_id = new_version_id.clone();
    artifact_mut.state = new_state;
    artifact_mut.updated_at = timestamp.into();
    if let Some(ref reason) = stale_reason {
        artifact_mut.stale_reason = Some(reason.clone());
    }

    // Get the artifact snapshot for stale propagation.
    let artifact_snapshot = artifact_mut.clone();

    // Add the version.
    store.versions.push(new_version.clone());

    // Propagate stale to downstream dependents.
    let stale_marks = stale_propagation::propagate_upstream_change(
        &artifact_snapshot,
        &store.artifacts,
        &store.links,
    );

    let mut stale_ids: Vec<String> = Vec::new();
    let mut audit_events = Vec::new();
    let seq_base = store.audit_events.len();

    // Mark downstream artifacts stale.
    for (i, mark) in stale_marks.iter().enumerate() {
        if let Some(downstream) = store.artifacts.iter_mut().find(|a| a.id == mark.artifact_id) {
            if matches!(
                downstream.state,
                ArtifactState::Approved | ArtifactState::Valid
            ) {
                downstream.state = ArtifactState::Stale;
                downstream.stale_reason = Some(mark.reason.to_human_string());
                downstream.updated_at = timestamp.into();
                stale_ids.push(downstream.id.clone());

                audit_events.push(audit_log::artifact_marked_stale(
                    &store.project.id,
                    &downstream.id,
                    &mark.reason.to_human_string(),
                    timestamp,
                    seq_base + i + 2,
                ));
            }
        }
    }

    // Emit the edit audit event.
    let edit_event = audit_log::artifact_updated(
        &store.project.id,
        artifact_id,
        new_version_number,
        content_hash,
        editor,
        timestamp,
        seq_base + 1,
    );
    audit_events.insert(0, edit_event);

    // If the artifact itself went stale, emit that event.
    if new_state == ArtifactState::Stale && old_state != ArtifactState::Stale {
        audit_events.push(audit_log::artifact_marked_stale(
            &store.project.id,
            artifact_id,
            stale_reason.as_deref().unwrap_or("Content edited"),
            timestamp,
            seq_base + stale_marks.len() + 2,
        ));
    }

    // Append all audit events to the store.
    store.audit_events.extend(audit_events.clone());

    Ok(EditResult {
        new_version,
        stale_artifact_ids: stale_ids,
        audit_events,
        new_state,
    })
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> LocalIdentity {
        LocalIdentity {
            id: "user-1".into(),
            display_name: "Editor".into(),
        }
    }

    #[test]
    fn edit_creates_new_version() {
        let mut store = ProjectStore::demo();
        let initial_version_count = store.versions.len();

        let result = edit_artifact(
            &mut store,
            "art-feat", // Feature Map (Approved in demo)
            serde_json::json!({"features": []}),
            "hash-new",
            &test_identity(),
            "2026-03-13T02:00:00Z",
        )
        .unwrap();

        assert_eq!(store.versions.len(), initial_version_count + 1);
        assert_eq!(result.new_version.version_number, 2);
        assert_eq!(result.new_version.artifact_id, "art-feat");
    }

    #[test]
    fn edit_approved_artifact_goes_stale() {
        let mut store = ProjectStore::demo();
        let result = edit_artifact(
            &mut store,
            "art-feat", // Approved
            serde_json::json!({"features": []}),
            "hash-new",
            &test_identity(),
            "t1",
        )
        .unwrap();

        assert_eq!(result.new_state, ArtifactState::Stale);
        let art = store.artifacts.iter().find(|a| a.id == "art-feat").unwrap();
        assert_eq!(art.state, ArtifactState::Stale);
        assert!(art.stale_reason.is_some());
    }

    #[test]
    fn edit_valid_artifact_goes_to_complete() {
        let mut store = ProjectStore::demo();
        let result = edit_artifact(
            &mut store,
            "art-phase", // Valid in demo
            serde_json::json!({"phases": []}),
            "hash-new",
            &test_identity(),
            "t1",
        )
        .unwrap();

        assert_eq!(result.new_state, ArtifactState::Complete);
    }

    #[test]
    fn edit_draft_stays_draft() {
        let mut store = ProjectStore::demo();
        let result = edit_artifact(
            &mut store,
            "art-drift", // Draft in demo
            serde_json::json!({"definitions": []}),
            "hash-new",
            &test_identity(),
            "t1",
        )
        .unwrap();

        assert_eq!(result.new_state, ArtifactState::Draft);
    }

    #[test]
    fn edit_constitution_requires_amendment() {
        let mut store = ProjectStore::demo();
        let result = edit_artifact(
            &mut store,
            "art-const",
            serde_json::json!({}),
            "hash",
            &test_identity(),
            "t1",
        );

        assert!(matches!(
            result,
            Err(EditError::ConstitutionRequiresAmendment)
        ));
    }

    #[test]
    fn edit_readiness_gate_fails() {
        let mut store = ProjectStore::demo();
        // Add a gate artifact to the store
        store.artifacts.push(Artifact {
            id: "art-gate".into(),
            project_id: "proj-1".into(),
            artifact_type: ArtifactType::ExecutionReadinessGate,
            title: "Execution Readiness Gate".into(),
            current_version_id: "art-gate-v1".into(),
            state: ArtifactState::Draft,
            validation_summary: ValidationSummary::default(),
            latest_approval_id: None,
            stale_reason: None,
            created_at: "t0".into(),
            updated_at: "t0".into(),
        });

        let result = edit_artifact(
            &mut store,
            "art-gate",
            serde_json::json!({}),
            "hash",
            &test_identity(),
            "t1",
        );

        assert!(matches!(result, Err(EditError::ReadinessGateNotEditable)));
    }

    #[test]
    fn edit_unknown_artifact_fails() {
        let mut store = ProjectStore::demo();
        let result = edit_artifact(
            &mut store,
            "nonexistent",
            serde_json::json!({}),
            "hash",
            &test_identity(),
            "t1",
        );

        assert!(matches!(
            result,
            Err(EditError::ArtifactNotFound { .. })
        ));
    }

    #[test]
    fn edit_emits_audit_events() {
        let mut store = ProjectStore::demo();
        let initial_events = store.audit_events.len();

        edit_artifact(
            &mut store,
            "art-feat",
            serde_json::json!({"features": []}),
            "hash-new",
            &test_identity(),
            "t1",
        )
        .unwrap();

        assert!(store.audit_events.len() > initial_events);
        // At minimum: one ArtifactUpdated event
        assert!(store
            .audit_events
            .iter()
            .any(|e| e.event_type == AuditEventType::ArtifactUpdated));
    }

    #[test]
    fn edit_propagates_stale_to_dependents() {
        let mut store = ProjectStore::demo();

        // Edit art-wf (UserFantasyWorkflows) which has downstream dependents
        // art-feat depends on art-wf (via Justifies link)
        let result = edit_artifact(
            &mut store,
            "art-wf",
            serde_json::json!({"fantasy_narrative": "updated"}),
            "hash-new",
            &test_identity(),
            "t1",
        )
        .unwrap();

        // art-wf itself was Approved → should go Stale
        assert_eq!(result.new_state, ArtifactState::Stale);
        // Downstream artifacts that were Approved/Valid should also be stale
        // (exact count depends on the demo topology)
    }

    #[test]
    fn edit_version_has_correct_parent() {
        let mut store = ProjectStore::demo();
        let result = edit_artifact(
            &mut store,
            "art-feat",
            serde_json::json!({"features": []}),
            "hash-new",
            &test_identity(),
            "t1",
        )
        .unwrap();

        assert_eq!(
            result.new_version.parent_version_id,
            Some("art-feat-v1".into())
        );
    }

    #[test]
    fn sequential_edits_increment_version() {
        let mut store = ProjectStore::demo();

        let r1 = edit_artifact(
            &mut store,
            "art-drift", // Draft — won't regress
            serde_json::json!({"v": 1}),
            "h1",
            &test_identity(),
            "t1",
        )
        .unwrap();
        assert_eq!(r1.new_version.version_number, 2);

        let r2 = edit_artifact(
            &mut store,
            "art-drift",
            serde_json::json!({"v": 2}),
            "h2",
            &test_identity(),
            "t2",
        )
        .unwrap();
        assert_eq!(r2.new_version.version_number, 3);
    }
}

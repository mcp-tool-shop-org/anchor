//! Anchor Law Engine — Amendment Protocol
//!
//! The Constitution is lockable, not sacred. Change is allowed
//! but must be visible, formal, and force downstream reconciliation.
//!
//! Amendment lifecycle:
//!   Proposed → ImpactAssessed → Applied → ReconciliationPending → Completed
//!
//! At each stage the system enforces constraints:
//! - Proposed: reason + changes captured, one active amendment at a time
//! - ImpactAssessed: engine computes which artifacts become stale
//! - Applied: constitution updated, all affected artifacts marked stale
//! - ReconciliationPending: export blocked until every stale artifact is re-approved
//! - Completed: all downstream artifacts reconciled, amendment sealed

use crate::domain::*;
use crate::stale_propagation;

// ─── Errors ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmendmentError {
    /// Another amendment is already active.
    AmendmentAlreadyActive { active_id: String },
    /// Amendment not found.
    NotFound { id: String },
    /// Invalid state transition for this amendment.
    InvalidTransition {
        amendment_id: String,
        current_status: AmendmentStatus,
        attempted_status: AmendmentStatus,
    },
    /// Constitution is not locked (nothing to amend).
    ConstitutionNotLocked,
    /// Patch is empty — no actual changes proposed.
    EmptyPatch,
    /// Cannot complete: artifacts still stale.
    ReconciliationIncomplete {
        stale_artifact_ids: Vec<String>,
    },
}

impl std::fmt::Display for AmendmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmendmentAlreadyActive { active_id } => {
                write!(f, "Amendment already active: {}", active_id)
            }
            Self::NotFound { id } => write!(f, "Amendment not found: {}", id),
            Self::InvalidTransition { amendment_id, current_status, attempted_status } => {
                write!(
                    f,
                    "Amendment {}: cannot transition {:?} → {:?}",
                    amendment_id, current_status, attempted_status
                )
            }
            Self::ConstitutionNotLocked => write!(f, "Constitution is not locked"),
            Self::EmptyPatch => write!(f, "Amendment patch is empty — no changes proposed"),
            Self::ReconciliationIncomplete { stale_artifact_ids } => {
                write!(
                    f,
                    "Reconciliation incomplete: {} artifacts still stale",
                    stale_artifact_ids.len()
                )
            }
        }
    }
}

// ─── Impact Assessment ──────────────────────────────────────

/// Result of computing amendment impact before applying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactAssessment {
    pub amendment_id: String,
    /// Artifact IDs that will be marked stale if this amendment is applied.
    pub affected_artifact_ids: Vec<String>,
    /// Human-readable summary.
    pub summary: String,
}

// ─── Propose ────────────────────────────────────────────────

/// Start a new amendment. Fails if another amendment is already active
/// or the constitution is not locked.
pub fn propose_amendment(
    project: &Project,
    constitution: &Constitution,
    amendments: &[Amendment],
    proposed_changes: ConstitutionPatch,
    reason: String,
    proposer: LocalIdentity,
    timestamp: &str,
    id_sequence: usize,
) -> Result<Amendment, AmendmentError> {
    if !constitution.locked {
        return Err(AmendmentError::ConstitutionNotLocked);
    }

    // Only one active amendment at a time.
    if let Some(active) = amendments.iter().find(|a| {
        !matches!(
            a.status,
            AmendmentStatus::Completed | AmendmentStatus::Abandoned
        )
    }) {
        return Err(AmendmentError::AmendmentAlreadyActive {
            active_id: active.id.clone(),
        });
    }

    // At least one field must actually change.
    if patch_is_empty(&proposed_changes) {
        return Err(AmendmentError::EmptyPatch);
    }

    Ok(Amendment {
        id: format!("amend-{}", id_sequence),
        project_id: project.id.clone(),
        target_constitution_version_id: constitution.version_id.clone(),
        proposed_changes,
        reason,
        expected_impact_summary: String::new(), // filled during assess_impact
        invalidated_artifact_ids: Vec::new(),
        resulting_constitution_version_id: None,
        proposer,
        status: AmendmentStatus::Proposed,
        created_at: timestamp.into(),
        updated_at: timestamp.into(),
        applied_at: None,
    })
}

// ─── Assess Impact ──────────────────────────────────────────

/// Compute which artifacts will be marked stale if this amendment is applied.
/// Moves amendment from Proposed → ImpactAssessed.
pub fn assess_impact(
    amendment: &mut Amendment,
    artifacts: &[Artifact],
) -> Result<ImpactAssessment, AmendmentError> {
    if amendment.status != AmendmentStatus::Proposed {
        return Err(AmendmentError::InvalidTransition {
            amendment_id: amendment.id.clone(),
            current_status: amendment.status,
            attempted_status: AmendmentStatus::ImpactAssessed,
        });
    }

    // Constitution amendment = nuclear: everything Valid/Approved downstream
    let stale_marks = stale_propagation::propagate_constitution_amendment(
        artifacts,
        &amendment.target_constitution_version_id,
        &format!("{}-next", amendment.target_constitution_version_id),
    );

    let affected_ids: Vec<String> = stale_marks
        .iter()
        .map(|m| m.artifact_id.clone())
        .collect();

    let summary = format!(
        "Amendment will invalidate {} artifact(s): {}",
        affected_ids.len(),
        affected_ids.join(", ")
    );

    amendment.invalidated_artifact_ids = affected_ids.clone();
    amendment.expected_impact_summary = summary.clone();
    amendment.status = AmendmentStatus::ImpactAssessed;

    Ok(ImpactAssessment {
        amendment_id: amendment.id.clone(),
        affected_artifact_ids: affected_ids,
        summary,
    })
}

// ─── Apply ──────────────────────────────────────────────────

/// Apply the amendment: update constitution, mark affected artifacts stale.
/// Moves amendment from ImpactAssessed → Applied.
///
/// Returns the new constitution and the list of artifacts that were marked stale.
pub fn apply_amendment(
    amendment: &mut Amendment,
    constitution: &mut Constitution,
    artifacts: &mut [Artifact],
    timestamp: &str,
    new_version_id: &str,
) -> Result<Vec<String>, AmendmentError> {
    if amendment.status != AmendmentStatus::ImpactAssessed {
        return Err(AmendmentError::InvalidTransition {
            amendment_id: amendment.id.clone(),
            current_status: amendment.status,
            attempted_status: AmendmentStatus::Applied,
        });
    }

    // Apply the patch to the constitution.
    apply_constitution_patch(constitution, &amendment.proposed_changes);
    constitution.version_id = new_version_id.into();
    constitution.parent_version_id = Some(amendment.target_constitution_version_id.clone());
    constitution.updated_at = timestamp.into();
    constitution.content_hash = format!("hash-{}", new_version_id);

    // Mark affected artifacts stale.
    let mut stale_ids = Vec::new();
    for artifact in artifacts.iter_mut() {
        if amendment.invalidated_artifact_ids.contains(&artifact.id)
            && matches!(
                artifact.state,
                ArtifactState::Approved | ArtifactState::Valid
            )
        {
            artifact.state = ArtifactState::Stale;
            artifact.stale_reason = Some(format!(
                "Constitution amended: {} → {}",
                amendment.target_constitution_version_id, new_version_id
            ));
            artifact.updated_at = timestamp.into();
            stale_ids.push(artifact.id.clone());
        }
    }

    amendment.resulting_constitution_version_id = Some(new_version_id.into());
    amendment.status = AmendmentStatus::Applied;
    amendment.applied_at = Some(timestamp.into());
    amendment.updated_at = timestamp.into();

    Ok(stale_ids)
}

// ─── Complete / Abandon ─────────────────────────────────────

/// Check if all affected artifacts have been reconciled (no longer stale).
/// If so, move amendment to Completed. If not, return the stale IDs.
pub fn try_complete_amendment(
    amendment: &mut Amendment,
    artifacts: &[Artifact],
    timestamp: &str,
) -> Result<(), AmendmentError> {
    if !matches!(
        amendment.status,
        AmendmentStatus::Applied | AmendmentStatus::ReconciliationPending
    ) {
        return Err(AmendmentError::InvalidTransition {
            amendment_id: amendment.id.clone(),
            current_status: amendment.status,
            attempted_status: AmendmentStatus::Completed,
        });
    }

    let still_stale: Vec<String> = amendment
        .invalidated_artifact_ids
        .iter()
        .filter(|id| {
            artifacts
                .iter()
                .any(|a| &a.id == *id && a.state == ArtifactState::Stale)
        })
        .cloned()
        .collect();

    if !still_stale.is_empty() {
        amendment.status = AmendmentStatus::ReconciliationPending;
        amendment.updated_at = timestamp.into();
        return Err(AmendmentError::ReconciliationIncomplete {
            stale_artifact_ids: still_stale,
        });
    }

    amendment.status = AmendmentStatus::Completed;
    amendment.updated_at = timestamp.into();
    Ok(())
}

/// Abandon an amendment that hasn't been applied yet.
pub fn abandon_amendment(
    amendment: &mut Amendment,
    timestamp: &str,
) -> Result<(), AmendmentError> {
    if matches!(
        amendment.status,
        AmendmentStatus::Applied
            | AmendmentStatus::ReconciliationPending
            | AmendmentStatus::Completed
            | AmendmentStatus::Abandoned
    ) {
        return Err(AmendmentError::InvalidTransition {
            amendment_id: amendment.id.clone(),
            current_status: amendment.status,
            attempted_status: AmendmentStatus::Abandoned,
        });
    }

    amendment.status = AmendmentStatus::Abandoned;
    amendment.updated_at = timestamp.into();
    Ok(())
}

// ─── Helpers ────────────────────────────────────────────────

fn patch_is_empty(patch: &ConstitutionPatch) -> bool {
    patch.one_sentence_promise.is_none()
        && patch.user_fantasy.is_none()
        && patch.non_negotiable_outcomes.is_none()
        && patch.anti_goals.is_none()
        && patch.quality_bar.is_none()
        && patch.failure_condition.is_none()
}

fn apply_constitution_patch(constitution: &mut Constitution, patch: &ConstitutionPatch) {
    if let Some(ref v) = patch.one_sentence_promise {
        constitution.one_sentence_promise = v.clone();
    }
    if let Some(ref v) = patch.user_fantasy {
        constitution.user_fantasy = v.clone();
    }
    if let Some(ref v) = patch.non_negotiable_outcomes {
        constitution.non_negotiable_outcomes = v.clone();
    }
    if let Some(ref v) = patch.anti_goals {
        constitution.anti_goals = v.clone();
    }
    if let Some(ref v) = patch.quality_bar {
        constitution.quality_bar = v.clone();
    }
    if let Some(ref v) = patch.failure_condition {
        constitution.failure_condition = v.clone();
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectStore;

    fn demo() -> ProjectStore {
        ProjectStore::demo()
    }

    fn test_patch() -> ConstitutionPatch {
        ConstitutionPatch {
            one_sentence_promise: Some("Updated promise".into()),
            user_fantasy: None,
            non_negotiable_outcomes: None,
            anti_goals: None,
            quality_bar: None,
            failure_condition: None,
        }
    }

    fn test_identity() -> LocalIdentity {
        LocalIdentity {
            id: "user-1".into(),
            display_name: "Designer".into(),
        }
    }

    #[test]
    fn propose_succeeds_with_locked_constitution() {
        let store = demo();
        let amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Refine the promise".into(),
            test_identity(),
            "2026-03-13T01:00:00Z",
            1,
        );
        assert!(amendment.is_ok());
        let a = amendment.unwrap();
        assert_eq!(a.status, AmendmentStatus::Proposed);
        assert_eq!(a.reason, "Refine the promise");
    }

    #[test]
    fn propose_fails_with_empty_patch() {
        let store = demo();
        let empty = ConstitutionPatch {
            one_sentence_promise: None,
            user_fantasy: None,
            non_negotiable_outcomes: None,
            anti_goals: None,
            quality_bar: None,
            failure_condition: None,
        };
        let result = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            empty,
            "No changes".into(),
            test_identity(),
            "t1",
            1,
        );
        assert!(matches!(result, Err(AmendmentError::EmptyPatch)));
    }

    #[test]
    fn propose_fails_with_active_amendment() {
        let store = demo();
        let first = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "First".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();

        let result = propose_amendment(
            &store.project,
            &store.constitution,
            &[first],
            test_patch(),
            "Second".into(),
            test_identity(),
            "t2",
            2,
        );
        assert!(matches!(
            result,
            Err(AmendmentError::AmendmentAlreadyActive { .. })
        ));
    }

    #[test]
    fn propose_fails_with_unlocked_constitution() {
        let mut store = demo();
        store.constitution.locked = false;
        let result = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        );
        assert!(matches!(
            result,
            Err(AmendmentError::ConstitutionNotLocked)
        ));
    }

    #[test]
    fn assess_impact_identifies_affected_artifacts() {
        let store = demo();
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();

        let impact = assess_impact(&mut amendment, &store.artifacts).unwrap();

        // The demo has 5 Approved artifacts (including constitution) and 1 Valid
        // Constitution amendment marks downstream Valid/Approved as stale
        // (Constitution itself is not downstream of itself)
        assert!(!impact.affected_artifact_ids.is_empty());
        assert_eq!(amendment.status, AmendmentStatus::ImpactAssessed);
    }

    #[test]
    fn apply_marks_artifacts_stale() {
        let mut store = demo();
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();
        assess_impact(&mut amendment, &store.artifacts).unwrap();

        let stale_ids = apply_amendment(
            &mut amendment,
            &mut store.constitution,
            &mut store.artifacts,
            "t2",
            "cv2",
        )
        .unwrap();

        assert!(!stale_ids.is_empty());
        assert_eq!(amendment.status, AmendmentStatus::Applied);
        assert_eq!(store.constitution.one_sentence_promise, "Updated promise");
        assert_eq!(store.constitution.version_id, "cv2");

        // Verify affected artifacts are actually stale now
        for id in &stale_ids {
            let art = store.artifacts.iter().find(|a| &a.id == id).unwrap();
            assert_eq!(art.state, ArtifactState::Stale);
            assert!(art.stale_reason.is_some());
        }
    }

    #[test]
    fn complete_fails_with_stale_artifacts() {
        let mut store = demo();
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();
        assess_impact(&mut amendment, &store.artifacts).unwrap();
        apply_amendment(
            &mut amendment,
            &mut store.constitution,
            &mut store.artifacts,
            "t2",
            "cv2",
        )
        .unwrap();

        let result = try_complete_amendment(&mut amendment, &store.artifacts, "t3");
        assert!(matches!(
            result,
            Err(AmendmentError::ReconciliationIncomplete { .. })
        ));
    }

    #[test]
    fn complete_succeeds_after_reconciliation() {
        let mut store = demo();
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();
        assess_impact(&mut amendment, &store.artifacts).unwrap();
        apply_amendment(
            &mut amendment,
            &mut store.constitution,
            &mut store.artifacts,
            "t2",
            "cv2",
        )
        .unwrap();

        // Simulate reconciliation: move all stale artifacts to Approved
        for art in store.artifacts.iter_mut() {
            if art.state == ArtifactState::Stale {
                art.state = ArtifactState::Approved;
                art.stale_reason = None;
            }
        }

        let result = try_complete_amendment(&mut amendment, &store.artifacts, "t3");
        assert!(result.is_ok());
        assert_eq!(amendment.status, AmendmentStatus::Completed);
    }

    #[test]
    fn abandon_works_before_apply() {
        let store = demo();
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();

        assert!(abandon_amendment(&mut amendment, "t2").is_ok());
        assert_eq!(amendment.status, AmendmentStatus::Abandoned);
    }

    #[test]
    fn abandon_fails_after_apply() {
        let mut store = demo();
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Test".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();
        assess_impact(&mut amendment, &store.artifacts).unwrap();
        apply_amendment(
            &mut amendment,
            &mut store.constitution,
            &mut store.artifacts,
            "t2",
            "cv2",
        )
        .unwrap();

        let result = abandon_amendment(&mut amendment, "t3");
        assert!(matches!(
            result,
            Err(AmendmentError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn full_amendment_lifecycle() {
        let mut store = demo();

        // 1. Propose
        let mut amendment = propose_amendment(
            &store.project,
            &store.constitution,
            &store.amendments,
            test_patch(),
            "Promise was too vague".into(),
            test_identity(),
            "t1",
            1,
        )
        .unwrap();
        assert_eq!(amendment.status, AmendmentStatus::Proposed);

        // 2. Assess impact
        let impact = assess_impact(&mut amendment, &store.artifacts).unwrap();
        assert_eq!(amendment.status, AmendmentStatus::ImpactAssessed);
        assert!(!impact.affected_artifact_ids.is_empty());

        // 3. Apply
        let stale_ids = apply_amendment(
            &mut amendment,
            &mut store.constitution,
            &mut store.artifacts,
            "t2",
            "cv2",
        )
        .unwrap();
        assert_eq!(amendment.status, AmendmentStatus::Applied);
        assert!(!stale_ids.is_empty());

        // 4. Try complete (should fail — stale artifacts exist)
        let result = try_complete_amendment(&mut amendment, &store.artifacts, "t3");
        assert!(result.is_err());
        assert_eq!(amendment.status, AmendmentStatus::ReconciliationPending);

        // 5. Reconcile: fix all stale artifacts
        for art in store.artifacts.iter_mut() {
            if art.state == ArtifactState::Stale {
                art.state = ArtifactState::Approved;
                art.stale_reason = None;
            }
        }

        // 6. Complete
        try_complete_amendment(&mut amendment, &store.artifacts, "t4").unwrap();
        assert_eq!(amendment.status, AmendmentStatus::Completed);
    }

    #[test]
    fn assess_before_propose_fails() {
        let store = demo();
        let mut bad = Amendment {
            id: "bad".into(),
            project_id: "proj-1".into(),
            target_constitution_version_id: "cv1".into(),
            proposed_changes: test_patch(),
            reason: "test".into(),
            expected_impact_summary: String::new(),
            invalidated_artifact_ids: vec![],
            resulting_constitution_version_id: None,
            proposer: test_identity(),
            status: AmendmentStatus::Applied, // wrong state
            created_at: "t1".into(),
            updated_at: "t1".into(),
            applied_at: None,
        };
        let result = assess_impact(&mut bad, &store.artifacts);
        assert!(matches!(
            result,
            Err(AmendmentError::InvalidTransition { .. })
        ));
    }
}

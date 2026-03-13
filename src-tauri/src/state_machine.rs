//! Anchor Canonical Schema Pack v1 — Artifact Lifecycle State Machine
//!
//! Enforces legal state transitions, validation interaction rules,
//! and stale propagation. This is the law. The UI is the window.

use crate::domain::{
    Artifact, ArtifactState, ArtifactType, DriftAlarmSeverity, DriftAlarmStatus,
    GateStatus, ValidationStatus, ValidationSummary,
};

// ─── State Transition Rules ─────────────────────────────────

/// All legal state transitions for an artifact.
/// §7.2 of the Canonical Schema Pack.
const LEGAL_TRANSITIONS: &[(ArtifactState, ArtifactState)] = &[
    (ArtifactState::Draft, ArtifactState::Complete),
    (ArtifactState::Complete, ArtifactState::Draft),
    (ArtifactState::Complete, ArtifactState::Valid),
    (ArtifactState::Valid, ArtifactState::Complete),
    (ArtifactState::Valid, ArtifactState::Approved),
    (ArtifactState::Approved, ArtifactState::Stale),
    (ArtifactState::Stale, ArtifactState::Complete),
    (ArtifactState::Stale, ArtifactState::Valid),
    // Stale -> Approved only after revalidation + reapproval (guarded below)
    (ArtifactState::Stale, ArtifactState::Approved),
];

/// Explicitly forbidden transitions.
/// §7.2: these must never be allowed regardless of context.
const FORBIDDEN_TRANSITIONS: &[(ArtifactState, ArtifactState)] = &[
    (ArtifactState::Draft, ArtifactState::Approved),
    (ArtifactState::Draft, ArtifactState::Valid),
    (ArtifactState::Complete, ArtifactState::Approved),
    (ArtifactState::Approved, ArtifactState::Draft),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// Transition is structurally forbidden by the state machine.
    Forbidden {
        from: ArtifactState,
        to: ArtifactState,
        reason: &'static str,
    },
    /// Transition requires preconditions that are not met.
    PreconditionFailed {
        from: ArtifactState,
        to: ArtifactState,
        missing: Vec<String>,
    },
}

/// Check whether a state transition is structurally legal.
/// Does NOT check preconditions — use `validate_transition` for full check.
pub fn is_legal_transition(from: ArtifactState, to: ArtifactState) -> bool {
    if from == to {
        return false; // no-op transitions are not legal events
    }
    LEGAL_TRANSITIONS.iter().any(|&(f, t)| f == from && t == to)
}

/// Validate a full transition including preconditions.
/// Returns Ok(()) if the transition is allowed, or a detailed error.
pub fn validate_transition(
    artifact: &Artifact,
    to: ArtifactState,
    validation: &ValidationSummary,
    has_current_approval: bool,
    has_revalidation: bool,
) -> Result<(), TransitionError> {
    let from = artifact.state;

    // Check forbidden first
    if FORBIDDEN_TRANSITIONS.iter().any(|&(f, t)| f == from && t == to) {
        return Err(TransitionError::Forbidden {
            from,
            to,
            reason: forbidden_reason(from, to),
        });
    }

    // Check structural legality
    if !is_legal_transition(from, to) {
        return Err(TransitionError::Forbidden {
            from,
            to,
            reason: "Transition not in legal transition table",
        });
    }

    // Check preconditions per target state
    let mut missing = Vec::new();

    match to {
        ArtifactState::Complete => {
            // §7.1: all required fields present — caller must verify
            // State machine trusts that caller has checked structural completeness
        }

        ArtifactState::Valid => {
            // §7.3: valid requires all three validation layers to pass
            if validation.structural != ValidationStatus::Pass {
                missing.push("Structural validation must pass".into());
            }
            if validation.relational != ValidationStatus::Pass {
                missing.push("Relational validation must pass".into());
            }
            if validation.intent != ValidationStatus::Pass {
                missing.push("Intent validation must pass".into());
            }

            // §7.2: Stale -> Valid requires revalidation
            if from == ArtifactState::Stale && !has_revalidation {
                missing.push("Stale artifacts require explicit revalidation".into());
            }
        }

        ArtifactState::Approved => {
            // §7.3: approved requires current state valid + approval event bound to current hash
            if validation.structural != ValidationStatus::Pass {
                missing.push("Structural validation must pass before approval".into());
            }
            if validation.relational != ValidationStatus::Pass {
                missing.push("Relational validation must pass before approval".into());
            }
            if validation.intent != ValidationStatus::Pass {
                missing.push("Intent validation must pass before approval".into());
            }
            if !has_current_approval {
                missing.push(
                    "Approval event bound to current version hash required".into(),
                );
            }

            // §7.2: Stale -> Approved requires revalidation AND reapproval
            if from == ArtifactState::Stale {
                if !has_revalidation {
                    missing.push("Stale artifacts require revalidation before reapproval".into());
                }
            }
        }

        ArtifactState::Stale => {
            // §7.3: stale is only valid from Approved (or Valid in edge cases)
            // The transition table already enforces Approved -> Stale
            // Stale is triggered by upstream changes, not user action
        }

        ArtifactState::Draft => {
            // Regression to draft: content was modified after completion
            // No additional preconditions beyond legality
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(TransitionError::PreconditionFailed { from, to, missing })
    }
}

fn forbidden_reason(from: ArtifactState, to: ArtifactState) -> &'static str {
    match (from, to) {
        (ArtifactState::Draft, ArtifactState::Approved) => {
            "Cannot approve a draft — must complete and validate first"
        }
        (ArtifactState::Draft, ArtifactState::Valid) => {
            "Cannot validate a draft — must complete first"
        }
        (ArtifactState::Complete, ArtifactState::Approved) => {
            "Cannot approve without validation — must validate first"
        }
        (ArtifactState::Approved, ArtifactState::Draft) => {
            "Cannot regress approved artifact to draft — mark stale first if upstream changed"
        }
        _ => "Transition not allowed by state machine rules",
    }
}

// ─── Stale Propagation ──────────────────────────────────────

/// Reasons an artifact may be marked stale.
/// §7.3: stale is triggered by upstream changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleReason {
    /// Constitution version changed (amendment applied).
    ConstitutionVersionChanged {
        old_version_id: String,
        new_version_id: String,
    },
    /// An upstream artifact's version changed, breaking a dependency.
    UpstreamArtifactChanged {
        upstream_artifact_id: String,
        upstream_artifact_type: ArtifactType,
    },
    /// A trace link this artifact depends on was removed or invalidated.
    TraceLinkInvalidated {
        trace_link_id: String,
    },
    /// A blocking or error-severity drift alarm is active against this artifact.
    DriftAlarmActive {
        drift_alarm_id: String,
    },
}

impl StaleReason {
    pub fn to_human_string(&self) -> String {
        match self {
            StaleReason::ConstitutionVersionChanged { old_version_id, new_version_id } => {
                format!(
                    "Constitution changed from version {} to {} — artifact must be reconciled",
                    old_version_id, new_version_id
                )
            }
            StaleReason::UpstreamArtifactChanged { upstream_artifact_id, upstream_artifact_type } => {
                format!(
                    "Upstream {:?} artifact ({}) changed — downstream must be revalidated",
                    upstream_artifact_type, upstream_artifact_id
                )
            }
            StaleReason::TraceLinkInvalidated { trace_link_id } => {
                format!(
                    "Trace link {} was removed or invalidated — traceability broken",
                    trace_link_id
                )
            }
            StaleReason::DriftAlarmActive { drift_alarm_id } => {
                format!(
                    "Active drift alarm {} blocks this artifact's validity",
                    drift_alarm_id
                )
            }
        }
    }
}

/// Determines which artifact types are downstream of a given artifact type.
/// §8.1: required directional relationships define the dependency graph.
///
/// When an upstream artifact changes, all downstream types must be checked
/// for staleness.
pub fn downstream_artifact_types(upstream: ArtifactType) -> &'static [ArtifactType] {
    match upstream {
        // Constitution is upstream of everything
        ArtifactType::Constitution => &[
            ArtifactType::UserFantasyWorkflows,
            ArtifactType::FeatureMap,
            ArtifactType::SystemArchitecture,
            ArtifactType::UxStateMap,
            ArtifactType::PhaseRoadmapContracts,
            ArtifactType::AcceptanceChecklists,
            ArtifactType::DriftAlarmDefinitions,
            ArtifactType::ExecutionReadinessGate,
        ],
        // Workflows derive from constitution, features derive from workflows
        ArtifactType::UserFantasyWorkflows => &[
            ArtifactType::FeatureMap,
            ArtifactType::UxStateMap,
            ArtifactType::PhaseRoadmapContracts,
            ArtifactType::AcceptanceChecklists,
            ArtifactType::ExecutionReadinessGate,
        ],
        // Features are implemented by systems
        ArtifactType::FeatureMap => &[
            ArtifactType::SystemArchitecture,
            ArtifactType::UxStateMap,
            ArtifactType::PhaseRoadmapContracts,
            ArtifactType::AcceptanceChecklists,
            ArtifactType::ExecutionReadinessGate,
        ],
        // Systems feed into phases and UX
        ArtifactType::SystemArchitecture => &[
            ArtifactType::UxStateMap,
            ArtifactType::PhaseRoadmapContracts,
            ArtifactType::AcceptanceChecklists,
            ArtifactType::ExecutionReadinessGate,
        ],
        // UX states feed into phases
        ArtifactType::UxStateMap => &[
            ArtifactType::PhaseRoadmapContracts,
            ArtifactType::AcceptanceChecklists,
            ArtifactType::ExecutionReadinessGate,
        ],
        // Phase contracts feed into acceptance checklists
        ArtifactType::PhaseRoadmapContracts => &[
            ArtifactType::AcceptanceChecklists,
            ArtifactType::ExecutionReadinessGate,
        ],
        // Acceptance checklists feed into gate
        ArtifactType::AcceptanceChecklists => &[
            ArtifactType::ExecutionReadinessGate,
        ],
        // Drift alarm definitions feed into gate
        ArtifactType::DriftAlarmDefinitions => &[
            ArtifactType::ExecutionReadinessGate,
        ],
        // Gate is terminal — nothing downstream
        ArtifactType::ExecutionReadinessGate => &[],
    }
}

/// Compute which artifacts in a project should be marked stale
/// after a constitution amendment is applied.
///
/// §9.2: all downstream artifacts of the constitution must be marked stale.
/// This is the nuclear option — an amendment invalidates everything.
pub fn compute_stale_from_constitution_change(
    artifacts: &[Artifact],
    old_constitution_version_id: &str,
    new_constitution_version_id: &str,
) -> Vec<(String, StaleReason)> {
    let mut stale_list = Vec::new();
    let downstream = downstream_artifact_types(ArtifactType::Constitution);

    for artifact in artifacts {
        // Only mark approved or valid artifacts as stale
        // Draft and complete artifacts are already untrustworthy
        if !matches!(artifact.state, ArtifactState::Approved | ArtifactState::Valid) {
            continue;
        }

        if downstream.contains(&artifact.artifact_type) {
            stale_list.push((
                artifact.id.clone(),
                StaleReason::ConstitutionVersionChanged {
                    old_version_id: old_constitution_version_id.to_string(),
                    new_version_id: new_constitution_version_id.to_string(),
                },
            ));
        }
    }

    stale_list
}

/// Compute which artifacts should be marked stale after an upstream
/// artifact version change.
///
/// §7.3: upstream version change breaks downstream dependencies.
pub fn compute_stale_from_upstream_change(
    artifacts: &[Artifact],
    changed_artifact: &Artifact,
) -> Vec<(String, StaleReason)> {
    let mut stale_list = Vec::new();
    let downstream = downstream_artifact_types(changed_artifact.artifact_type);

    for artifact in artifacts {
        if artifact.id == changed_artifact.id {
            continue;
        }

        if !matches!(artifact.state, ArtifactState::Approved | ArtifactState::Valid) {
            continue;
        }

        if downstream.contains(&artifact.artifact_type) {
            stale_list.push((
                artifact.id.clone(),
                StaleReason::UpstreamArtifactChanged {
                    upstream_artifact_id: changed_artifact.id.clone(),
                    upstream_artifact_type: changed_artifact.artifact_type,
                },
            ));
        }
    }

    stale_list
}

// ─── Execution Readiness Gate ───────────────────────────────

/// §13. Gate blocking conditions — computed, not authored.
/// Returns a list of blocking reasons if the gate should be blocked.
pub fn compute_gate_blocking_reasons(
    artifacts: &[Artifact],
    active_drift_alarms: &[(String, crate::domain::DriftAlarmSeverity)],
    active_amendment_completed: bool,
    all_approvals_current: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();

    for artifact in artifacts {
        // Gate artifact itself is excluded from these checks
        if artifact.artifact_type == ArtifactType::ExecutionReadinessGate {
            continue;
        }

        match artifact.state {
            ArtifactState::Draft => {
                reasons.push(format!(
                    "Artifact '{}' ({:?}) is still in draft state",
                    artifact.title, artifact.artifact_type
                ));
            }
            ArtifactState::Complete => {
                reasons.push(format!(
                    "Artifact '{}' ({:?}) is complete but not validated",
                    artifact.title, artifact.artifact_type
                ));
            }
            ArtifactState::Valid => {
                reasons.push(format!(
                    "Artifact '{}' ({:?}) is validated but not approved",
                    artifact.title, artifact.artifact_type
                ));
            }
            ArtifactState::Stale => {
                reasons.push(format!(
                    "Artifact '{}' ({:?}) is stale — {}",
                    artifact.title,
                    artifact.artifact_type,
                    artifact.stale_reason.as_deref().unwrap_or("upstream change")
                ));
            }
            ArtifactState::Approved => {
                // Good — this is the required state
            }
        }
    }

    // Check drift alarms
    for (alarm_id, severity) in active_drift_alarms {
        if matches!(severity, DriftAlarmSeverity::Blocking | DriftAlarmSeverity::Error) {
            reasons.push(format!(
                "Active {:?}-severity drift alarm: {}",
                severity, alarm_id
            ));
        }
    }

    // Check amendment status
    if !active_amendment_completed {
        reasons.push("Active amendment has not been completed — all affected artifacts must be reconciled".into());
    }

    // Check approval currency
    if !all_approvals_current {
        reasons.push("One or more approvals bind to an outdated constitution version".into());
    }

    reasons
}

/// Compute the gate status from blocking reasons.
pub fn compute_gate_status(blocking_reasons: &[String]) -> GateStatus {
    if blocking_reasons.is_empty() {
        GateStatus::Ready
    } else {
        GateStatus::Blocked
    }
}

// ─── Artifact Ordering ──────────────────────────────────────

/// The canonical ordering of artifacts in the spine.
/// This defines the wizard progression sequence.
/// §2.2: artifacts must be worked in this order for approval.
pub fn artifact_spine_order(artifact_type: ArtifactType) -> u8 {
    match artifact_type {
        ArtifactType::Constitution => 0,
        ArtifactType::UserFantasyWorkflows => 1,
        ArtifactType::FeatureMap => 2,
        ArtifactType::SystemArchitecture => 3,
        ArtifactType::UxStateMap => 4,
        ArtifactType::PhaseRoadmapContracts => 5,
        ArtifactType::AcceptanceChecklists => 6,
        ArtifactType::DriftAlarmDefinitions => 7,
        ArtifactType::ExecutionReadinessGate => 8,
    }
}

/// Check if an artifact type can be approved given the current
/// state of prior artifacts in the spine.
///
/// UX rule: strict progression for approval, but read-ahead allowed.
pub fn can_approve_at_position(
    target_type: ArtifactType,
    artifacts: &[Artifact],
) -> Result<(), Vec<String>> {
    let target_order = artifact_spine_order(target_type);
    let mut blockers = Vec::new();

    for artifact in artifacts {
        let order = artifact_spine_order(artifact.artifact_type);
        if order < target_order && artifact.state != ArtifactState::Approved {
            blockers.push(format!(
                "{:?} must be approved before {:?} (currently {:?})",
                artifact.artifact_type, target_type, artifact.state
            ));
        }
    }

    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ValidationStatus, ValidationSummary};

    fn make_artifact(artifact_type: ArtifactType, state: ArtifactState) -> Artifact {
        Artifact {
            id: format!("{:?}-1", artifact_type),
            project_id: "proj-1".into(),
            artifact_type,
            title: format!("{:?}", artifact_type),
            current_version_id: "v1".into(),
            state,
            validation_summary: ValidationSummary::default(),
            latest_approval_id: None,
            stale_reason: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn passing_validation() -> ValidationSummary {
        ValidationSummary {
            structural: ValidationStatus::Pass,
            relational: ValidationStatus::Pass,
            intent: ValidationStatus::Pass,
            last_validated_at: Some("2026-03-13T00:00:00Z".into()),
        }
    }

    // ─── Transition legality ────────────────────────────────

    #[test]
    fn draft_to_complete_is_legal() {
        assert!(is_legal_transition(ArtifactState::Draft, ArtifactState::Complete));
    }

    #[test]
    fn draft_to_approved_is_forbidden() {
        assert!(!is_legal_transition(ArtifactState::Draft, ArtifactState::Approved));
    }

    #[test]
    fn draft_to_valid_is_forbidden() {
        assert!(!is_legal_transition(ArtifactState::Draft, ArtifactState::Valid));
    }

    #[test]
    fn complete_to_approved_is_forbidden() {
        assert!(!is_legal_transition(ArtifactState::Complete, ArtifactState::Approved));
    }

    #[test]
    fn approved_to_draft_is_forbidden() {
        assert!(!is_legal_transition(ArtifactState::Approved, ArtifactState::Draft));
    }

    #[test]
    fn valid_to_approved_is_legal() {
        assert!(is_legal_transition(ArtifactState::Valid, ArtifactState::Approved));
    }

    #[test]
    fn approved_to_stale_is_legal() {
        assert!(is_legal_transition(ArtifactState::Approved, ArtifactState::Stale));
    }

    #[test]
    fn noop_transition_is_not_legal() {
        assert!(!is_legal_transition(ArtifactState::Draft, ArtifactState::Draft));
    }

    // ─── Transition validation with preconditions ───────────

    #[test]
    fn valid_requires_all_validation_layers_pass() {
        let artifact = make_artifact(ArtifactType::FeatureMap, ArtifactState::Complete);
        let bad_validation = ValidationSummary {
            structural: ValidationStatus::Pass,
            relational: ValidationStatus::Fail,
            intent: ValidationStatus::Pass,
            last_validated_at: None,
        };

        let result = validate_transition(
            &artifact,
            ArtifactState::Valid,
            &bad_validation,
            false,
            false,
        );

        assert!(result.is_err());
        if let Err(TransitionError::PreconditionFailed { missing, .. }) = result {
            assert!(missing.iter().any(|m| m.contains("Relational")));
        }
    }

    #[test]
    fn approved_requires_approval_event() {
        let artifact = make_artifact(ArtifactType::FeatureMap, ArtifactState::Valid);
        let validation = passing_validation();

        let result = validate_transition(
            &artifact,
            ArtifactState::Approved,
            &validation,
            false, // no approval
            false,
        );

        assert!(result.is_err());
    }

    #[test]
    fn approved_succeeds_with_all_preconditions() {
        let artifact = make_artifact(ArtifactType::FeatureMap, ArtifactState::Valid);
        let validation = passing_validation();

        let result = validate_transition(
            &artifact,
            ArtifactState::Approved,
            &validation,
            true,
            false,
        );

        assert!(result.is_ok());
    }

    // ─── Stale propagation ──────────────────────────────────

    #[test]
    fn constitution_change_marks_all_downstream_stale() {
        let artifacts = vec![
            make_artifact(ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact(ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact(ArtifactType::SystemArchitecture, ArtifactState::Draft),
        ];

        let stale = compute_stale_from_constitution_change(&artifacts, "v1", "v2");

        // Draft artifact should NOT be marked stale (already untrustworthy)
        assert_eq!(stale.len(), 2);
    }

    #[test]
    fn upstream_change_only_marks_downstream_types() {
        let changed = make_artifact(ArtifactType::FeatureMap, ArtifactState::Approved);
        let artifacts = vec![
            make_artifact(ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact(ArtifactType::SystemArchitecture, ArtifactState::Approved),
            make_artifact(ArtifactType::PhaseRoadmapContracts, ArtifactState::Approved),
        ];

        let stale = compute_stale_from_upstream_change(&artifacts, &changed);

        // Workflows is NOT downstream of FeatureMap
        // SystemArchitecture and PhaseRoadmapContracts ARE downstream
        assert_eq!(stale.len(), 2);
        let ids: Vec<&str> = stale.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"SystemArchitecture-1"));
        assert!(ids.contains(&"PhaseRoadmapContracts-1"));
    }

    // ─── Gate computation ───────────────────────────────────

    #[test]
    fn gate_blocks_on_draft_artifact() {
        let artifacts = vec![
            make_artifact(ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact(ArtifactType::FeatureMap, ArtifactState::Draft),
        ];

        let reasons = compute_gate_blocking_reasons(&artifacts, &[], true, true);
        assert!(!reasons.is_empty());
        assert_eq!(compute_gate_status(&reasons), GateStatus::Blocked);
    }

    #[test]
    fn gate_blocks_on_active_blocking_alarm() {
        let artifacts = vec![
            make_artifact(ArtifactType::Constitution, ArtifactState::Approved),
        ];
        let alarms = vec![("alarm-1".to_string(), DriftAlarmSeverity::Blocking)];

        let reasons = compute_gate_blocking_reasons(&artifacts, &alarms, true, true);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn gate_ready_when_all_approved_and_clean() {
        let artifacts = vec![
            make_artifact(ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact(ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact(ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact(ArtifactType::SystemArchitecture, ArtifactState::Approved),
            make_artifact(ArtifactType::UxStateMap, ArtifactState::Approved),
            make_artifact(ArtifactType::PhaseRoadmapContracts, ArtifactState::Approved),
            make_artifact(ArtifactType::AcceptanceChecklists, ArtifactState::Approved),
            make_artifact(ArtifactType::DriftAlarmDefinitions, ArtifactState::Approved),
            make_artifact(ArtifactType::ExecutionReadinessGate, ArtifactState::Approved),
        ];

        let reasons = compute_gate_blocking_reasons(&artifacts, &[], true, true);
        assert!(reasons.is_empty());
        assert_eq!(compute_gate_status(&reasons), GateStatus::Ready);
    }

    // ─── Spine ordering ─────────────────────────────────────

    #[test]
    fn cannot_approve_feature_map_before_workflows() {
        let artifacts = vec![
            make_artifact(ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact(ArtifactType::UserFantasyWorkflows, ArtifactState::Valid),
        ];

        let result = can_approve_at_position(ArtifactType::FeatureMap, &artifacts);
        assert!(result.is_err());
    }

    #[test]
    fn can_approve_workflows_after_constitution() {
        let artifacts = vec![
            make_artifact(ArtifactType::Constitution, ArtifactState::Approved),
        ];

        let result = can_approve_at_position(ArtifactType::UserFantasyWorkflows, &artifacts);
        assert!(result.is_ok());
    }
}

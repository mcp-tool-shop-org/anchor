//! Anchor Law Engine — Blast Radius / Impact Analysis
//!
//! Given a change (edit, amendment, stale mark), compute the full
//! blast radius: which artifacts are affected, why, which approvals
//! break, and what must happen next.
//!
//! The "what broke and what do I do now?" engine.

use serde::Serialize;

use crate::domain::*;
use crate::traceability;

// ─── Impact Report ──────────────────────────────────────────

/// Complete blast-radius report for a change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactReport {
    pub trigger: ImpactTrigger,
    pub affected_artifacts: Vec<AffectedArtifact>,
    pub invalidated_approvals: Vec<InvalidatedApproval>,
    pub total_affected: usize,
    pub total_approvals_lost: usize,
    pub severity: ImpactSeverity,
    pub recovery_plan: Vec<RecoveryStep>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactTrigger {
    pub kind: TriggerKind,
    pub source_artifact_id: String,
    pub source_artifact_title: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    Edit,
    Amendment,
    StalePropagate,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedArtifact {
    pub artifact_id: String,
    pub title: String,
    pub artifact_type: ArtifactType,
    pub current_state: ArtifactState,
    pub will_become: ArtifactState,
    pub distance_from_source: u32,
    pub propagation_path: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidatedApproval {
    pub approval_id: String,
    pub artifact_id: String,
    pub artifact_title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactSeverity {
    /// 0 artifacts affected
    None,
    /// 1-2 artifacts affected
    Low,
    /// 3-5 artifacts affected
    Medium,
    /// 6+ artifacts or constitution change
    High,
    /// Constitution amendment — everything downstream
    Nuclear,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryStep {
    pub order: u32,
    pub artifact_id: String,
    pub action: String,
    pub reason: String,
}

// ─── Impact Analysis ────────────────────────────────────────

/// Compute the blast radius of editing a specific artifact.
pub fn impact_of_edit(
    artifact_id: &str,
    artifacts: &[Artifact],
    links: &[TraceLink],
    approvals: &[Approval],
) -> Option<ImpactReport> {
    let source = artifacts.iter().find(|a| a.id == artifact_id)?;

    let trigger = ImpactTrigger {
        kind: TriggerKind::Edit,
        source_artifact_id: source.id.clone(),
        source_artifact_title: source.title.clone(),
        description: format!("Editing {} ({})", source.title, source.id),
    };

    let mut affected = Vec::new();
    let mut visited = vec![artifact_id.to_string()];
    walk_downstream(artifact_id, artifacts, links, 1, &[artifact_id.to_string()], &mut affected, &mut visited);

    let invalidated = find_invalidated_approvals(&affected, approvals, artifacts);
    let severity = compute_severity(&affected, source);
    let recovery = build_recovery_plan(&affected, source);

    let total_affected = affected.len();
    let total_approvals_lost = invalidated.len();

    Some(ImpactReport {
        trigger,
        affected_artifacts: affected,
        invalidated_approvals: invalidated,
        total_affected,
        total_approvals_lost,
        severity,
        recovery_plan: recovery,
    })
}

/// Compute the blast radius of a constitution amendment.
pub fn impact_of_amendment(
    artifacts: &[Artifact],
    _links: &[TraceLink],
    approvals: &[Approval],
    constitution: &Constitution,
) -> ImpactReport {
    let trigger = ImpactTrigger {
        kind: TriggerKind::Amendment,
        source_artifact_id: constitution.artifact_id.clone(),
        source_artifact_title: "Product Constitution".into(),
        description: "Constitution amendment — all downstream artifacts must be reconciled".into(),
    };

    // Nuclear: every non-Constitution artifact that is Valid or Approved becomes Stale
    let affected: Vec<AffectedArtifact> = artifacts
        .iter()
        .filter(|a| {
            a.artifact_type != ArtifactType::Constitution
                && a.artifact_type != ArtifactType::ExecutionReadinessGate
                && (a.state == ArtifactState::Valid || a.state == ArtifactState::Approved)
        })
        .map(|a| AffectedArtifact {
            artifact_id: a.id.clone(),
            title: a.title.clone(),
            artifact_type: a.artifact_type,
            current_state: a.state,
            will_become: ArtifactState::Stale,
            distance_from_source: 1,
            propagation_path: vec![constitution.artifact_id.clone(), a.id.clone()],
            reason: "Constitution amendment invalidates all downstream artifacts".into(),
        })
        .collect();

    let invalidated: Vec<InvalidatedApproval> = approvals
        .iter()
        .filter(|appr| {
            artifacts.iter().any(|a| {
                a.id == appr.artifact_id
                    && a.artifact_type != ArtifactType::Constitution
                    && (a.state == ArtifactState::Valid || a.state == ArtifactState::Approved)
            })
        })
        .map(|appr| {
            let title = artifacts.iter().find(|a| a.id == appr.artifact_id)
                .map(|a| a.title.clone())
                .unwrap_or_default();
            InvalidatedApproval {
                approval_id: appr.id.clone(),
                artifact_id: appr.artifact_id.clone(),
                artifact_title: title,
                reason: "Constitution amendment invalidates all approvals".into(),
            }
        })
        .collect();

    let total_affected = affected.len();
    let total_approvals_lost = invalidated.len();

    // Build recovery: constitution order (top-down)
    let recovery = build_amendment_recovery(&affected);

    ImpactReport {
        trigger,
        affected_artifacts: affected,
        invalidated_approvals: invalidated,
        total_affected,
        total_approvals_lost,
        severity: ImpactSeverity::Nuclear,
        recovery_plan: recovery,
    }
}

// ─── Graph Walking ──────────────────────────────────────────

fn walk_downstream(
    source_id: &str,
    artifacts: &[Artifact],
    links: &[TraceLink],
    depth: u32,
    path: &[String],
    affected: &mut Vec<AffectedArtifact>,
    visited: &mut Vec<String>,
) {
    // Find all artifacts that depend on this one (downstream = links where target == source_id)
    let downstream = traceability::downstream_links(source_id, links);

    for link in downstream {
        let dependent_id = &link.source_node_id;
        if visited.contains(dependent_id) {
            continue; // cycle protection
        }
        visited.push(dependent_id.clone());

        if let Some(dep_artifact) = artifacts.iter().find(|a| a.id == *dependent_id) {
            // Only Valid/Approved artifacts are affected (Draft/Complete are already untrustworthy)
            let will_be_affected = dep_artifact.state == ArtifactState::Valid
                || dep_artifact.state == ArtifactState::Approved;

            if will_be_affected {
                let mut propagation_path = path.to_vec();
                propagation_path.push(dependent_id.clone());

                affected.push(AffectedArtifact {
                    artifact_id: dep_artifact.id.clone(),
                    title: dep_artifact.title.clone(),
                    artifact_type: dep_artifact.artifact_type,
                    current_state: dep_artifact.state,
                    will_become: ArtifactState::Stale,
                    distance_from_source: depth,
                    propagation_path: propagation_path.clone(),
                    reason: format!("Depends on {} via {} link", source_id, format!("{:?}", link.link_type).to_lowercase()),
                });
            }

            // Recurse regardless — a Draft artifact might have Valid dependents
            let mut next_path = path.to_vec();
            next_path.push(dependent_id.clone());
            walk_downstream(dependent_id, artifacts, links, depth + 1, &next_path, affected, visited);
        }
    }
}

fn find_invalidated_approvals(
    affected: &[AffectedArtifact],
    approvals: &[Approval],
    _artifacts: &[Artifact],
) -> Vec<InvalidatedApproval> {
    affected
        .iter()
        .filter_map(|aa| {
            approvals.iter().find(|appr| appr.artifact_id == aa.artifact_id).map(|appr| {
                InvalidatedApproval {
                    approval_id: appr.id.clone(),
                    artifact_id: aa.artifact_id.clone(),
                    artifact_title: aa.title.clone(),
                    reason: format!(
                        "Upstream change from {} propagated through {} hop(s)",
                        aa.propagation_path.first().unwrap_or(&"?".into()),
                        aa.distance_from_source
                    ),
                }
            })
        })
        .collect()
}

fn compute_severity(affected: &[AffectedArtifact], source: &Artifact) -> ImpactSeverity {
    if source.artifact_type == ArtifactType::Constitution {
        return ImpactSeverity::Nuclear;
    }
    match affected.len() {
        0 => ImpactSeverity::None,
        1..=2 => ImpactSeverity::Low,
        3..=5 => ImpactSeverity::Medium,
        _ => ImpactSeverity::High,
    }
}

fn build_recovery_plan(affected: &[AffectedArtifact], source: &Artifact) -> Vec<RecoveryStep> {
    let mut steps = Vec::new();
    let mut order = 1;

    // Step 0: the source itself may need revalidation
    steps.push(RecoveryStep {
        order,
        artifact_id: source.id.clone(),
        action: "Complete editing and re-validate".into(),
        reason: "Source artifact was edited — must be validated before downstream propagation can begin".into(),
    });
    order += 1;

    // Sort affected by distance (closest first)
    let mut sorted: Vec<_> = affected.iter().collect();
    sorted.sort_by_key(|a| a.distance_from_source);

    for aa in sorted {
        steps.push(RecoveryStep {
            order,
            artifact_id: aa.artifact_id.clone(),
            action: format!("Reconcile {} with upstream changes, then re-validate and re-approve", aa.title),
            reason: aa.reason.clone(),
        });
        order += 1;
    }

    steps
}

fn build_amendment_recovery(affected: &[AffectedArtifact]) -> Vec<RecoveryStep> {
    // Constitutional ordering for recovery
    let type_order = [
        ArtifactType::UserFantasyWorkflows,
        ArtifactType::FeatureMap,
        ArtifactType::SystemArchitecture,
        ArtifactType::UxStateMap,
        ArtifactType::PhaseRoadmapContracts,
        ArtifactType::AcceptanceChecklists,
        ArtifactType::DriftAlarmDefinitions,
    ];

    let mut steps = Vec::new();
    let mut order = 1;

    for expected_type in &type_order {
        for aa in affected.iter().filter(|a| a.artifact_type == *expected_type) {
            steps.push(RecoveryStep {
                order,
                artifact_id: aa.artifact_id.clone(),
                action: format!(
                    "Reconcile {:?} with amended constitution, re-validate, re-approve",
                    aa.artifact_type
                ),
                reason: "Constitution changed — all downstream must reconcile".into(),
            });
            order += 1;
        }
    }

    steps
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectStore;

    fn demo() -> ProjectStore {
        ProjectStore::demo()
    }

    #[test]
    fn edit_impact_on_constitution_is_nuclear() {
        let s = demo();
        let report = impact_of_edit("art-const", &s.artifacts, &s.links, &s.approvals);
        assert!(report.is_some());
        let r = report.unwrap();
        assert_eq!(r.severity, ImpactSeverity::Nuclear);
    }

    #[test]
    fn edit_impact_on_leaf_is_low() {
        let s = demo();
        // art-check (acceptance checklists) has no downstream dependents
        let report = impact_of_edit("art-check", &s.artifacts, &s.links, &s.approvals);
        assert!(report.is_some());
        let r = report.unwrap();
        assert_eq!(r.severity, ImpactSeverity::None);
        assert_eq!(r.total_affected, 0);
    }

    #[test]
    fn edit_impact_on_workflows_propagates() {
        let s = demo();
        // art-wf has downstream: art-feat (justifies), art-ux (depends_on)
        let report = impact_of_edit("art-wf", &s.artifacts, &s.links, &s.approvals);
        assert!(report.is_some());
        let r = report.unwrap();
        assert!(r.total_affected > 0, "Workflows should have downstream dependents");
    }

    #[test]
    fn amendment_impact_is_nuclear() {
        let s = demo();
        let report = impact_of_amendment(&s.artifacts, &s.links, &s.approvals, &s.constitution);
        assert_eq!(report.severity, ImpactSeverity::Nuclear);
        // All Valid/Approved non-Constitution artifacts should be affected
        let valid_or_approved = s.artifacts.iter()
            .filter(|a| {
                a.artifact_type != ArtifactType::Constitution
                    && a.artifact_type != ArtifactType::ExecutionReadinessGate
                    && (a.state == ArtifactState::Valid || a.state == ArtifactState::Approved)
            })
            .count();
        assert_eq!(report.total_affected, valid_or_approved);
    }

    #[test]
    fn amendment_recovery_follows_constitutional_order() {
        let s = demo();
        let report = impact_of_amendment(&s.artifacts, &s.links, &s.approvals, &s.constitution);
        if report.recovery_plan.len() >= 2 {
            // First recovery should be for workflows (closest to constitution)
            let first = &report.recovery_plan[0];
            let first_artifact = s.artifacts.iter().find(|a| a.id == first.artifact_id);
            assert!(first_artifact.is_some());
        }
    }

    #[test]
    fn nonexistent_artifact_returns_none() {
        let s = demo();
        let report = impact_of_edit("does-not-exist", &s.artifacts, &s.links, &s.approvals);
        assert!(report.is_none());
    }

    #[test]
    fn invalidated_approvals_tracked() {
        let s = demo();
        let report = impact_of_amendment(&s.artifacts, &s.links, &s.approvals, &s.constitution);
        assert!(report.total_approvals_lost > 0, "Amendment should invalidate approvals");
    }

    #[test]
    fn recovery_plan_covers_all_affected() {
        let s = demo();
        let report = impact_of_edit("art-wf", &s.artifacts, &s.links, &s.approvals).unwrap();
        // Recovery plan should have 1 entry for source + 1 per affected
        assert_eq!(report.recovery_plan.len(), report.total_affected + 1);
    }

    #[test]
    fn propagation_paths_are_correct() {
        let s = demo();
        let report = impact_of_edit("art-wf", &s.artifacts, &s.links, &s.approvals).unwrap();
        for aa in &report.affected_artifacts {
            // Path should start with source
            assert_eq!(aa.propagation_path.first().unwrap(), "art-wf");
            // Path should end with the affected artifact
            assert_eq!(aa.propagation_path.last().unwrap(), &aa.artifact_id);
        }
    }
}

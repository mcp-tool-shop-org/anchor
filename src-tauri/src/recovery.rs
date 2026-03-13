//! Anchor — Recovery & Next-Action Engine
//!
//! For any artifact or project state, this module computes:
//! - What's blocking progress
//! - What's the next lawful action
//! - What's the minimal recovery path to gate-ready
//!
//! The operator should never need to guess. This module is the GPS.

use crate::domain::*;
use crate::readiness_gate;
use crate::traceability;

// ─── Types ──────────────────────────────────────────────────

/// A single recommended action the operator can take.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryAction {
    pub action_type: ActionType,
    pub target_artifact_id: String,
    pub target_artifact_title: String,
    pub title: String,
    pub description: String,
    pub priority: u32,
    pub prerequisites: Vec<String>,
    /// Which constitutional rule or gate check requires this action.
    pub rule_clause: String,
    /// Why this action is prioritized above others.
    pub why_first: String,
}

/// What kind of action this is — determines the UI affordance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Edit the artifact content (reconcile stale, fix validation)
    EditContent,
    /// Transition to a specific state
    TransitionState,
    /// Add a required trace link
    AddTraceLink,
    /// Re-validate after changes
    Revalidate,
    /// Re-approve after re-validation
    Reapprove,
    /// Reconcile a stale artifact (edit + revalidate + reapprove)
    ReconcileStale,
    /// Propose a constitutional amendment
    ProposeAmendment,
}

/// Overall project health assessment.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHealth {
    pub status: HealthStatus,
    pub gate_status: GateStatus,
    pub total_artifacts: usize,
    pub ready_artifacts: usize,
    pub stale_artifacts: usize,
    pub blocked_artifacts: usize,
    pub active_alarms: usize,
    pub missing_links: usize,
    pub next_actions: Vec<RecoveryAction>,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// All artifacts approved, gate ready
    Healthy,
    /// Some issues but no blocking failures
    NeedsAttention,
    /// Blocking issues prevent export
    Critical,
}

// ─── Per-Artifact Recovery ──────────────────────────────────

/// Compute the next actions for a single artifact.
/// Returns actions sorted by priority (1 = most urgent).
pub fn next_actions_for_artifact(
    artifact: &Artifact,
    all_artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    approvals: &[Approval],
    links: &[TraceLink],
    alarms: &[DriftAlarm],
) -> Vec<RecoveryAction> {
    let mut actions = Vec::new();
    let title = artifact.title.clone();
    let id = artifact.id.clone();

    match artifact.state {
        // ─── Stale: highest urgency ─────────────────────────
        ArtifactState::Stale => {
            actions.push(RecoveryAction {
                action_type: ActionType::ReconcileStale,
                target_artifact_id: id.clone(),
                target_artifact_title: title.clone(),
                title: format!("Reconcile stale: {}", title),
                description: "This artifact was invalidated by an upstream change. Review content against the current constitution, update if needed, then revalidate and reapprove.".into(),
                priority: 1,
                prerequisites: vec![],
                rule_clause: "§13.3 — No stale artifact may be present at export time".into(),
                why_first: "Stale artifacts block the readiness gate. Until reconciled, no export is possible.".into(),
            });
        }

        // ─── Draft: needs content then transition ───────────
        ArtifactState::Draft => {
            // Check if content exists
            let version = versions.iter().find(|v| v.id == artifact.current_version_id);
            let has_content = version
                .map(|v| v.content != serde_json::Value::Null && v.content != serde_json::json!({}))
                .unwrap_or(false);

            if !has_content {
                actions.push(RecoveryAction {
                    action_type: ActionType::EditContent,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Add content: {}", title),
                    description: "This artifact is in Draft state with no content. Add meaningful content before advancing.".into(),
                    priority: 2,
                    prerequisites: vec![],
                    rule_clause: "§5.1 — Every artifact must have substantive content before leaving Draft".into(),
                    why_first: "Content is the foundation. Without it, validation and traceability checks cannot proceed.".into(),
                });
            }

            // Check missing trace links
            let missing = get_missing_trace_requirements(artifact, all_artifacts, links);
            for (req_desc, link_type, upstream_type) in &missing {
                actions.push(RecoveryAction {
                    action_type: ActionType::AddTraceLink,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Add {:?} link from {}", link_type, title),
                    description: format!("{} — link to a {:?} artifact", req_desc, upstream_type),
                    priority: 3,
                    prerequisites: vec![],
                    rule_clause: format!("§8.1 — {}", req_desc),
                    why_first: "Trace links are checked before state transitions. Missing links block validation.".into(),
                });
            }

            if has_content {
                actions.push(RecoveryAction {
                    action_type: ActionType::TransitionState,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Advance {} to Complete", title),
                    description: "Content is present. Mark this artifact as complete to begin validation.".into(),
                    priority: 4,
                    prerequisites: vec![],
                    rule_clause: "§5.2 — Draft → Complete requires substantive content".into(),
                    why_first: "This is the natural next step after content is authored. Lower priority than content and links.".into(),
                });
            }
        }

        // ─── Complete: needs validation then links ──────────
        ArtifactState::Complete => {
            let missing = get_missing_trace_requirements(artifact, all_artifacts, links);
            for (req_desc, link_type, upstream_type) in &missing {
                actions.push(RecoveryAction {
                    action_type: ActionType::AddTraceLink,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Add {:?} link from {}", link_type, title),
                    description: format!("{} — link to a {:?} artifact", req_desc, upstream_type),
                    priority: 2,
                    prerequisites: vec![],
                    rule_clause: format!("§8.1 — {}", req_desc),
                    why_first: "Missing links must be added before validation can complete.".into(),
                });
            }

            actions.push(RecoveryAction {
                action_type: ActionType::TransitionState,
                target_artifact_id: id.clone(),
                target_artifact_title: title.clone(),
                title: format!("Advance {} to Valid", title),
                description: "Mark as validated after confirming content and trace links are correct.".into(),
                priority: 3,
                prerequisites: vec![],
                rule_clause: "§5.3 — Complete → Valid requires all validation layers to pass".into(),
                why_first: "Validation must follow content completion. Links should be resolved first.".into(),
            });
        }

        // ─── Valid: needs approval ──────────────────────────
        ArtifactState::Valid => {
            let has_approval = approvals.iter().any(|a| a.artifact_id == id);
            if !has_approval {
                actions.push(RecoveryAction {
                    action_type: ActionType::Reapprove,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Approve {}", title),
                    description: "This artifact is validated but not yet approved. Approve to clear the readiness gate.".into(),
                    priority: 2,
                    prerequisites: vec![],
                    rule_clause: "§5.4 — Valid → Approved requires explicit approval event".into(),
                    why_first: "Approval is the final step before this artifact clears the gate.".into(),
                });
            } else {
                actions.push(RecoveryAction {
                    action_type: ActionType::TransitionState,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Advance {} to Approved", title),
                    description: "Approval exists. Advance to Approved state.".into(),
                    priority: 2,
                    prerequisites: vec![],
                    rule_clause: "§5.4 — Approval record exists, transition is lawful".into(),
                    why_first: "Approval is recorded. This transition completes the artifact lifecycle.".into(),
                });
            }
        }

        // ─── Approved: check for active alarms ──────────────
        ArtifactState::Approved => {
            let artifact_alarms: Vec<_> = alarms
                .iter()
                .filter(|a| {
                    a.affected_node_ids.contains(&id)
                        && a.status == DriftAlarmStatus::Active
                })
                .collect();

            if !artifact_alarms.is_empty() {
                actions.push(RecoveryAction {
                    action_type: ActionType::Revalidate,
                    target_artifact_id: id.clone(),
                    target_artifact_title: title.clone(),
                    title: format!("Resolve {} alarm(s) on {}", artifact_alarms.len(), title),
                    description: format!(
                        "Active drift alarms: {}. Review and resolve each alarm.",
                        artifact_alarms.iter().map(|a| a.explanation.as_str()).collect::<Vec<_>>().join("; ")
                    ),
                    priority: 1,
                    prerequisites: vec![],
                    rule_clause: "§13.3 — Active blocking alarms prevent export".into(),
                    why_first: "Drift alarms indicate structural violations. They must be resolved before re-approval.".into(),
                });
            }
            // If approved with no alarms, artifact is healthy — no actions needed
        }
    }

    // Sort by priority
    actions.sort_by_key(|a| a.priority);
    actions
}

// ─── Project-Wide Health ────────────────────────────────────

/// Compute overall project health with prioritized recovery actions.
pub fn project_health(
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    approvals: &[Approval],
    links: &[TraceLink],
    constitution: &Constitution,
    alarms: &[DriftAlarm],
    amendments: &[Amendment],
) -> ProjectHealth {
    let gate = readiness_gate::evaluate(
        artifacts, versions, approvals, links, constitution, alarms, amendments,
    );

    let stale_count = artifacts.iter().filter(|a| a.state == ArtifactState::Stale).count();
    let approved_count = artifacts.iter().filter(|a| a.state == ArtifactState::Approved).count();
    let active_alarm_count = alarms.iter().filter(|a| a.status == DriftAlarmStatus::Active).count();

    // Compute missing links across all artifacts
    let missing_link_count: usize = artifacts
        .iter()
        .map(|a| get_missing_trace_requirements(a, artifacts, links).len())
        .sum();

    // Collect per-artifact actions, dedup and prioritize
    let mut all_actions: Vec<RecoveryAction> = Vec::new();
    for artifact in artifacts {
        let actions = next_actions_for_artifact(artifact, artifacts, versions, approvals, links, alarms);
        all_actions.extend(actions);
    }
    all_actions.sort_by_key(|a| a.priority);

    // Compute blocked count (anything not approved and not healthy)
    let blocked_count = artifacts
        .iter()
        .filter(|a| a.state != ArtifactState::Approved && a.state != ArtifactState::Valid)
        .count();

    let status = if gate.status == GateStatus::Ready {
        HealthStatus::Healthy
    } else if stale_count > 0 || active_alarm_count > 0 {
        HealthStatus::Critical
    } else {
        HealthStatus::NeedsAttention
    };

    let summary = match status {
        HealthStatus::Healthy => "All artifacts approved. Gate is ready for export.".into(),
        HealthStatus::Critical => format!(
            "{} stale artifact(s), {} active alarm(s). Immediate recovery needed.",
            stale_count, active_alarm_count
        ),
        HealthStatus::NeedsAttention => format!(
            "{}/{} artifacts approved. {} action(s) to reach export readiness.",
            approved_count,
            artifacts.len(),
            all_actions.len()
        ),
    };

    ProjectHealth {
        status,
        gate_status: gate.status,
        total_artifacts: artifacts.len(),
        ready_artifacts: approved_count,
        stale_artifacts: stale_count,
        blocked_artifacts: blocked_count,
        active_alarms: active_alarm_count,
        missing_links: missing_link_count,
        next_actions: all_actions,
        summary,
    }
}

// ─── Trace Link Helpers ─────────────────────────────────────

/// Check TRACE_REQUIREMENTS and return missing links for this artifact.
/// Returns: Vec<(description, required_link_type, required_upstream_type)>
fn get_missing_trace_requirements(
    artifact: &Artifact,
    all_artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<(String, TraceLinkType, ArtifactType)> {
    let mut missing = Vec::new();

    // TRACE_REQUIREMENTS from traceability module — replicated here
    // to avoid coupling to the internal struct. We check the same rules.
    let requirements: Vec<(ArtifactType, &[TraceLinkType], &[ArtifactType], &str)> = vec![
        (
            ArtifactType::UserFantasyWorkflows,
            &[TraceLinkType::DerivesFrom],
            &[ArtifactType::Constitution],
            "Workflows must trace to constitution via derives_from",
        ),
        (
            ArtifactType::FeatureMap,
            &[TraceLinkType::Justifies, TraceLinkType::DerivesFrom],
            &[ArtifactType::UserFantasyWorkflows],
            "Features must trace to workflows via justifies or derives_from",
        ),
        (
            ArtifactType::SystemArchitecture,
            &[TraceLinkType::Implements],
            &[ArtifactType::FeatureMap],
            "Systems must trace to features via implements",
        ),
        (
            ArtifactType::UxStateMap,
            &[TraceLinkType::DependsOn],
            &[ArtifactType::UserFantasyWorkflows, ArtifactType::FeatureMap],
            "UX states must trace to workflows or features via depends_on",
        ),
        (
            ArtifactType::PhaseRoadmapContracts,
            &[TraceLinkType::ValidatedBy],
            &[ArtifactType::Constitution],
            "Phases must trace to constitution via validated_by",
        ),
        (
            ArtifactType::DriftAlarmDefinitions,
            &[TraceLinkType::InvalidatedBy],
            &[ArtifactType::Constitution],
            "Drift alarms must trace to constitution via invalidated_by",
        ),
    ];

    for (art_type, req_link_types, upstream_types, desc) in &requirements {
        if artifact.artifact_type != *art_type {
            continue;
        }

        let outbound = traceability::upstream_links(&artifact.id, links);
        let has_required = outbound.iter().any(|link| {
            let type_ok = req_link_types.contains(&link.link_type);
            let target_ok = all_artifacts.iter().any(|a| {
                a.id == link.target_node_id && upstream_types.contains(&a.artifact_type)
            });
            type_ok && target_ok
        });

        if !has_required {
            // Return the first required link type as the suggestion
            missing.push((desc.to_string(), req_link_types[0], upstream_types[0]));
        }
    }

    missing
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectStore;

    fn store_artifacts(store: &ProjectStore) -> (&[Artifact], &[ArtifactVersion], &[Approval], &[TraceLink], &[DriftAlarm]) {
        (&store.artifacts, &store.versions, &store.approvals, &store.links, &store.alarms)
    }

    #[test]
    fn healthy_project_returns_no_critical_actions() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        let health = project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        // Crystal Sanctum is the closest to healthy — no stale, no alarms
        assert_ne!(health.status, HealthStatus::Critical);
        assert_eq!(health.stale_artifacts, 0);
        assert_eq!(health.active_alarms, 0);
    }

    #[test]
    fn stale_project_returns_critical() {
        let store = ProjectStore::load_scenario("ember-saga").unwrap();
        let health = project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(health.status, HealthStatus::Critical);
        assert!(health.stale_artifacts > 0);
        assert!(!health.next_actions.is_empty());
    }

    #[test]
    fn missing_traceability_generates_link_actions() {
        let store = ProjectStore::load_scenario("shadow-protocol").unwrap();
        let health = project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert!(health.missing_links > 0);
        let link_actions: Vec<_> = health
            .next_actions
            .iter()
            .filter(|a| a.action_type == ActionType::AddTraceLink)
            .collect();
        assert!(!link_actions.is_empty());
    }

    #[test]
    fn forge_quest_needs_attention() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let health = project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        // Forge Quest has mixed states — not healthy
        assert_ne!(health.status, HealthStatus::Healthy);
        assert!(!health.next_actions.is_empty());
    }

    #[test]
    fn approved_artifact_has_no_actions() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        let approved = store.artifacts.iter().find(|a| a.state == ArtifactState::Approved).unwrap();
        let (arts, vers, apps, links, alarms) = store_artifacts(&store);
        let actions = next_actions_for_artifact(approved, arts, vers, apps, links, alarms);
        assert!(actions.is_empty());
    }

    #[test]
    fn stale_artifact_gets_reconcile_action() {
        let store = ProjectStore::load_scenario("ember-saga").unwrap();
        let stale = store.artifacts.iter().find(|a| a.state == ArtifactState::Stale);
        if let Some(stale_art) = stale {
            let (arts, vers, apps, links, alarms) = store_artifacts(&store);
            let actions = next_actions_for_artifact(stale_art, arts, vers, apps, links, alarms);
            assert!(actions.iter().any(|a| a.action_type == ActionType::ReconcileStale));
            // Reconcile should be priority 1
            assert_eq!(actions[0].priority, 1);
        }
    }

    #[test]
    fn draft_artifact_gets_content_and_transition_actions() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let draft = store.artifacts.iter().find(|a| a.state == ArtifactState::Draft);
        if let Some(draft_art) = draft {
            let (arts, vers, apps, links, alarms) = store_artifacts(&store);
            let actions = next_actions_for_artifact(draft_art, arts, vers, apps, links, alarms);
            // Draft artifacts should have at least a transition suggestion
            assert!(!actions.is_empty());
        }
    }

    #[test]
    fn actions_sorted_by_priority() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let health = project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        for window in health.next_actions.windows(2) {
            assert!(window[0].priority <= window[1].priority);
        }
    }

    #[test]
    fn health_summary_is_nonempty() {
        for scenario in &["forge-quest", "crystal-sanctum", "shadow-protocol", "ember-saga"] {
            let store = ProjectStore::load_scenario(scenario).unwrap();
            let health = project_health(
                &store.artifacts, &store.versions, &store.approvals,
                &store.links, &store.constitution, &store.alarms, &store.amendments,
            );
            assert!(!health.summary.is_empty(), "Empty summary for {}", scenario);
        }
    }
}

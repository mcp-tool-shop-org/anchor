//! Anchor Law Engine — Stale Propagation
//!
//! Real dependency walk. On any upstream change:
//! 1. Identify the changed node
//! 2. Walk the traceability graph to find all dependents
//! 3. Mark each Valid/Approved dependent as Stale
//! 4. Recurse: if a newly-stale node had its own dependents, propagate further
//!
//! Constitution amendments trigger the nuclear path: everything downstream becomes Stale.

use crate::domain::{
    Artifact, ArtifactState, ArtifactType, TraceLink,
};
use crate::state_machine::downstream_artifact_types;

// ─── Stale Reasons ──────────────────────────────────────────

/// Why an artifact was marked stale.
/// Each variant carries the evidence needed for "Why blocked?" panels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropagatedStaleReason {
    /// Constitution amendment: the nuclear option.
    ConstitutionAmended {
        old_version_id: String,
        new_version_id: String,
    },
    /// A direct upstream artifact changed.
    DirectUpstreamChanged {
        upstream_artifact_id: String,
        upstream_artifact_type: ArtifactType,
    },
    /// Transitive: an upstream-of-upstream changed, propagated through the graph.
    TransitiveUpstreamChanged {
        root_cause_artifact_id: String,
        root_cause_type: ArtifactType,
        via_artifact_id: String,
        via_type: ArtifactType,
    },
    /// A trace link was removed that this artifact depended on.
    TraceLinkRemoved {
        link_id: String,
        was_target_of: String,
    },
}

impl PropagatedStaleReason {
    pub fn to_human_string(&self) -> String {
        match self {
            Self::ConstitutionAmended { old_version_id, new_version_id } => {
                format!(
                    "Constitution amended from {} to {} — all downstream artifacts must be reconciled",
                    old_version_id, new_version_id
                )
            }
            Self::DirectUpstreamChanged { upstream_artifact_id, upstream_artifact_type } => {
                format!(
                    "Direct upstream {:?} ({}) changed — must reconcile and revalidate",
                    upstream_artifact_type, upstream_artifact_id
                )
            }
            Self::TransitiveUpstreamChanged {
                root_cause_artifact_id,
                root_cause_type,
                via_artifact_id,
                via_type,
            } => {
                format!(
                    "Upstream {:?} ({}) changed, propagated through {:?} ({}) — must reconcile",
                    root_cause_type, root_cause_artifact_id, via_type, via_artifact_id
                )
            }
            Self::TraceLinkRemoved { link_id, was_target_of } => {
                format!(
                    "Trace link {} removed — {} lost its upstream justification",
                    link_id, was_target_of
                )
            }
        }
    }

    pub fn to_stale_reason_string(&self) -> String {
        self.to_human_string()
    }
}

// ─── Stale Propagation Results ──────────────────────────────

/// A single artifact that should be marked stale, with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleMark {
    pub artifact_id: String,
    pub artifact_type: ArtifactType,
    pub reason: PropagatedStaleReason,
}

// ─── Constitution Amendment Propagation ─────────────────────

/// §9.2: Constitution amendment marks ALL downstream artifacts stale.
/// This is the nuclear option. Only Valid and Approved artifacts are affected
/// (Draft and Complete are already untrustworthy).
pub fn propagate_constitution_amendment(
    artifacts: &[Artifact],
    old_constitution_version_id: &str,
    new_constitution_version_id: &str,
) -> Vec<StaleMark> {
    let downstream = downstream_artifact_types(ArtifactType::Constitution);

    artifacts
        .iter()
        .filter(|a| {
            downstream.contains(&a.artifact_type)
                && matches!(a.state, ArtifactState::Approved | ArtifactState::Valid)
        })
        .map(|a| StaleMark {
            artifact_id: a.id.clone(),
            artifact_type: a.artifact_type,
            reason: PropagatedStaleReason::ConstitutionAmended {
                old_version_id: old_constitution_version_id.to_string(),
                new_version_id: new_constitution_version_id.to_string(),
            },
        })
        .collect()
}

// ─── Upstream Artifact Change Propagation ───────────────────

/// When an artifact changes (new version created), walk the dependency graph
/// and mark all downstream Valid/Approved artifacts as Stale.
///
/// This uses both the structural type-level dependency graph AND
/// explicit trace links for a belt-and-suspenders approach.
pub fn propagate_upstream_change(
    changed_artifact: &Artifact,
    all_artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<StaleMark> {
    let mut stale_marks = Vec::new();
    let mut visited: Vec<String> = Vec::new();

    propagate_recursive(
        changed_artifact,
        changed_artifact,
        all_artifacts,
        links,
        &mut stale_marks,
        &mut visited,
        true, // first level = direct
    );

    stale_marks
}

fn propagate_recursive(
    root_cause: &Artifact,
    current: &Artifact,
    all_artifacts: &[Artifact],
    links: &[TraceLink],
    stale_marks: &mut Vec<StaleMark>,
    visited: &mut Vec<String>,
    is_direct: bool,
) {
    if visited.contains(&current.id) {
        return; // prevent cycles
    }
    visited.push(current.id.clone());

    // Find downstream artifacts via type-level dependency graph
    let type_downstream = downstream_artifact_types(current.artifact_type);

    // Find downstream artifacts via explicit trace links
    // (links where current artifact is the target = something depends on it)
    let link_dependents: Vec<&str> = links
        .iter()
        .filter(|l| l.target_node_id == current.id)
        .map(|l| l.source_node_id.as_str())
        .collect();

    for artifact in all_artifacts {
        if artifact.id == root_cause.id || artifact.id == current.id {
            continue;
        }

        // Already marked in this propagation pass
        if stale_marks.iter().any(|sm| sm.artifact_id == artifact.id) {
            continue;
        }

        // Only mark Valid or Approved artifacts
        if !matches!(artifact.state, ArtifactState::Approved | ArtifactState::Valid) {
            continue;
        }

        // Check if this artifact is downstream (by type graph or explicit link)
        let is_type_downstream = type_downstream.contains(&artifact.artifact_type);
        let is_link_dependent = link_dependents.contains(&artifact.id.as_str());

        if is_type_downstream || is_link_dependent {
            let reason = if is_direct {
                PropagatedStaleReason::DirectUpstreamChanged {
                    upstream_artifact_id: current.id.clone(),
                    upstream_artifact_type: current.artifact_type,
                }
            } else {
                PropagatedStaleReason::TransitiveUpstreamChanged {
                    root_cause_artifact_id: root_cause.id.clone(),
                    root_cause_type: root_cause.artifact_type,
                    via_artifact_id: current.id.clone(),
                    via_type: current.artifact_type,
                }
            };

            stale_marks.push(StaleMark {
                artifact_id: artifact.id.clone(),
                artifact_type: artifact.artifact_type,
                reason,
            });

            // Recurse: this newly-stale artifact's dependents may also need marking
            propagate_recursive(
                root_cause,
                artifact,
                all_artifacts,
                links,
                stale_marks,
                visited,
                false, // transitive from here on
            );
        }
    }
}

// ─── Trace Link Removal Propagation ─────────────────────────

/// When a trace link is removed, the source artifact may lose its
/// upstream justification. Mark it stale if it was Valid or Approved.
pub fn propagate_link_removal(
    removed_link: &TraceLink,
    all_artifacts: &[Artifact],
    remaining_links: &[TraceLink],
) -> Vec<StaleMark> {
    let mut stale_marks = Vec::new();

    // The source of the removed link is the artifact that was depending on the target
    if let Some(affected) = all_artifacts
        .iter()
        .find(|a| a.id == removed_link.source_node_id)
    {
        if matches!(affected.state, ArtifactState::Approved | ArtifactState::Valid) {
            // Check if the artifact still has other upstream links
            let still_has_upstream = remaining_links
                .iter()
                .any(|l| l.source_node_id == affected.id);

            if !still_has_upstream {
                stale_marks.push(StaleMark {
                    artifact_id: affected.id.clone(),
                    artifact_type: affected.artifact_type,
                    reason: PropagatedStaleReason::TraceLinkRemoved {
                        link_id: removed_link.id.clone(),
                        was_target_of: affected.id.clone(),
                    },
                });
            }
        }
    }

    stale_marks
}

// ─── Summary ────────────────────────────────────────────────

/// Summary of a stale propagation pass, for gate computation and UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StalePropagationSummary {
    pub total_marked_stale: usize,
    pub by_type: Vec<(ArtifactType, usize)>,
    pub constitution_triggered: bool,
    pub marks: Vec<StaleMark>,
}

pub fn summarize(marks: &[StaleMark], was_constitution_change: bool) -> StalePropagationSummary {
    let mut by_type: Vec<(ArtifactType, usize)> = Vec::new();

    for mark in marks {
        if let Some(entry) = by_type.iter_mut().find(|(t, _)| *t == mark.artifact_type) {
            entry.1 += 1;
        } else {
            by_type.push((mark.artifact_type, 1));
        }
    }

    StalePropagationSummary {
        total_marked_stale: marks.len(),
        by_type,
        constitution_triggered: was_constitution_change,
        marks: marks.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LocalIdentity, TraceLinkType, ValidationSummary};

    fn identity() -> LocalIdentity {
        LocalIdentity {
            id: "user-1".into(),
            display_name: "Test".into(),
        }
    }

    fn make_artifact(id: &str, artifact_type: ArtifactType, state: ArtifactState) -> Artifact {
        Artifact {
            id: id.into(),
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

    fn make_link(id: &str, source: &str, target: &str) -> TraceLink {
        TraceLink {
            id: id.into(),
            project_id: "proj-1".into(),
            source_node_id: source.into(),
            target_node_id: target.into(),
            link_type: TraceLinkType::DerivesFrom,
            rationale: "test".into(),
            created_by: identity(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    // ─── Constitution amendment tests ───────────────────────

    #[test]
    fn constitution_amendment_marks_all_downstream_stale() {
        let artifacts = vec![
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact("sys-1", ArtifactType::SystemArchitecture, ArtifactState::Valid),
            make_artifact("ux-1", ArtifactType::UxStateMap, ArtifactState::Draft), // should NOT be marked
        ];

        let marks = propagate_constitution_amendment(&artifacts, "v1", "v2");

        assert_eq!(marks.len(), 3); // wf, feat, sys — not ux (draft)
        assert!(marks.iter().all(|m| matches!(
            m.reason,
            PropagatedStaleReason::ConstitutionAmended { .. }
        )));
    }

    #[test]
    fn constitution_amendment_skips_draft_and_complete() {
        let artifacts = vec![
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Draft),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Complete),
        ];

        let marks = propagate_constitution_amendment(&artifacts, "v1", "v2");
        assert!(marks.is_empty());
    }

    // ─── Upstream change tests ──────────────────────────────

    #[test]
    fn upstream_change_marks_type_downstream() {
        let changed = make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved);
        let all = vec![
            changed.clone(),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact("sys-1", ArtifactType::SystemArchitecture, ArtifactState::Approved),
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved), // NOT downstream
        ];
        let links = vec![];

        let marks = propagate_upstream_change(&changed, &all, &links);

        let ids: Vec<&str> = marks.iter().map(|m| m.artifact_id.as_str()).collect();
        assert!(ids.contains(&"feat-1"));
        assert!(!ids.contains(&"const-1")); // constitution is NOT downstream of workflows
    }

    #[test]
    fn upstream_change_propagates_transitively() {
        // Workflow changes → Feature (direct) → System (transitive)
        let changed = make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved);
        let all = vec![
            changed.clone(),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact("sys-1", ArtifactType::SystemArchitecture, ArtifactState::Approved),
        ];
        let links = vec![];

        let marks = propagate_upstream_change(&changed, &all, &links);

        // Feature should be direct, System should be marked too (feature is upstream of system)
        assert!(marks.len() >= 2);

        let sys_mark = marks.iter().find(|m| m.artifact_id == "sys-1");
        assert!(sys_mark.is_some());
    }

    #[test]
    fn upstream_change_follows_trace_links() {
        // Even if type graph doesn't connect them, explicit trace links should cause staleness
        let changed = make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved);
        let drift_def = make_artifact("dd-1", ArtifactType::DriftAlarmDefinitions, ArtifactState::Approved);
        let all = vec![changed.clone(), drift_def.clone()];

        // Explicit link: drift definitions depend on workflows (unusual but valid)
        let links = vec![make_link("link-1", "dd-1", "wf-1")];

        let marks = propagate_upstream_change(&changed, &all, &links);
        let dd_mark = marks.iter().find(|m| m.artifact_id == "dd-1");
        assert!(dd_mark.is_some());
    }

    #[test]
    fn upstream_change_skips_draft_artifacts() {
        let changed = make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved);
        let all = vec![
            changed.clone(),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Draft),
        ];
        let links = vec![];

        let marks = propagate_upstream_change(&changed, &all, &links);
        assert!(marks.is_empty());
    }

    // ─── Link removal tests ────────────────────────────────

    #[test]
    fn link_removal_marks_orphan_stale() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
        ];
        let removed = make_link("link-1", "feat-1", "wf-1");
        let remaining = vec![]; // no other links

        let marks = propagate_link_removal(&removed, &artifacts, &remaining);
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].artifact_id, "feat-1");
    }

    #[test]
    fn link_removal_does_not_mark_if_other_links_exist() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact("wf-2", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
        ];
        let removed = make_link("link-1", "feat-1", "wf-1");
        // feat-1 still has another link
        let remaining = vec![make_link("link-2", "feat-1", "wf-2")];

        let marks = propagate_link_removal(&removed, &artifacts, &remaining);
        assert!(marks.is_empty());
    }

    // ─── Summary tests ─────────────────────────────────────

    #[test]
    fn summary_counts_correctly() {
        let marks = vec![
            StaleMark {
                artifact_id: "wf-1".into(),
                artifact_type: ArtifactType::UserFantasyWorkflows,
                reason: PropagatedStaleReason::ConstitutionAmended {
                    old_version_id: "v1".into(),
                    new_version_id: "v2".into(),
                },
            },
            StaleMark {
                artifact_id: "feat-1".into(),
                artifact_type: ArtifactType::FeatureMap,
                reason: PropagatedStaleReason::ConstitutionAmended {
                    old_version_id: "v1".into(),
                    new_version_id: "v2".into(),
                },
            },
        ];

        let summary = summarize(&marks, true);
        assert_eq!(summary.total_marked_stale, 2);
        assert!(summary.constitution_triggered);
        assert_eq!(summary.by_type.len(), 2);
    }
}

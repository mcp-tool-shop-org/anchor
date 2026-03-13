//! Anchor Law Engine — Traceability Graph
//!
//! Owns node/link integrity. For every meaningful node, answers:
//! - "What justifies this?" (upstream query)
//! - "What depends on this?" (downstream query)
//!
//! All functions are pure. They take project state in and produce
//! validation results out.

use crate::domain::{
    Artifact, ArtifactType, RuleProvenance, RuleResult, RuleResultStatus,
    SourceArtifactType, TraceLink, TraceLinkType,
};

// ─── Required Trace Link Rules ──────────────────────────────

/// §8.1: minimum required upstream relationships per artifact type.
/// Each entry: (artifact_type, required_upstream_link_types, upstream_artifact_types)
struct TraceRequirement {
    artifact_type: ArtifactType,
    required_link_types: &'static [TraceLinkType],
    required_upstream_types: &'static [ArtifactType],
    rule_id: &'static str,
    description: &'static str,
}

const TRACE_REQUIREMENTS: &[TraceRequirement] = &[
    TraceRequirement {
        artifact_type: ArtifactType::UserFantasyWorkflows,
        required_link_types: &[TraceLinkType::DerivesFrom],
        required_upstream_types: &[ArtifactType::Constitution],
        rule_id: "trace-001",
        description: "Workflows must trace to constitution clauses via derives_from",
    },
    TraceRequirement {
        artifact_type: ArtifactType::FeatureMap,
        required_link_types: &[TraceLinkType::Justifies, TraceLinkType::DerivesFrom],
        required_upstream_types: &[ArtifactType::UserFantasyWorkflows],
        rule_id: "trace-002",
        description: "Features must trace to workflows via justifies or derives_from",
    },
    TraceRequirement {
        artifact_type: ArtifactType::SystemArchitecture,
        required_link_types: &[TraceLinkType::Implements],
        required_upstream_types: &[ArtifactType::FeatureMap],
        rule_id: "trace-003",
        description: "Systems must trace to features via implements",
    },
    TraceRequirement {
        artifact_type: ArtifactType::UxStateMap,
        required_link_types: &[TraceLinkType::DependsOn],
        required_upstream_types: &[ArtifactType::UserFantasyWorkflows, ArtifactType::FeatureMap],
        rule_id: "trace-004",
        description: "UX states must trace to workflows or features via depends_on",
    },
    TraceRequirement {
        artifact_type: ArtifactType::PhaseRoadmapContracts,
        required_link_types: &[TraceLinkType::ValidatedBy],
        required_upstream_types: &[ArtifactType::Constitution],
        rule_id: "trace-005",
        description: "Phases must trace to constitutional obligations via validated_by",
    },
    TraceRequirement {
        artifact_type: ArtifactType::DriftAlarmDefinitions,
        required_link_types: &[TraceLinkType::InvalidatedBy],
        required_upstream_types: &[ArtifactType::Constitution],
        rule_id: "trace-006",
        description: "Drift alarm definitions must trace to violated constitution nodes via invalidated_by",
    },
];

// ─── Graph Queries ──────────────────────────────────────────

/// Find all trace links where this artifact is the target (upstream justification).
pub fn upstream_links<'a>(
    artifact_id: &str,
    links: &'a [TraceLink],
) -> Vec<&'a TraceLink> {
    links
        .iter()
        .filter(|link| link.source_node_id == artifact_id)
        .collect()
}

/// Find all trace links where this artifact is the source (downstream dependents).
pub fn downstream_links<'a>(
    artifact_id: &str,
    links: &'a [TraceLink],
) -> Vec<&'a TraceLink> {
    links
        .iter()
        .filter(|link| link.target_node_id == artifact_id)
        .collect()
}

/// For a given artifact, answer: "What justifies this?"
/// Returns all upstream artifacts linked via any trace link.
pub fn justification_chain<'a>(
    artifact_id: &str,
    links: &'a [TraceLink],
    artifacts: &'a [Artifact],
) -> Vec<(&'a TraceLink, Option<&'a Artifact>)> {
    upstream_links(artifact_id, links)
        .into_iter()
        .map(|link| {
            let target_artifact = artifacts.iter().find(|a| a.id == link.target_node_id);
            (link, target_artifact)
        })
        .collect()
}

/// For a given artifact, answer: "What depends on this?"
/// Returns all downstream artifacts that reference this one.
pub fn dependent_chain<'a>(
    artifact_id: &str,
    links: &'a [TraceLink],
    artifacts: &'a [Artifact],
) -> Vec<(&'a TraceLink, Option<&'a Artifact>)> {
    downstream_links(artifact_id, links)
        .into_iter()
        .map(|link| {
            let source_artifact = artifacts.iter().find(|a| a.id == link.source_node_id);
            (link, source_artifact)
        })
        .collect()
}

// ─── Validation ─────────────────────────────────────────────

/// Validate that an artifact has the required upstream trace links.
/// §8.1: minimum required relationships.
pub fn validate_artifact_traceability(
    artifact: &Artifact,
    all_artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<RuleResult> {
    let mut results = Vec::new();

    // Find the requirement for this artifact type
    let requirements: Vec<&TraceRequirement> = TRACE_REQUIREMENTS
        .iter()
        .filter(|r| r.artifact_type == artifact.artifact_type)
        .collect();

    // Constitution and execution_readiness_gate have no upstream requirements
    if requirements.is_empty() {
        return results;
    }

    for req in &requirements {
        // Find upstream links from this artifact
        let outbound = upstream_links(&artifact.id, links);

        // Check if any link matches the required types AND points to the right upstream artifact type
        let has_required_link = outbound.iter().any(|link| {
            // Link type must be one of the required types
            let type_matches = req.required_link_types.contains(&link.link_type);
            // Target must be an artifact of the required upstream type
            let target_matches = all_artifacts.iter().any(|a| {
                a.id == link.target_node_id
                    && req.required_upstream_types.contains(&a.artifact_type)
            });
            type_matches && target_matches
        });

        let status = if has_required_link {
            RuleResultStatus::Pass
        } else {
            RuleResultStatus::Fail
        };

        results.push(RuleResult {
            rule_id: req.rule_id.to_string(),
            status,
            message: if status == RuleResultStatus::Pass {
                format!("{:?} has required upstream traceability", artifact.artifact_type)
            } else {
                format!(
                    "{:?} is missing required upstream trace link: {}",
                    artifact.artifact_type, req.description
                )
            },
            rule_provenance: RuleProvenance {
                source_artifact_type: SourceArtifactType::SystemRule,
                source_clause: format!("§8.1 — {}", req.rule_id),
                human_label: req.description.to_string(),
            },
            affected_node_ids: vec![artifact.id.clone()],
        });
    }

    results
}

/// Validate that all trace link endpoints resolve to real artifacts.
/// §8.2: no dangling references.
pub fn validate_link_endpoints(
    links: &[TraceLink],
    artifacts: &[Artifact],
) -> Vec<RuleResult> {
    let artifact_ids: Vec<&str> = artifacts.iter().map(|a| a.id.as_str()).collect();
    let mut results = Vec::new();

    for link in links {
        let source_exists = artifact_ids.contains(&link.source_node_id.as_str());
        let target_exists = artifact_ids.contains(&link.target_node_id.as_str());

        if !source_exists || !target_exists {
            let mut dangling = Vec::new();
            if !source_exists {
                dangling.push(format!("source {}", link.source_node_id));
            }
            if !target_exists {
                dangling.push(format!("target {}", link.target_node_id));
            }

            results.push(RuleResult {
                rule_id: "trace-endpoint-001".to_string(),
                status: RuleResultStatus::Fail,
                message: format!(
                    "Trace link {} has dangling endpoint(s): {}",
                    link.id,
                    dangling.join(", ")
                ),
                rule_provenance: RuleProvenance {
                    source_artifact_type: SourceArtifactType::SystemRule,
                    source_clause: "§8.2 — trace link endpoint resolution".to_string(),
                    human_label: "All trace link endpoints must resolve to existing artifacts"
                        .to_string(),
                },
                affected_node_ids: vec![link.source_node_id.clone(), link.target_node_id.clone()],
            });
        }
    }

    results
}

/// Validate bidirectional explainability: every non-terminal, non-root artifact
/// must have both upstream justification and downstream dependents.
/// §8.2: bidirectional explainability requirement.
pub fn validate_bidirectional_explainability(
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<RuleResult> {
    let mut results = Vec::new();

    for artifact in artifacts {
        // Constitution is root — only needs downstream
        // ExecutionReadinessGate is terminal — only needs upstream
        let needs_upstream = artifact.artifact_type != ArtifactType::Constitution;
        let needs_downstream = artifact.artifact_type != ArtifactType::ExecutionReadinessGate;

        if needs_upstream {
            let up = upstream_links(&artifact.id, links);
            if up.is_empty() {
                results.push(RuleResult {
                    rule_id: "trace-bidir-upstream".to_string(),
                    status: RuleResultStatus::Fail,
                    message: format!(
                        "{:?} ({}) has no upstream justification — cannot answer 'what justifies this?'",
                        artifact.artifact_type, artifact.id
                    ),
                    rule_provenance: RuleProvenance {
                        source_artifact_type: SourceArtifactType::SystemRule,
                        source_clause: "§8.2 — bidirectional explainability".to_string(),
                        human_label: "Every non-root artifact must have upstream justification".to_string(),
                    },
                    affected_node_ids: vec![artifact.id.clone()],
                });
            }
        }

        if needs_downstream {
            let down = downstream_links(&artifact.id, links);
            if down.is_empty() {
                results.push(RuleResult {
                    rule_id: "trace-bidir-downstream".to_string(),
                    status: RuleResultStatus::Fail,
                    message: format!(
                        "{:?} ({}) has no downstream dependents — cannot answer 'what depends on this?'",
                        artifact.artifact_type, artifact.id
                    ),
                    rule_provenance: RuleProvenance {
                        source_artifact_type: SourceArtifactType::SystemRule,
                        source_clause: "§8.2 — bidirectional explainability".to_string(),
                        human_label: "Every non-terminal artifact must have downstream dependents".to_string(),
                    },
                    affected_node_ids: vec![artifact.id.clone()],
                });
            }
        }
    }

    results
}

/// Run full traceability validation for the entire project.
/// Returns all rule results across all artifacts and links.
pub fn validate_all_traceability(
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<RuleResult> {
    let mut results = Vec::new();

    // Per-artifact required link checks
    for artifact in artifacts {
        results.extend(validate_artifact_traceability(artifact, artifacts, links));
    }

    // Endpoint resolution
    results.extend(validate_link_endpoints(links, artifacts));

    // Bidirectional explainability
    results.extend(validate_bidirectional_explainability(artifacts, links));

    results
}

/// Count failures in a set of rule results.
pub fn failure_count(results: &[RuleResult]) -> usize {
    results
        .iter()
        .filter(|r| r.status == RuleResultStatus::Fail)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ArtifactState, LocalIdentity, ValidationSummary};

    fn identity() -> LocalIdentity {
        LocalIdentity {
            id: "user-1".into(),
            display_name: "Test".into(),
        }
    }

    fn make_artifact(id: &str, artifact_type: ArtifactType) -> Artifact {
        Artifact {
            id: id.into(),
            project_id: "proj-1".into(),
            artifact_type,
            title: format!("{:?}", artifact_type),
            current_version_id: "v1".into(),
            state: ArtifactState::Approved,
            validation_summary: ValidationSummary::default(),
            latest_approval_id: None,
            stale_reason: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn make_link(id: &str, source: &str, target: &str, link_type: TraceLinkType) -> TraceLink {
        TraceLink {
            id: id.into(),
            project_id: "proj-1".into(),
            source_node_id: source.into(),
            target_node_id: target.into(),
            link_type,
            rationale: "test link".into(),
            created_by: identity(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    #[test]
    fn workflow_without_constitution_link_fails() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows),
        ];
        let links = vec![]; // no links

        let results = validate_artifact_traceability(&artifacts[1], &artifacts, &links);
        assert_eq!(failure_count(&results), 1);
        assert!(results[0].message.contains("missing required upstream"));
    }

    #[test]
    fn workflow_with_constitution_link_passes() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows),
        ];
        let links = vec![make_link(
            "link-1",
            "wf-1",
            "const-1",
            TraceLinkType::DerivesFrom,
        )];

        let results = validate_artifact_traceability(&artifacts[1], &artifacts, &links);
        assert_eq!(failure_count(&results), 0);
    }

    #[test]
    fn feature_without_workflow_link_fails() {
        let artifacts = vec![
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows),
            make_artifact("feat-1", ArtifactType::FeatureMap),
        ];
        let links = vec![];

        let results = validate_artifact_traceability(&artifacts[1], &artifacts, &links);
        assert_eq!(failure_count(&results), 1);
    }

    #[test]
    fn feature_with_workflow_justifies_link_passes() {
        let artifacts = vec![
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows),
            make_artifact("feat-1", ArtifactType::FeatureMap),
        ];
        let links = vec![make_link(
            "link-1",
            "feat-1",
            "wf-1",
            TraceLinkType::Justifies,
        )];

        let results = validate_artifact_traceability(&artifacts[1], &artifacts, &links);
        assert_eq!(failure_count(&results), 0);
    }

    #[test]
    fn dangling_link_target_fails_endpoint_validation() {
        let artifacts = vec![make_artifact("a-1", ArtifactType::FeatureMap)];
        let links = vec![make_link(
            "link-1",
            "a-1",
            "nonexistent",
            TraceLinkType::DerivesFrom,
        )];

        let results = validate_link_endpoints(&links, &artifacts);
        assert_eq!(failure_count(&results), 1);
        assert!(results[0].message.contains("dangling"));
    }

    #[test]
    fn valid_link_endpoints_pass() {
        let artifacts = vec![
            make_artifact("a-1", ArtifactType::FeatureMap),
            make_artifact("a-2", ArtifactType::UserFantasyWorkflows),
        ];
        let links = vec![make_link(
            "link-1",
            "a-1",
            "a-2",
            TraceLinkType::DerivesFrom,
        )];

        let results = validate_link_endpoints(&links, &artifacts);
        assert_eq!(failure_count(&results), 0);
    }

    #[test]
    fn non_root_artifact_without_upstream_fails_bidirectional() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap),
        ];
        let links = vec![];

        let results = validate_bidirectional_explainability(&artifacts, &links);
        // Should fail for both upstream (no justification) and downstream (no dependents, but
        // FeatureMap is not terminal so it needs downstream too)
        let failures: Vec<_> = results.iter().filter(|r| r.status == RuleResultStatus::Fail).collect();
        assert!(failures.len() >= 1);
        assert!(failures.iter().any(|r| r.message.contains("upstream justification")));
    }

    #[test]
    fn constitution_needs_no_upstream() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution),
        ];
        // Constitution only needs downstream, not upstream
        let links = vec![]; // no links — downstream check will fail but upstream won't

        let results = validate_bidirectional_explainability(&artifacts, &links);
        let upstream_failures: Vec<_> = results
            .iter()
            .filter(|r| r.status == RuleResultStatus::Fail && r.message.contains("upstream"))
            .collect();
        assert_eq!(upstream_failures.len(), 0);
    }

    #[test]
    fn full_validation_catches_multiple_issues() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows),
            make_artifact("feat-1", ArtifactType::FeatureMap),
        ];
        let links = vec![
            // wf derives from constitution — good
            make_link("link-1", "wf-1", "const-1", TraceLinkType::DerivesFrom),
            // feat has NO link to wf — bad
        ];

        let results = validate_all_traceability(&artifacts, &links);
        let failures = failure_count(&results);
        assert!(failures > 0, "Should catch missing feature→workflow link");
    }
}

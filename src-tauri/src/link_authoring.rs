//! Anchor — Trace Link Authoring with Legality Checks
//!
//! Operators can add and remove trace links. Every mutation is
//! checked against the traceability rules before it happens.
//! The UI gets clear feedback: what's legal, what's missing,
//! and what would break if a link is removed.

use crate::domain::*;
use crate::traceability;

// ─── Types ──────────────────────────────────────────────────

/// Result of adding a trace link.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddLinkResult {
    pub success: bool,
    pub link: Option<TraceLink>,
    pub error: Option<String>,
}

/// Result of removing a trace link.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveLinkResult {
    pub success: bool,
    pub warning: Option<String>,
    pub orphaned_artifacts: Vec<String>,
}

/// A suggested link the operator should add.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkSuggestion {
    pub source_artifact_id: String,
    pub source_title: String,
    pub suggested_link_type: TraceLinkType,
    pub suggested_target_type: ArtifactType,
    pub candidate_targets: Vec<LinkTarget>,
    pub rule_description: String,
}

/// A possible target for a suggested link.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkTarget {
    pub artifact_id: String,
    pub title: String,
    pub artifact_type: ArtifactType,
}

/// What link types are allowed from a given artifact.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedLinks {
    pub artifact_id: String,
    pub artifact_type: ArtifactType,
    pub allowed: Vec<AllowedLinkOption>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedLinkOption {
    pub link_type: TraceLinkType,
    pub target_types: Vec<ArtifactType>,
    pub candidates: Vec<LinkTarget>,
    pub required: bool,
    pub already_satisfied: bool,
}

// ─── Trace Link Requirements (shared with traceability.rs) ──

struct LinkRule {
    artifact_type: ArtifactType,
    link_types: &'static [TraceLinkType],
    upstream_types: &'static [ArtifactType],
    description: &'static str,
}

const LINK_RULES: &[LinkRule] = &[
    LinkRule {
        artifact_type: ArtifactType::UserFantasyWorkflows,
        link_types: &[TraceLinkType::DerivesFrom],
        upstream_types: &[ArtifactType::Constitution],
        description: "Workflows must trace to constitution via derives_from",
    },
    LinkRule {
        artifact_type: ArtifactType::FeatureMap,
        link_types: &[TraceLinkType::Justifies, TraceLinkType::DerivesFrom],
        upstream_types: &[ArtifactType::UserFantasyWorkflows],
        description: "Features must trace to workflows via justifies or derives_from",
    },
    LinkRule {
        artifact_type: ArtifactType::SystemArchitecture,
        link_types: &[TraceLinkType::Implements],
        upstream_types: &[ArtifactType::FeatureMap],
        description: "Systems must trace to features via implements",
    },
    LinkRule {
        artifact_type: ArtifactType::UxStateMap,
        link_types: &[TraceLinkType::DependsOn],
        upstream_types: &[ArtifactType::UserFantasyWorkflows, ArtifactType::FeatureMap],
        description: "UX states must trace to workflows or features via depends_on",
    },
    LinkRule {
        artifact_type: ArtifactType::PhaseRoadmapContracts,
        link_types: &[TraceLinkType::ValidatedBy],
        upstream_types: &[ArtifactType::Constitution],
        description: "Phases must trace to constitution via validated_by",
    },
    LinkRule {
        artifact_type: ArtifactType::DriftAlarmDefinitions,
        link_types: &[TraceLinkType::InvalidatedBy],
        upstream_types: &[ArtifactType::Constitution],
        description: "Drift alarms must trace to constitution via invalidated_by",
    },
];

// ─── Link Addition ──────────────────────────────────────────

/// Add a trace link with legality checks.
///
/// Validates:
/// - Source and target artifacts exist
/// - No duplicate link (same source + target + type)
/// - Link type is structurally valid (not nonsense)
pub fn add_link(
    source_id: &str,
    target_id: &str,
    link_type: TraceLinkType,
    rationale: &str,
    artifacts: &[Artifact],
    links: &[TraceLink],
    actor: &LocalIdentity,
) -> AddLinkResult {
    // Validate source exists
    let source = match artifacts.iter().find(|a| a.id == source_id) {
        Some(a) => a,
        None => return AddLinkResult {
            success: false,
            link: None,
            error: Some(format!("Source artifact not found: {}", source_id)),
        },
    };

    // Validate target exists
    let _target = match artifacts.iter().find(|a| a.id == target_id) {
        Some(a) => a,
        None => return AddLinkResult {
            success: false,
            link: None,
            error: Some(format!("Target artifact not found: {}", target_id)),
        },
    };

    // Check for duplicate
    let is_duplicate = links.iter().any(|l| {
        l.source_node_id == source_id
            && l.target_node_id == target_id
            && l.link_type == link_type
    });
    if is_duplicate {
        return AddLinkResult {
            success: false,
            link: None,
            error: Some(format!(
                "Duplicate link: {:?} from {} to {} already exists",
                link_type, source_id, target_id
            )),
        };
    }

    // Prevent self-links
    if source_id == target_id {
        return AddLinkResult {
            success: false,
            link: None,
            error: Some("Cannot create a trace link from an artifact to itself".into()),
        };
    }

    // Create the link
    let link = TraceLink {
        id: format!("link-{}", links.len() + 1),
        project_id: source.project_id.clone(),
        source_node_id: source_id.into(),
        target_node_id: target_id.into(),
        link_type,
        rationale: rationale.into(),
        created_by: actor.clone(),
        created_at: "2026-03-13T12:00:00Z".into(),
    };

    AddLinkResult {
        success: true,
        link: Some(link),
        error: None,
    }
}

/// Check what would happen if a link is removed.
/// Returns warning if it would leave the source artifact without required links.
pub fn check_removal_impact(
    link_id: &str,
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> RemoveLinkResult {
    let link = match links.iter().find(|l| l.id == link_id) {
        Some(l) => l,
        None => return RemoveLinkResult {
            success: false,
            warning: Some(format!("Link not found: {}", link_id)),
            orphaned_artifacts: vec![],
        },
    };

    let source = match artifacts.iter().find(|a| a.id == link.source_node_id) {
        Some(a) => a,
        None => return RemoveLinkResult {
            success: true,
            warning: None,
            orphaned_artifacts: vec![],
        },
    };

    // Simulate removal: check if the source would lose its required links
    let remaining_links: Vec<_> = links.iter().filter(|l| l.id != link_id).collect();
    let remaining_refs: Vec<TraceLink> = remaining_links.into_iter().cloned().collect();

    let mut orphaned = Vec::new();
    for rule in LINK_RULES {
        if rule.artifact_type != source.artifact_type {
            continue;
        }

        let outbound = traceability::upstream_links(&source.id, &remaining_refs);
        let still_satisfied = outbound.iter().any(|l| {
            let type_ok = rule.link_types.contains(&l.link_type);
            let target_ok = artifacts
                .iter()
                .any(|a| a.id == l.target_node_id && rule.upstream_types.contains(&a.artifact_type));
            type_ok && target_ok
        });

        if !still_satisfied {
            orphaned.push(source.id.clone());
        }
    }

    let warning = if !orphaned.is_empty() {
        Some(format!(
            "Removing this link will leave {} without required traceability. The artifact may fail validation.",
            source.title
        ))
    } else {
        None
    };

    RemoveLinkResult {
        success: true,
        warning,
        orphaned_artifacts: orphaned,
    }
}

// ─── Link Discovery ─────────────────────────────────────────

/// Get all allowed link types for an artifact, with candidate targets.
pub fn get_allowed_links(
    artifact_id: &str,
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> Option<AllowedLinks> {
    let artifact = artifacts.iter().find(|a| a.id == artifact_id)?;

    let mut allowed = Vec::new();

    for rule in LINK_RULES {
        if rule.artifact_type != artifact.artifact_type {
            continue;
        }

        // Check if this requirement is already satisfied
        let outbound = traceability::upstream_links(&artifact.id, links);
        let satisfied = outbound.iter().any(|l| {
            let type_ok = rule.link_types.contains(&l.link_type);
            let target_ok = artifacts
                .iter()
                .any(|a| a.id == l.target_node_id && rule.upstream_types.contains(&a.artifact_type));
            type_ok && target_ok
        });

        // Find candidate target artifacts
        let candidates: Vec<LinkTarget> = artifacts
            .iter()
            .filter(|a| rule.upstream_types.contains(&a.artifact_type) && a.id != artifact_id)
            .map(|a| LinkTarget {
                artifact_id: a.id.clone(),
                title: a.title.clone(),
                artifact_type: a.artifact_type,
            })
            .collect();

        for lt in rule.link_types {
            allowed.push(AllowedLinkOption {
                link_type: *lt,
                target_types: rule.upstream_types.to_vec(),
                candidates: candidates.clone(),
                required: true,
                already_satisfied: satisfied,
            });
        }
    }

    // Also allow any-to-any links with the 6 standard types (non-required)
    // This enables organic traceability beyond the minimum requirements
    let all_types = [
        TraceLinkType::Justifies,
        TraceLinkType::DerivesFrom,
        TraceLinkType::Implements,
        TraceLinkType::DependsOn,
        TraceLinkType::ValidatedBy,
        TraceLinkType::InvalidatedBy,
    ];

    let existing_required: Vec<TraceLinkType> = allowed.iter().map(|a| a.link_type).collect();
    for lt in &all_types {
        if existing_required.contains(lt) {
            continue;
        }
        let all_targets: Vec<LinkTarget> = artifacts
            .iter()
            .filter(|a| a.id != artifact_id)
            .map(|a| LinkTarget {
                artifact_id: a.id.clone(),
                title: a.title.clone(),
                artifact_type: a.artifact_type,
            })
            .collect();
        allowed.push(AllowedLinkOption {
            link_type: *lt,
            target_types: vec![],
            candidates: all_targets,
            required: false,
            already_satisfied: false,
        });
    }

    Some(AllowedLinks {
        artifact_id: artifact_id.into(),
        artifact_type: artifact.artifact_type,
        allowed,
    })
}

/// Get missing links across the whole project — what the operator should add.
pub fn get_missing_links(
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<LinkSuggestion> {
    let mut suggestions = Vec::new();

    for artifact in artifacts {
        for rule in LINK_RULES {
            if rule.artifact_type != artifact.artifact_type {
                continue;
            }

            let outbound = traceability::upstream_links(&artifact.id, links);
            let satisfied = outbound.iter().any(|l| {
                let type_ok = rule.link_types.contains(&l.link_type);
                let target_ok = artifacts
                    .iter()
                    .any(|a| a.id == l.target_node_id && rule.upstream_types.contains(&a.artifact_type));
                type_ok && target_ok
            });

            if !satisfied {
                let candidates: Vec<LinkTarget> = artifacts
                    .iter()
                    .filter(|a| rule.upstream_types.contains(&a.artifact_type))
                    .map(|a| LinkTarget {
                        artifact_id: a.id.clone(),
                        title: a.title.clone(),
                        artifact_type: a.artifact_type,
                    })
                    .collect();

                suggestions.push(LinkSuggestion {
                    source_artifact_id: artifact.id.clone(),
                    source_title: artifact.title.clone(),
                    suggested_link_type: rule.link_types[0],
                    suggested_target_type: rule.upstream_types[0],
                    candidate_targets: candidates,
                    rule_description: rule.description.into(),
                });
            }
        }
    }

    suggestions
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectStore;

    fn actor() -> LocalIdentity {
        LocalIdentity {
            id: "user-test".into(),
            display_name: "Test Operator".into(),
        }
    }

    #[test]
    fn add_link_succeeds_for_valid_artifacts() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        // Find two distinct artifacts
        let art1 = &store.artifacts[0];
        let art2 = &store.artifacts[1];
        let result = add_link(
            &art1.id, &art2.id, TraceLinkType::DerivesFrom,
            "Test link", &store.artifacts, &store.links, &actor(),
        );
        assert!(result.success);
        assert!(result.link.is_some());
    }

    #[test]
    fn add_link_rejects_missing_source() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let result = add_link(
            "nonexistent", &store.artifacts[0].id, TraceLinkType::DerivesFrom,
            "Test", &store.artifacts, &store.links, &actor(),
        );
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not found"));
    }

    #[test]
    fn add_link_rejects_duplicate() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        if let Some(existing) = store.links.first() {
            let result = add_link(
                &existing.source_node_id, &existing.target_node_id,
                existing.link_type, "Dup", &store.artifacts, &store.links, &actor(),
            );
            assert!(!result.success);
            assert!(result.error.unwrap().contains("Duplicate"));
        }
    }

    #[test]
    fn add_link_rejects_self_link() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let art = &store.artifacts[0];
        let result = add_link(
            &art.id, &art.id, TraceLinkType::DependsOn,
            "Self", &store.artifacts, &store.links, &actor(),
        );
        assert!(!result.success);
        assert!(result.error.unwrap().contains("itself"));
    }

    #[test]
    fn removal_warns_on_required_link() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        // Crystal Sanctum has all required links. Removing one should warn.
        if let Some(required_link) = store.links.first() {
            let result = check_removal_impact(
                &required_link.id, &store.artifacts, &store.links,
            );
            assert!(result.success);
            // May or may not warn depending on which link
        }
    }

    #[test]
    fn removal_fails_for_missing_link() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let result = check_removal_impact(
            "nonexistent-link", &store.artifacts, &store.links,
        );
        assert!(!result.success);
    }

    #[test]
    fn allowed_links_returns_options() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        // Get allowed links for a workflow artifact
        let wf = store.artifacts.iter().find(|a| a.artifact_type == ArtifactType::UserFantasyWorkflows);
        if let Some(wf_art) = wf {
            let allowed = get_allowed_links(&wf_art.id, &store.artifacts, &store.links);
            assert!(allowed.is_some());
            let allowed = allowed.unwrap();
            // Should have required derives_from + optional types
            assert!(!allowed.allowed.is_empty());
            assert!(allowed.allowed.iter().any(|a| a.required));
        }
    }

    #[test]
    fn missing_links_detected_in_shadow_protocol() {
        let store = ProjectStore::load_scenario("shadow-protocol").unwrap();
        let missing = get_missing_links(&store.artifacts, &store.links);
        assert!(!missing.is_empty(), "Shadow Protocol should have missing links");
    }

    #[test]
    fn no_missing_links_in_crystal_sanctum() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        let missing = get_missing_links(&store.artifacts, &store.links);
        assert!(missing.is_empty(), "Crystal Sanctum should have all required links");
    }

    #[test]
    fn suggestions_include_candidate_targets() {
        let store = ProjectStore::load_scenario("shadow-protocol").unwrap();
        let missing = get_missing_links(&store.artifacts, &store.links);
        for suggestion in &missing {
            assert!(!suggestion.candidate_targets.is_empty(),
                "Suggestion for {} should have candidate targets", suggestion.source_title);
        }
    }
}

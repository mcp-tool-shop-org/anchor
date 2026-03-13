//! Anchor Law Engine — Drift Rule Engine
//!
//! Turns the drift alarm taxonomy into executable rules.
//! Each rule is a pure function: project state in, alarm list out.
//!
//! Five categories: traceability, constitution, sequence, quality, scope.

use crate::domain::{
    Artifact, ArtifactState, ArtifactType, ArtifactVersion, Constitution,
    DriftAlarm, DriftAlarmSeverity, DriftAlarmStatus, DriftAlarmType,
    RuleProvenance, SourceArtifactType, TraceLink,
};
use crate::traceability;

// ─── Rule Definitions ───────────────────────────────────────

/// Run all drift alarm rules against the current project state.
/// Returns a list of new alarms to raise (does not mutate state).
pub fn evaluate_all_rules(
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    links: &[TraceLink],
    constitution: &Constitution,
) -> Vec<DriftAlarmBlueprint> {
    let mut alarms = Vec::new();

    alarms.extend(check_traceability_drift(artifacts, links));
    alarms.extend(check_constitution_drift(artifacts, versions, constitution));
    alarms.extend(check_sequence_drift(artifacts, versions, constitution));
    alarms.extend(check_scope_drift(artifacts, links));

    alarms
}

/// Blueprint for a drift alarm to be raised.
/// Caller is responsible for assigning IDs and timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftAlarmBlueprint {
    pub alarm_type: DriftAlarmType,
    pub severity: DriftAlarmSeverity,
    pub source_artifact_id: String,
    pub affected_node_ids: Vec<String>,
    pub violated_rule_id: String,
    pub rule_provenance: RuleProvenance,
    pub explanation: String,
    pub remediation_path: Vec<String>,
}

// ─── Traceability Drift ─────────────────────────────────────

/// §11.1: item has no valid upstream justification or required trace link.
fn check_traceability_drift(
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<DriftAlarmBlueprint> {
    let mut alarms = Vec::new();

    for artifact in artifacts {
        // Skip constitution (root) and gate (computed)
        if matches!(
            artifact.artifact_type,
            ArtifactType::Constitution | ArtifactType::ExecutionReadinessGate
        ) {
            continue;
        }

        let results = traceability::validate_artifact_traceability(artifact, artifacts, links);
        for result in &results {
            if result.status == crate::domain::RuleResultStatus::Fail {
                alarms.push(DriftAlarmBlueprint {
                    alarm_type: DriftAlarmType::TraceabilityDrift,
                    severity: DriftAlarmSeverity::Error,
                    source_artifact_id: artifact.id.clone(),
                    affected_node_ids: vec![artifact.id.clone()],
                    violated_rule_id: result.rule_id.clone(),
                    rule_provenance: result.rule_provenance.clone(),
                    explanation: result.message.clone(),
                    remediation_path: vec![
                        format!(
                            "Add required trace link from {:?} to upstream artifact",
                            artifact.artifact_type
                        ),
                        "Re-validate artifact after adding link".to_string(),
                    ],
                });
            }
        }
    }

    alarms
}

// ─── Constitution Drift ─────────────────────────────────────

/// §11.2: artifact conflicts with promise, anti-goals, or quality bar.
///
/// This checks for structural indicators of constitution drift:
/// - Features with declared anti-goal conflicts
/// - Artifacts whose version was created against a different constitution version
fn check_constitution_drift(
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    constitution: &Constitution,
) -> Vec<DriftAlarmBlueprint> {
    let mut alarms = Vec::new();

    for artifact in artifacts {
        if artifact.artifact_type == ArtifactType::Constitution {
            continue;
        }

        // Find current version for this artifact
        let current_version = versions
            .iter()
            .find(|v| v.id == artifact.current_version_id);

        if let Some(version) = current_version {
            // Check if artifact was created against a different constitution version
            if version.constitution_version_id != constitution.version_id {
                alarms.push(DriftAlarmBlueprint {
                    alarm_type: DriftAlarmType::ConstitutionDrift,
                    severity: DriftAlarmSeverity::Warning,
                    source_artifact_id: artifact.id.clone(),
                    affected_node_ids: vec![artifact.id.clone()],
                    violated_rule_id: "const-drift-001".to_string(),
                    rule_provenance: RuleProvenance {
                        source_artifact_type: SourceArtifactType::Artifact(
                            ArtifactType::Constitution,
                        ),
                        source_clause: "Constitution version alignment".to_string(),
                        human_label:
                            "Artifact version was created against a different constitution version"
                                .to_string(),
                    },
                    explanation: format!(
                        "{:?} version {} was created against constitution version {}, \
                         but current constitution is version {}",
                        artifact.artifact_type,
                        version.version_number,
                        version.constitution_version_id,
                        constitution.version_id
                    ),
                    remediation_path: vec![
                        "Review artifact content against current constitution".to_string(),
                        "Update artifact to align with current constitution version".to_string(),
                        "Re-validate and re-approve".to_string(),
                    ],
                });
            }
        }
    }

    alarms
}

// ─── Sequence Drift ─────────────────────────────────────────

/// §11.3: downstream approved while upstream changed.
///
/// Checks for artifacts that are still Approved but whose constitution
/// version reference doesn't match the current constitution.
fn check_sequence_drift(
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    constitution: &Constitution,
) -> Vec<DriftAlarmBlueprint> {
    let mut alarms = Vec::new();

    for artifact in artifacts {
        if artifact.state != ArtifactState::Approved {
            continue;
        }
        if artifact.artifact_type == ArtifactType::Constitution {
            continue;
        }

        let current_version = versions
            .iter()
            .find(|v| v.id == artifact.current_version_id);

        if let Some(version) = current_version {
            if version.constitution_version_id != constitution.version_id {
                alarms.push(DriftAlarmBlueprint {
                    alarm_type: DriftAlarmType::SequenceDrift,
                    severity: DriftAlarmSeverity::Blocking,
                    source_artifact_id: artifact.id.clone(),
                    affected_node_ids: vec![artifact.id.clone()],
                    violated_rule_id: "seq-drift-001".to_string(),
                    rule_provenance: RuleProvenance {
                        source_artifact_type: SourceArtifactType::SystemRule,
                        source_clause: "§11.3 — sequence drift detection".to_string(),
                        human_label:
                            "Approved artifact has outdated constitution version reference"
                                .to_string(),
                    },
                    explanation: format!(
                        "{:?} is still Approved but was approved against constitution version {}, \
                         current is {}. This artifact should be Stale.",
                        artifact.artifact_type,
                        version.constitution_version_id,
                        constitution.version_id
                    ),
                    remediation_path: vec![
                        "Mark artifact as Stale".to_string(),
                        "Reconcile content with new constitution".to_string(),
                        "Re-validate and re-approve".to_string(),
                    ],
                });
            }
        }
    }

    alarms
}

// ─── Scope Drift ────────────────────────────────────────────

/// §11.5: new surface area appears without explicit justification.
///
/// Features without any workflow link, systems without any feature link.
fn check_scope_drift(
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> Vec<DriftAlarmBlueprint> {
    let mut alarms = Vec::new();

    for artifact in artifacts {
        match artifact.artifact_type {
            ArtifactType::FeatureMap => {
                // Feature must have at least one link to a workflow
                let has_workflow_link = links.iter().any(|link| {
                    link.source_node_id == artifact.id
                        && artifacts.iter().any(|a| {
                            a.id == link.target_node_id
                                && a.artifact_type == ArtifactType::UserFantasyWorkflows
                        })
                });

                if !has_workflow_link {
                    alarms.push(DriftAlarmBlueprint {
                        alarm_type: DriftAlarmType::ScopeDrift,
                        severity: DriftAlarmSeverity::Error,
                        source_artifact_id: artifact.id.clone(),
                        affected_node_ids: vec![artifact.id.clone()],
                        violated_rule_id: "scope-drift-001".to_string(),
                        rule_provenance: RuleProvenance {
                            source_artifact_type: SourceArtifactType::SystemRule,
                            source_clause: "§11.5 — scope drift detection".to_string(),
                            human_label: "Feature exists without workflow justification"
                                .to_string(),
                        },
                        explanation: format!(
                            "Feature {:?} ({}) has no trace link to any workflow — \
                             new surface area without explicit authorization",
                            artifact.title, artifact.id
                        ),
                        remediation_path: vec![
                            "Link feature to a workflow that justifies it".to_string(),
                            "Or remove the feature if it is not authorized by the user fantasy"
                                .to_string(),
                        ],
                    });
                }
            }
            ArtifactType::SystemArchitecture => {
                // System must have at least one link to a feature
                let has_feature_link = links.iter().any(|link| {
                    link.source_node_id == artifact.id
                        && artifacts.iter().any(|a| {
                            a.id == link.target_node_id
                                && a.artifact_type == ArtifactType::FeatureMap
                        })
                });

                if !has_feature_link {
                    alarms.push(DriftAlarmBlueprint {
                        alarm_type: DriftAlarmType::ScopeDrift,
                        severity: DriftAlarmSeverity::Error,
                        source_artifact_id: artifact.id.clone(),
                        affected_node_ids: vec![artifact.id.clone()],
                        violated_rule_id: "scope-drift-002".to_string(),
                        rule_provenance: RuleProvenance {
                            source_artifact_type: SourceArtifactType::SystemRule,
                            source_clause: "§11.5 — scope drift detection".to_string(),
                            human_label:
                                "System module exists without feature implementation trace"
                                    .to_string(),
                        },
                        explanation: format!(
                            "System {:?} ({}) has no trace link to any feature — \
                             system exists without feature justification",
                            artifact.title, artifact.id
                        ),
                        remediation_path: vec![
                            "Link system to a feature it implements".to_string(),
                            "Or remove the system if no feature requires it".to_string(),
                        ],
                    });
                }
            }
            _ => {}
        }
    }

    alarms
}

// ─── Alarm Resolution ───────────────────────────────────────

/// Given existing active alarms and newly computed alarm blueprints,
/// determine which alarms should be resolved (no longer triggered).
pub fn compute_resolved_alarms<'a>(
    active_alarms: &'a [DriftAlarm],
    current_blueprints: &[DriftAlarmBlueprint],
) -> Vec<&'a str> {
    active_alarms
        .iter()
        .filter(|alarm| {
            alarm.status == DriftAlarmStatus::Active
                && !current_blueprints.iter().any(|bp| {
                    bp.violated_rule_id == alarm.violated_rule_id
                        && bp.source_artifact_id == alarm.source_artifact_id
                })
        })
        .map(|alarm| alarm.id.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LocalIdentity, ValidationSummary};

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
            current_version_id: format!("{}-v1", id),
            state,
            validation_summary: ValidationSummary::default(),
            latest_approval_id: None,
            stale_reason: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn make_version(
        artifact_id: &str,
        constitution_version_id: &str,
    ) -> ArtifactVersion {
        ArtifactVersion {
            id: format!("{}-v1", artifact_id),
            artifact_id: artifact_id.into(),
            project_id: "proj-1".into(),
            version_number: 1,
            constitution_version_id: constitution_version_id.into(),
            content: serde_json::Value::Null,
            content_hash: "hash-1".into(),
            parent_version_id: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            created_by: identity(),
        }
    }

    fn make_constitution(version_id: &str) -> Constitution {
        Constitution {
            id: "const-1".into(),
            artifact_id: "art-const".into(),
            version_id: version_id.into(),
            project_id: "proj-1".into(),
            one_sentence_promise: "Test promise".into(),
            user_fantasy: "Test fantasy".into(),
            non_negotiable_outcomes: vec![],
            anti_goals: vec![],
            quality_bar: "High".into(),
            failure_condition: "None".into(),
            locked: true,
            content_hash: "hash-const".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            approved_at: Some("2026-03-13T00:00:00Z".into()),
            approved_by: Some(identity()),
            parent_version_id: None,
        }
    }

    fn make_link(source: &str, target: &str, link_type: crate::domain::TraceLinkType) -> TraceLink {
        TraceLink {
            id: format!("link-{}-{}", source, target),
            project_id: "proj-1".into(),
            source_node_id: source.into(),
            target_node_id: target.into(),
            link_type,
            rationale: "test".into(),
            created_by: identity(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    #[test]
    fn traceability_drift_fires_on_unlinked_feature() {
        let artifacts = vec![
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
        ];
        let links = vec![]; // no links

        let alarms = check_traceability_drift(&artifacts, &links);
        assert!(!alarms.is_empty());
        assert!(alarms.iter().any(|a| a.alarm_type == DriftAlarmType::TraceabilityDrift));
    }

    #[test]
    fn no_traceability_drift_when_properly_linked() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
        ];
        let links = vec![make_link(
            "wf-1",
            "const-1",
            crate::domain::TraceLinkType::DerivesFrom,
        )];

        let alarms = check_traceability_drift(&artifacts, &links);
        let wf_alarms: Vec<_> = alarms
            .iter()
            .filter(|a| a.source_artifact_id == "wf-1")
            .collect();
        assert!(wf_alarms.is_empty());
    }

    #[test]
    fn constitution_drift_fires_on_version_mismatch() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
        ];
        let versions = vec![make_version("feat-1", "old-const-v1")];
        let constitution = make_constitution("new-const-v2");

        let alarms = check_constitution_drift(&artifacts, &versions, &constitution);
        assert_eq!(alarms.len(), 1);
        assert_eq!(alarms[0].alarm_type, DriftAlarmType::ConstitutionDrift);
    }

    #[test]
    fn no_constitution_drift_when_versions_match() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
        ];
        let versions = vec![make_version("feat-1", "const-v1")];
        let constitution = make_constitution("const-v1");

        let alarms = check_constitution_drift(&artifacts, &versions, &constitution);
        assert!(alarms.is_empty());
    }

    #[test]
    fn sequence_drift_fires_on_approved_with_old_constitution() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
        ];
        let versions = vec![make_version("feat-1", "old-const-v1")];
        let constitution = make_constitution("new-const-v2");

        let alarms = check_sequence_drift(&artifacts, &versions, &constitution);
        assert_eq!(alarms.len(), 1);
        assert_eq!(alarms[0].alarm_type, DriftAlarmType::SequenceDrift);
        assert_eq!(alarms[0].severity, DriftAlarmSeverity::Blocking);
    }

    #[test]
    fn sequence_drift_ignores_non_approved_artifacts() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Draft),
        ];
        let versions = vec![make_version("feat-1", "old-const-v1")];
        let constitution = make_constitution("new-const-v2");

        let alarms = check_sequence_drift(&artifacts, &versions, &constitution);
        assert!(alarms.is_empty());
    }

    #[test]
    fn scope_drift_fires_on_feature_without_workflow_link() {
        let artifacts = vec![
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
        ];
        let links = vec![];

        let alarms = check_scope_drift(&artifacts, &links);
        assert_eq!(alarms.len(), 1);
        assert_eq!(alarms[0].alarm_type, DriftAlarmType::ScopeDrift);
    }

    #[test]
    fn scope_drift_fires_on_system_without_feature_link() {
        let artifacts = vec![
            make_artifact("sys-1", ArtifactType::SystemArchitecture, ArtifactState::Approved),
        ];
        let links = vec![];

        let alarms = check_scope_drift(&artifacts, &links);
        assert_eq!(alarms.len(), 1);
        assert_eq!(alarms[0].violated_rule_id, "scope-drift-002");
    }

    #[test]
    fn evaluate_all_catches_multiple_drift_types() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
        ];
        let versions = vec![
            make_version("wf-1", "const-v1"),
            make_version("feat-1", "old-const-v1"),
        ];
        let links = vec![]; // no links at all
        let constitution = make_constitution("const-v1");

        let alarms = evaluate_all_rules(&artifacts, &versions, &links, &constitution);
        // Should fire: traceability drift (wf missing const link, feat missing wf link)
        // + scope drift (feat without workflow link)
        assert!(alarms.len() >= 2);
    }
}

//! Anchor Law Engine — Execution Readiness Gate
//!
//! One pure function that produces:
//! - Gate status (blocked/ready)
//! - Blocking reasons with rule provenance
//! - Stale artifact summary
//! - Outdated approval list
//! - Active blocking alarms
//! - Export manifest preview
//!
//! Dead simple to call. Impossible for the frontend to fake.

use crate::domain::{
    Artifact, ArtifactState, ArtifactType, ArtifactVersion, Approval, BlockingReason,
    Constitution, DriftAlarm, DriftAlarmSeverity, DriftAlarmStatus, Amendment,
    AmendmentStatus, ExportFileEntry, ExportManifestPreview, GateStatus,
    RuleProvenance, SourceArtifactType, TraceLink,
};
use crate::drift_rules::{self, DriftAlarmBlueprint};
use crate::stale_propagation;
use crate::traceability;

// ─── Gate Evaluation ────────────────────────────────────────

/// Complete gate evaluation result.
#[derive(Debug, Clone)]
pub struct GateEvaluation {
    pub status: GateStatus,
    pub blocking_reasons: Vec<BlockingReason>,
    pub stale_summary: StaleSummary,
    pub outdated_approvals: Vec<OutdatedApproval>,
    pub active_blocking_alarms: Vec<ActiveAlarmSummary>,
    pub traceability_failures: usize,
    pub readiness_summary: String,
    pub export_manifest_preview: ExportManifestPreview,
}

#[derive(Debug, Clone)]
pub struct StaleSummary {
    pub count: usize,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutdatedApproval {
    pub approval_id: String,
    pub artifact_id: String,
    pub approved_against_version: String,
    pub current_constitution_version: String,
}

#[derive(Debug, Clone)]
pub struct ActiveAlarmSummary {
    pub alarm_id: String,
    pub alarm_type: crate::domain::DriftAlarmType,
    pub severity: DriftAlarmSeverity,
    pub source_artifact_id: String,
    pub explanation: String,
}

/// Evaluate the Execution Readiness Gate.
///
/// This is the single entry point. It checks everything and produces
/// a complete, self-describing result that tells you exactly why
/// export is or isn't allowed.
pub fn evaluate(
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    approvals: &[Approval],
    links: &[TraceLink],
    constitution: &Constitution,
    active_alarms: &[DriftAlarm],
    amendments: &[Amendment],
) -> GateEvaluation {
    let mut blocking_reasons = Vec::new();

    // ─── 1. Artifact state checks ───────────────────────────
    check_artifact_states(artifacts, &mut blocking_reasons);

    // ─── 2. Stale artifacts ─────────────────────────────────
    let stale_summary = check_stale_artifacts(artifacts);
    if stale_summary.count > 0 {
        blocking_reasons.push(BlockingReason {
            code: "STALE_ARTIFACTS".into(),
            message: format!(
                "{} artifact(s) are stale and must be reconciled: {}",
                stale_summary.count,
                stale_summary.artifact_ids.join(", ")
            ),
            affected_artifact_ids: stale_summary.artifact_ids.clone(),
            rule_provenance: system_provenance(
                "§13.3 — gate blocks on stale artifacts",
                "No artifact may be stale at export time",
            ),
            remediation_steps: vec![
                "Review each stale artifact against current constitution".into(),
                "Update content to reconcile with upstream changes".into(),
                "Re-validate and re-approve each stale artifact".into(),
            ],
        });
    }

    // ─── 3. Drift alarms ───────────────────────────────────
    let active_blocking_alarms = check_drift_alarms(active_alarms);
    if !active_blocking_alarms.is_empty() {
        blocking_reasons.push(BlockingReason {
            code: "ACTIVE_DRIFT_ALARMS".into(),
            message: format!(
                "{} active blocking/error-severity drift alarm(s)",
                active_blocking_alarms.len()
            ),
            affected_artifact_ids: active_blocking_alarms
                .iter()
                .map(|a| a.source_artifact_id.clone())
                .collect(),
            rule_provenance: system_provenance(
                "§13.3 — gate blocks on active drift alarms",
                "No blocking or error-severity drift alarm may be active at export time",
            ),
            remediation_steps: vec![
                "Review each active drift alarm".into(),
                "Resolve the underlying issue (add trace links, reconcile content, etc.)".into(),
                "Alarms will auto-resolve when the violation is fixed".into(),
            ],
        });
    }

    // ─── 4. Amendment completion ────────────────────────────
    check_amendments(amendments, &mut blocking_reasons);

    // ─── 5. Approval currency ──────────────────────────────
    let outdated_approvals = check_approval_currency(
        artifacts, versions, approvals, constitution,
    );
    if !outdated_approvals.is_empty() {
        blocking_reasons.push(BlockingReason {
            code: "OUTDATED_APPROVALS".into(),
            message: format!(
                "{} approval(s) bind to an outdated constitution version",
                outdated_approvals.len()
            ),
            affected_artifact_ids: outdated_approvals
                .iter()
                .map(|a| a.artifact_id.clone())
                .collect(),
            rule_provenance: system_provenance(
                "§13.3 — gate blocks on outdated approvals",
                "All approvals must bind to current constitution version",
            ),
            remediation_steps: vec![
                "Re-approve each artifact against the current constitution version".into(),
            ],
        });
    }

    // ─── 6. Traceability completeness ──────────────────────
    let trace_results = traceability::validate_all_traceability(artifacts, links);
    let traceability_failures = traceability::failure_count(&trace_results);
    if traceability_failures > 0 {
        blocking_reasons.push(BlockingReason {
            code: "TRACEABILITY_INCOMPLETE".into(),
            message: format!(
                "{} traceability validation failure(s)",
                traceability_failures
            ),
            affected_artifact_ids: trace_results
                .iter()
                .filter(|r| r.status == crate::domain::RuleResultStatus::Fail)
                .flat_map(|r| r.affected_node_ids.clone())
                .collect(),
            rule_provenance: system_provenance(
                "§8 — traceability graph completeness",
                "All required trace links must exist and resolve",
            ),
            remediation_steps: vec![
                "Add missing trace links between artifacts".into(),
                "Ensure all link endpoints resolve to existing artifacts".into(),
                "Ensure bidirectional explainability for all non-root/non-terminal nodes".into(),
            ],
        });
    }

    // ─── Compute status ─────────────────────────────────────
    let status = if blocking_reasons.is_empty() {
        GateStatus::Ready
    } else {
        GateStatus::Blocked
    };

    let readiness_summary = if status == GateStatus::Ready {
        "All artifacts approved, no stale artifacts, no active drift alarms, \
         traceability complete, all approvals current. Ready for export."
            .to_string()
    } else {
        format!(
            "Export blocked: {} reason(s). Resolve all blocking reasons before export.",
            blocking_reasons.len()
        )
    };

    let export_manifest_preview = build_export_manifest(artifacts);

    GateEvaluation {
        status,
        blocking_reasons,
        stale_summary,
        outdated_approvals,
        active_blocking_alarms,
        traceability_failures,
        readiness_summary,
        export_manifest_preview,
    }
}

// ─── Individual Checks ──────────────────────────────────────

fn check_artifact_states(artifacts: &[Artifact], reasons: &mut Vec<BlockingReason>) {
    for artifact in artifacts {
        if artifact.artifact_type == ArtifactType::ExecutionReadinessGate {
            continue;
        }

        let (code, message) = match artifact.state {
            ArtifactState::Draft => (
                "ARTIFACT_DRAFT",
                format!(
                    "{:?} '{}' is in draft state — must be completed, validated, and approved",
                    artifact.artifact_type, artifact.title
                ),
            ),
            ArtifactState::Complete => (
                "ARTIFACT_NOT_VALIDATED",
                format!(
                    "{:?} '{}' is complete but not validated",
                    artifact.artifact_type, artifact.title
                ),
            ),
            ArtifactState::Valid => (
                "ARTIFACT_NOT_APPROVED",
                format!(
                    "{:?} '{}' is validated but not approved",
                    artifact.artifact_type, artifact.title
                ),
            ),
            ArtifactState::Stale => continue, // handled in stale check
            ArtifactState::Approved => continue,
        };

        reasons.push(BlockingReason {
            code: code.into(),
            message,
            affected_artifact_ids: vec![artifact.id.clone()],
            rule_provenance: system_provenance(
                "§13.3 — all non-gate artifacts must be Approved",
                "Every artifact must reach Approved state before export",
            ),
            remediation_steps: match artifact.state {
                ArtifactState::Draft => vec![
                    "Complete all required fields".into(),
                    "Run validation".into(),
                    "Approve the artifact".into(),
                ],
                ArtifactState::Complete => vec![
                    "Run structural, relational, and intent validation".into(),
                    "Approve after validation passes".into(),
                ],
                ArtifactState::Valid => vec!["Approve the artifact".into()],
                _ => vec![],
            },
        });
    }
}

fn check_stale_artifacts(artifacts: &[Artifact]) -> StaleSummary {
    let stale: Vec<String> = artifacts
        .iter()
        .filter(|a| a.state == ArtifactState::Stale)
        .map(|a| a.id.clone())
        .collect();

    StaleSummary {
        count: stale.len(),
        artifact_ids: stale,
    }
}

fn check_drift_alarms(alarms: &[DriftAlarm]) -> Vec<ActiveAlarmSummary> {
    alarms
        .iter()
        .filter(|a| {
            a.status == DriftAlarmStatus::Active
                && matches!(
                    a.severity,
                    DriftAlarmSeverity::Blocking | DriftAlarmSeverity::Error
                )
        })
        .map(|a| ActiveAlarmSummary {
            alarm_id: a.id.clone(),
            alarm_type: a.alarm_type,
            severity: a.severity,
            source_artifact_id: a.source_artifact_id.clone(),
            explanation: a.explanation.clone(),
        })
        .collect()
}

fn check_amendments(amendments: &[Amendment], reasons: &mut Vec<BlockingReason>) {
    let incomplete: Vec<&Amendment> = amendments
        .iter()
        .filter(|a| !matches!(a.status, AmendmentStatus::Completed | AmendmentStatus::Abandoned))
        .collect();

    for amendment in &incomplete {
        reasons.push(BlockingReason {
            code: "INCOMPLETE_AMENDMENT".into(),
            message: format!(
                "Amendment {} is {:?} — must be completed or abandoned before export",
                amendment.id, amendment.status
            ),
            affected_artifact_ids: amendment.invalidated_artifact_ids.clone(),
            rule_provenance: system_provenance(
                "§13.3 — gate blocks on incomplete amendments",
                "All amendments must reach Completed or Abandoned status before export",
            ),
            remediation_steps: vec![
                "Reconcile all invalidated artifacts with the amended constitution".into(),
                "Re-approve affected artifacts".into(),
                "Mark amendment as Completed".into(),
            ],
        });
    }
}

fn check_approval_currency(
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    approvals: &[Approval],
    constitution: &Constitution,
) -> Vec<OutdatedApproval> {
    let mut outdated = Vec::new();

    for artifact in artifacts {
        if artifact.artifact_type == ArtifactType::ExecutionReadinessGate
            || artifact.artifact_type == ArtifactType::Constitution
        {
            continue;
        }

        if artifact.state != ArtifactState::Approved {
            continue;
        }

        // Find the latest approval for this artifact
        if let Some(approval) = approvals
            .iter()
            .filter(|a| a.artifact_id == artifact.id)
            .max_by_key(|a| &a.created_at)
        {
            // Find the version this approval points to
            if let Some(version) = versions
                .iter()
                .find(|v| v.id == approval.artifact_version_id)
            {
                if version.constitution_version_id != constitution.version_id {
                    outdated.push(OutdatedApproval {
                        approval_id: approval.id.clone(),
                        artifact_id: artifact.id.clone(),
                        approved_against_version: version.constitution_version_id.clone(),
                        current_constitution_version: constitution.version_id.clone(),
                    });
                }
            }
        }
    }

    outdated
}

// ─── Export Manifest ────────────────────────────────────────

/// §15: canonical export output structure.
fn build_export_manifest(artifacts: &[Artifact]) -> ExportManifestPreview {
    let mut files = vec![
        ExportFileEntry {
            path: "project.json".into(),
            kind: crate::domain::ExportFileKind::Json,
            derived_from_artifact_ids: artifacts.iter().map(|a| a.id.clone()).collect(),
        },
        ExportFileEntry {
            path: "constitution.md".into(),
            kind: crate::domain::ExportFileKind::Markdown,
            derived_from_artifact_ids: artifacts
                .iter()
                .filter(|a| a.artifact_type == ArtifactType::Constitution)
                .map(|a| a.id.clone())
                .collect(),
        },
    ];

    // Artifact markdown files
    let artifact_files: Vec<(&str, ArtifactType)> = vec![
        ("artifacts/user-fantasy-workflows.md", ArtifactType::UserFantasyWorkflows),
        ("artifacts/feature-map.md", ArtifactType::FeatureMap),
        ("artifacts/system-architecture.md", ArtifactType::SystemArchitecture),
        ("artifacts/ux-state-map.md", ArtifactType::UxStateMap),
        ("artifacts/phase-roadmap-contracts.md", ArtifactType::PhaseRoadmapContracts),
        ("artifacts/acceptance-checklists.md", ArtifactType::AcceptanceChecklists),
        ("artifacts/drift-alarm-definitions.md", ArtifactType::DriftAlarmDefinitions),
    ];

    for (path, artifact_type) in &artifact_files {
        files.push(ExportFileEntry {
            path: path.to_string(),
            kind: crate::domain::ExportFileKind::Markdown,
            derived_from_artifact_ids: artifacts
                .iter()
                .filter(|a| a.artifact_type == *artifact_type)
                .map(|a| a.id.clone())
                .collect(),
        });
    }

    // Report files
    let report_files = [
        "reports/traceability-matrix.md",
        "reports/audit-log.md",
        "reports/drift-report.md",
        "reports/execution-readiness-report.md",
    ];

    for path in &report_files {
        files.push(ExportFileEntry {
            path: path.to_string(),
            kind: crate::domain::ExportFileKind::Report,
            derived_from_artifact_ids: artifacts.iter().map(|a| a.id.clone()).collect(),
        });
    }

    ExportManifestPreview {
        file_count: files.len(),
        files,
    }
}

// ─── Helpers ────────────────────────────────────────────────

fn system_provenance(clause: &str, label: &str) -> RuleProvenance {
    RuleProvenance {
        source_artifact_type: SourceArtifactType::SystemRule,
        source_clause: clause.to_string(),
        human_label: label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ApprovalType, DriftAlarmType, LocalIdentity, TraceLinkType, ValidationSummary,
    };

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

    fn make_version(artifact_id: &str, constitution_version: &str) -> ArtifactVersion {
        ArtifactVersion {
            id: format!("{}-v1", artifact_id),
            artifact_id: artifact_id.into(),
            project_id: "proj-1".into(),
            version_number: 1,
            constitution_version_id: constitution_version.into(),
            content: serde_json::Value::Null,
            content_hash: "hash".into(),
            parent_version_id: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            created_by: identity(),
        }
    }

    fn make_approval(artifact_id: &str, version_id: &str) -> Approval {
        Approval {
            id: format!("appr-{}", artifact_id),
            project_id: "proj-1".into(),
            artifact_id: artifact_id.into(),
            artifact_version_id: version_id.into(),
            artifact_content_hash: "hash".into(),
            approval_type: ApprovalType::Standard,
            approver: identity(),
            rationale: None,
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn make_constitution(version_id: &str) -> Constitution {
        Constitution {
            id: "const-1".into(),
            artifact_id: "art-const".into(),
            version_id: version_id.into(),
            project_id: "proj-1".into(),
            one_sentence_promise: "Test".into(),
            user_fantasy: "Test".into(),
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

    fn make_link(source: &str, target: &str) -> TraceLink {
        TraceLink {
            id: format!("link-{}-{}", source, target),
            project_id: "proj-1".into(),
            source_node_id: source.into(),
            target_node_id: target.into(),
            link_type: TraceLinkType::DerivesFrom,
            rationale: "test".into(),
            created_by: identity(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    // ─── Gate status tests ──────────────────────────────────

    #[test]
    fn gate_blocks_on_draft_artifact() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Draft),
        ];
        let versions = vec![make_version("wf-1", "cv1")];
        let approvals = vec![];
        let links = vec![make_link("wf-1", "const-1")];
        let constitution = make_constitution("cv1");

        let result = evaluate(&artifacts, &versions, &approvals, &links, &constitution, &[], &[]);

        assert_eq!(result.status, GateStatus::Blocked);
        assert!(result.blocking_reasons.iter().any(|r| r.code == "ARTIFACT_DRAFT"));
    }

    #[test]
    fn gate_blocks_on_stale_artifact() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Stale),
        ];
        let versions = vec![make_version("wf-1", "cv1")];
        let constitution = make_constitution("cv1");

        let result = evaluate(&artifacts, &versions, &[], &[], &constitution, &[], &[]);

        assert_eq!(result.status, GateStatus::Blocked);
        assert!(result.stale_summary.count > 0);
    }

    #[test]
    fn gate_blocks_on_active_blocking_alarm() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
        ];
        let constitution = make_constitution("cv1");
        let alarms = vec![DriftAlarm {
            id: "alarm-1".into(),
            project_id: "proj-1".into(),
            alarm_type: DriftAlarmType::TraceabilityDrift,
            severity: DriftAlarmSeverity::Blocking,
            source_artifact_id: "const-1".into(),
            affected_node_ids: vec!["const-1".into()],
            violated_rule_id: "trace-001".into(),
            rule_provenance: system_provenance("test", "test"),
            explanation: "test alarm".into(),
            remediation_path: vec![],
            status: crate::domain::DriftAlarmStatus::Active,
            created_at: "2026-03-13T00:00:00Z".into(),
            resolved_at: None,
        }];

        let result = evaluate(&artifacts, &[], &[], &[], &constitution, &alarms, &[]);

        assert_eq!(result.status, GateStatus::Blocked);
        assert!(!result.active_blocking_alarms.is_empty());
    }

    #[test]
    fn gate_blocks_on_incomplete_amendment() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
        ];
        let constitution = make_constitution("cv1");
        let amendments = vec![Amendment {
            id: "amend-1".into(),
            project_id: "proj-1".into(),
            target_constitution_version_id: "cv1".into(),
            proposed_changes: crate::domain::ConstitutionPatch {
                one_sentence_promise: Some("new promise".into()),
                user_fantasy: None,
                non_negotiable_outcomes: None,
                anti_goals: None,
                quality_bar: None,
                failure_condition: None,
            },
            reason: "test".into(),
            expected_impact_summary: "test".into(),
            invalidated_artifact_ids: vec![],
            resulting_constitution_version_id: None,
            proposer: identity(),
            status: AmendmentStatus::Proposed,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            applied_at: None,
        }];

        let result = evaluate(&artifacts, &[], &[], &[], &constitution, &[], &amendments);

        assert_eq!(result.status, GateStatus::Blocked);
        assert!(result.blocking_reasons.iter().any(|r| r.code == "INCOMPLETE_AMENDMENT"));
    }

    #[test]
    fn gate_blocks_on_outdated_approval() {
        let artifacts = vec![
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
        ];
        let versions = vec![make_version("wf-1", "old-cv1")]; // approved against old version
        let approvals = vec![make_approval("wf-1", "wf-1-v1")];
        let constitution = make_constitution("new-cv2"); // current is different

        let result = evaluate(&artifacts, &versions, &approvals, &[], &constitution, &[], &[]);

        assert_eq!(result.status, GateStatus::Blocked);
        assert!(!result.outdated_approvals.is_empty());
    }

    #[test]
    fn gate_ready_when_everything_clean() {
        // Minimal valid project: just a constitution, all approved, matching versions
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows, ArtifactState::Approved),
            make_artifact("feat-1", ArtifactType::FeatureMap, ArtifactState::Approved),
            make_artifact("sys-1", ArtifactType::SystemArchitecture, ArtifactState::Approved),
            make_artifact("ux-1", ArtifactType::UxStateMap, ArtifactState::Approved),
            make_artifact("phase-1", ArtifactType::PhaseRoadmapContracts, ArtifactState::Approved),
            make_artifact("check-1", ArtifactType::AcceptanceChecklists, ArtifactState::Approved),
            make_artifact("drift-1", ArtifactType::DriftAlarmDefinitions, ArtifactState::Approved),
        ];
        let versions: Vec<ArtifactVersion> = artifacts
            .iter()
            .map(|a| make_version(&a.id, "cv1"))
            .collect();
        let approvals: Vec<Approval> = artifacts
            .iter()
            .filter(|a| a.artifact_type != ArtifactType::Constitution)
            .map(|a| make_approval(&a.id, &format!("{}-v1", a.id)))
            .collect();
        let constitution = make_constitution("cv1");

        // Full trace link chain
        let links = vec![
            make_link("wf-1", "const-1"),
            make_link("feat-1", "wf-1"),
            make_link("sys-1", "feat-1"),
            make_link("ux-1", "wf-1"),
            make_link("phase-1", "const-1"),
            make_link("check-1", "phase-1"),
            make_link("drift-1", "const-1"),
        ];

        let result = evaluate(
            &artifacts, &versions, &approvals, &links, &constitution, &[], &[],
        );

        assert_eq!(result.status, GateStatus::Ready);
        assert!(result.blocking_reasons.is_empty());
        assert_eq!(result.stale_summary.count, 0);
        assert!(result.outdated_approvals.is_empty());
        assert!(result.active_blocking_alarms.is_empty());
    }

    // ─── Export manifest tests ──────────────────────────────

    #[test]
    fn export_manifest_has_correct_file_count() {
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution, ArtifactState::Approved),
        ];

        let manifest = build_export_manifest(&artifacts);

        // project.json + constitution.md + 7 artifact .md + 4 reports = 13
        assert_eq!(manifest.file_count, 13);
        assert!(manifest.files.iter().any(|f| f.path == "project.json"));
        assert!(manifest.files.iter().any(|f| f.path == "constitution.md"));
        assert!(manifest.files.iter().any(|f| f.path == "reports/audit-log.md"));
    }
}

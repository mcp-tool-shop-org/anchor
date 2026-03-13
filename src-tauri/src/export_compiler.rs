//! Anchor Export Compiler
//!
//! Takes validated project state and renders the canonical export package.
//! Only callable after the readiness gate returns `Ready`.
//!
//! Produces:
//! - project.json (machine-readable canonical source)
//! - constitution.md
//! - artifacts/*.md (one per artifact type)
//! - reports/traceability-matrix.md
//! - reports/audit-log.md
//! - reports/drift-report.md
//! - reports/execution-readiness-report.md

use crate::domain::*;
use crate::readiness_gate::{self, GateEvaluation};
use crate::traceability;

// ─── Export Package ─────────────────────────────────────────

/// A rendered export file: path + utf-8 content.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub path: String,
    pub content: String,
}

/// The complete export package, ready to write to disk.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPackage {
    pub project_name: String,
    pub schema_version: String,
    pub files: Vec<ExportFile>,
    pub manifest: ExportManifestPreview,
}

/// Error returned when export is blocked.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBlocked {
    pub gate_evaluation: GateEvaluation,
}

// ─── Input Bundle ───────────────────────────────────────────

/// Everything the export compiler needs. Caller assembles this
/// from the project store.
pub struct ExportInput<'a> {
    pub project: &'a Project,
    pub constitution: &'a Constitution,
    pub artifacts: &'a [Artifact],
    pub versions: &'a [ArtifactVersion],
    pub approvals: &'a [Approval],
    pub links: &'a [TraceLink],
    pub alarms: &'a [DriftAlarm],
    pub amendments: &'a [Amendment],
    pub audit_events: &'a [AuditEvent],
}

// ─── Compile ────────────────────────────────────────────────

/// Compile the export package. Returns `Err` if the gate is blocked.
pub fn compile(input: &ExportInput) -> Result<ExportPackage, ExportBlocked> {
    // Run the gate first — no export if blocked
    let gate = readiness_gate::evaluate(
        input.artifacts,
        input.versions,
        input.approvals,
        input.links,
        input.constitution,
        input.alarms,
        input.amendments,
    );

    if gate.status == GateStatus::Blocked {
        return Err(ExportBlocked {
            gate_evaluation: gate,
        });
    }

    let mut files = Vec::new();

    // 1. project.json — canonical machine-readable source
    files.push(render_project_json(input));

    // 2. constitution.md
    files.push(render_constitution_md(input.constitution));

    // 3. artifacts/*.md
    for artifact in input.artifacts {
        if artifact.artifact_type == ArtifactType::Constitution
            || artifact.artifact_type == ArtifactType::ExecutionReadinessGate
        {
            continue;
        }
        let version = input
            .versions
            .iter()
            .find(|v| v.id == artifact.current_version_id);
        files.push(render_artifact_md(artifact, version));
    }

    // 4. reports/
    files.push(render_traceability_matrix(
        input.artifacts,
        input.links,
    ));
    files.push(render_audit_log(input.audit_events));
    files.push(render_drift_report(input.alarms));
    files.push(render_readiness_report(&gate));

    let manifest = ExportManifestPreview {
        file_count: files.len(),
        files: files
            .iter()
            .map(|f| ExportFileEntry {
                path: f.path.clone(),
                kind: infer_kind(&f.path),
                derived_from_artifact_ids: vec![],
            })
            .collect(),
    };

    Ok(ExportPackage {
        project_name: input.project.name.clone(),
        schema_version: SCHEMA_VERSION.to_string(),
        files,
        manifest,
    })
}

fn infer_kind(path: &str) -> ExportFileKind {
    if path.ends_with(".json") {
        ExportFileKind::Json
    } else if path.starts_with("reports/") {
        ExportFileKind::Report
    } else {
        ExportFileKind::Markdown
    }
}

// ─── Renderers ──────────────────────────────────────────────

fn render_project_json(input: &ExportInput) -> ExportFile {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ProjectExport<'a> {
        schema_version: &'a str,
        project: &'a Project,
        constitution: &'a Constitution,
        artifacts: &'a [Artifact],
        versions: &'a [ArtifactVersion],
        approvals: &'a [Approval],
        trace_links: &'a [TraceLink],
        amendments: &'a [Amendment],
        audit_events: &'a [AuditEvent],
    }

    let export = ProjectExport {
        schema_version: SCHEMA_VERSION,
        project: input.project,
        constitution: input.constitution,
        artifacts: input.artifacts,
        versions: input.versions,
        approvals: input.approvals,
        trace_links: input.links,
        amendments: input.amendments,
        audit_events: input.audit_events,
    };

    ExportFile {
        path: "project.json".into(),
        content: serde_json::to_string_pretty(&export).unwrap_or_else(|e| {
            format!("{{\"error\": \"serialization failed: {}\"}}", e)
        }),
    }
}

fn render_constitution_md(c: &Constitution) -> ExportFile {
    let mut md = String::new();
    md.push_str("# Product Constitution\n\n");
    md.push_str(&format!("**Version:** {}\n\n", c.version_id));
    md.push_str("## One-Sentence Promise\n\n");
    md.push_str(&c.one_sentence_promise);
    md.push_str("\n\n## User Fantasy\n\n");
    md.push_str(&c.user_fantasy);
    md.push_str("\n\n## Non-Negotiable Outcomes\n\n");
    for outcome in &c.non_negotiable_outcomes {
        md.push_str(&format!("- {}\n", outcome));
    }
    md.push_str("\n## Anti-Goals\n\n");
    for ag in &c.anti_goals {
        md.push_str(&format!("- {}\n", ag));
    }
    md.push_str("\n## Quality Bar\n\n");
    md.push_str(&c.quality_bar);
    md.push_str("\n\n## Failure Condition\n\n");
    md.push_str(&c.failure_condition);
    md.push('\n');

    if let Some(ref by) = c.approved_by {
        md.push_str(&format!(
            "\n---\n\nApproved by **{}** at {}\n",
            by.display_name,
            c.approved_at.as_deref().unwrap_or("unknown")
        ));
    }

    ExportFile {
        path: "constitution.md".into(),
        content: md,
    }
}

fn render_artifact_md(artifact: &Artifact, version: Option<&ArtifactVersion>) -> ExportFile {
    let filename = match artifact.artifact_type {
        ArtifactType::UserFantasyWorkflows => "user-fantasy-workflows",
        ArtifactType::FeatureMap => "feature-map",
        ArtifactType::SystemArchitecture => "system-architecture",
        ArtifactType::UxStateMap => "ux-state-map",
        ArtifactType::PhaseRoadmapContracts => "phase-roadmap-contracts",
        ArtifactType::AcceptanceChecklists => "acceptance-checklists",
        ArtifactType::DriftAlarmDefinitions => "drift-alarm-definitions",
        _ => "unknown",
    };

    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", artifact.title));
    md.push_str(&format!("**Type:** {:?}\n", artifact.artifact_type));
    md.push_str(&format!("**State:** {:?}\n", artifact.state));

    if let Some(v) = version {
        md.push_str(&format!("**Version:** {} (hash: {})\n", v.version_number, v.content_hash));
        md.push_str(&format!(
            "**Constitution Version:** {}\n",
            v.constitution_version_id
        ));
        md.push_str("\n## Content\n\n");
        md.push_str("```json\n");
        md.push_str(
            &serde_json::to_string_pretty(&v.content).unwrap_or_else(|_| "null".into()),
        );
        md.push_str("\n```\n");
    } else {
        md.push_str("\n*No version snapshot available.*\n");
    }

    ExportFile {
        path: format!("artifacts/{}.md", filename),
        content: md,
    }
}

fn render_traceability_matrix(
    artifacts: &[Artifact],
    links: &[TraceLink],
) -> ExportFile {
    let mut md = String::new();
    md.push_str("# Traceability Matrix\n\n");
    md.push_str("| Source | → | Target | Link Type | Rationale |\n");
    md.push_str("|--------|---|--------|-----------|----------|\n");

    for link in links {
        let source_name = artifacts
            .iter()
            .find(|a| a.id == link.source_node_id)
            .map(|a| a.title.as_str())
            .unwrap_or(&link.source_node_id);
        let target_name = artifacts
            .iter()
            .find(|a| a.id == link.target_node_id)
            .map(|a| a.title.as_str())
            .unwrap_or(&link.target_node_id);

        md.push_str(&format!(
            "| {} | → | {} | {:?} | {} |\n",
            source_name, target_name, link.link_type, link.rationale
        ));
    }

    // Validation summary
    let results = traceability::validate_all_traceability(artifacts, links);
    let failures = traceability::failure_count(&results);
    md.push_str(&format!(
        "\n**Validation:** {} check(s), {} failure(s)\n",
        results.len(),
        failures
    ));

    ExportFile {
        path: "reports/traceability-matrix.md".into(),
        content: md,
    }
}

fn render_audit_log(events: &[AuditEvent]) -> ExportFile {
    let mut md = String::new();
    md.push_str("# Audit Log\n\n");
    md.push_str("| Time | Event | Actor |\n");
    md.push_str("|------|-------|-------|\n");

    for event in events {
        let actor = match &event.actor {
            AuditActor::User(u) => u.display_name.as_str(),
            AuditActor::System => "System",
        };
        md.push_str(&format!(
            "| {} | {:?} | {} |\n",
            event.occurred_at, event.event_type, actor
        ));
    }

    md.push_str(&format!("\n**Total events:** {}\n", events.len()));

    ExportFile {
        path: "reports/audit-log.md".into(),
        content: md,
    }
}

fn render_drift_report(alarms: &[DriftAlarm]) -> ExportFile {
    let mut md = String::new();
    md.push_str("# Drift Report\n\n");

    let active: Vec<_> = alarms
        .iter()
        .filter(|a| a.status == DriftAlarmStatus::Active)
        .collect();
    let resolved: Vec<_> = alarms
        .iter()
        .filter(|a| a.status == DriftAlarmStatus::Resolved)
        .collect();

    md.push_str(&format!(
        "**Active:** {} | **Resolved:** {} | **Total:** {}\n\n",
        active.len(),
        resolved.len(),
        alarms.len()
    ));

    if !active.is_empty() {
        md.push_str("## Active Alarms\n\n");
        for alarm in &active {
            md.push_str(&format!("### {} ({:?})\n\n", alarm.id, alarm.alarm_type));
            md.push_str(&format!("- **Severity:** {:?}\n", alarm.severity));
            md.push_str(&format!("- **Source:** {}\n", alarm.source_artifact_id));
            md.push_str(&format!("- **Explanation:** {}\n", alarm.explanation));
            md.push_str("- **Remediation:**\n");
            for step in &alarm.remediation_path {
                md.push_str(&format!("  1. {}\n", step));
            }
            md.push('\n');
        }
    }

    if !resolved.is_empty() {
        md.push_str("## Resolved Alarms\n\n");
        md.push_str("| Alarm | Type | Resolved At |\n");
        md.push_str("|-------|------|------------|\n");
        for alarm in &resolved {
            md.push_str(&format!(
                "| {} | {:?} | {} |\n",
                alarm.id,
                alarm.alarm_type,
                alarm.resolved_at.as_deref().unwrap_or("unknown")
            ));
        }
    }

    ExportFile {
        path: "reports/drift-report.md".into(),
        content: md,
    }
}

fn render_readiness_report(gate: &GateEvaluation) -> ExportFile {
    let mut md = String::new();
    md.push_str("# Execution Readiness Report\n\n");
    md.push_str(&format!("**Status:** {:?}\n\n", gate.status));
    md.push_str(&gate.readiness_summary);
    md.push_str("\n\n");

    if !gate.blocking_reasons.is_empty() {
        md.push_str("## Blocking Reasons\n\n");
        for reason in &gate.blocking_reasons {
            md.push_str(&format!("### {}\n\n", reason.code));
            md.push_str(&format!("{}\n\n", reason.message));
            md.push_str("**Remediation:**\n");
            for step in &reason.remediation_steps {
                md.push_str(&format!("1. {}\n", step));
            }
            md.push('\n');
        }
    }

    md.push_str("## Export Manifest\n\n");
    md.push_str(&format!(
        "**Files:** {}\n\n",
        gate.export_manifest_preview.file_count
    ));
    md.push_str("| Path | Kind |\n");
    md.push_str("|------|------|\n");
    for file in &gate.export_manifest_preview.files {
        md.push_str(&format!("| {} | {:?} |\n", file.path, file.kind));
    }

    // Stale summary
    if gate.stale_summary.count > 0 {
        md.push_str(&format!(
            "\n## Stale Artifacts ({})\n\n",
            gate.stale_summary.count
        ));
        for id in &gate.stale_summary.artifact_ids {
            md.push_str(&format!("- {}\n", id));
        }
    }

    // Outdated approvals
    if !gate.outdated_approvals.is_empty() {
        md.push_str(&format!(
            "\n## Outdated Approvals ({})\n\n",
            gate.outdated_approvals.len()
        ));
        for oa in &gate.outdated_approvals {
            md.push_str(&format!(
                "- **{}**: approved against `{}`, current is `{}`\n",
                oa.artifact_id, oa.approved_against_version, oa.current_constitution_version
            ));
        }
    }

    ExportFile {
        path: "reports/execution-readiness-report.md".into(),
        content: md,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::*;

    fn identity() -> LocalIdentity {
        LocalIdentity {
            id: "user-1".into(),
            display_name: "Test User".into(),
        }
    }

    fn make_project() -> Project {
        Project {
            id: "proj-1".into(),
            schema_version: SCHEMA_VERSION.into(),
            name: "Test Project".into(),
            slug: "test-project".into(),
            description: "A test".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            created_by: identity(),
            current_constitution_version_id: "cv1".into(),
            artifact_ids: vec![],
            active_amendment_id: None,
            settings: ProjectSettings::default(),
        }
    }

    fn make_constitution() -> Constitution {
        Constitution {
            id: "const-1".into(),
            artifact_id: "art-const".into(),
            version_id: "cv1".into(),
            project_id: "proj-1".into(),
            one_sentence_promise: "Build something great".into(),
            user_fantasy: "The user opens the app and everything just works".into(),
            non_negotiable_outcomes: vec!["Coherence".into(), "Traceability".into()],
            anti_goals: vec!["No cloud dependency".into()],
            quality_bar: "High".into(),
            failure_condition: "Ceremony without coherence".into(),
            locked: true,
            content_hash: "hash-const".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            approved_at: Some("2026-03-13T00:00:00Z".into()),
            approved_by: Some(identity()),
            parent_version_id: None,
        }
    }

    fn make_artifact(id: &str, art_type: ArtifactType) -> Artifact {
        Artifact {
            id: id.into(),
            project_id: "proj-1".into(),
            artifact_type: art_type,
            title: format!("{:?}", art_type),
            current_version_id: format!("{}-v1", id),
            state: ArtifactState::Approved,
            validation_summary: ValidationSummary::default(),
            latest_approval_id: None,
            stale_reason: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn make_version(artifact_id: &str) -> ArtifactVersion {
        ArtifactVersion {
            id: format!("{}-v1", artifact_id),
            artifact_id: artifact_id.into(),
            project_id: "proj-1".into(),
            version_number: 1,
            constitution_version_id: "cv1".into(),
            content: serde_json::json!({"placeholder": true}),
            content_hash: "hash".into(),
            parent_version_id: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            created_by: identity(),
        }
    }

    fn make_approval(artifact_id: &str) -> Approval {
        Approval {
            id: format!("appr-{}", artifact_id),
            project_id: "proj-1".into(),
            artifact_id: artifact_id.into(),
            artifact_version_id: format!("{}-v1", artifact_id),
            artifact_content_hash: "hash".into(),
            approval_type: ApprovalType::Standard,
            approver: identity(),
            rationale: None,
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn make_link(source: &str, target: &str, link_type: TraceLinkType) -> TraceLink {
        TraceLink {
            id: format!("link-{}-{}", source, target),
            project_id: "proj-1".into(),
            source_node_id: source.into(),
            target_node_id: target.into(),
            link_type,
            rationale: "justified".into(),
            created_by: identity(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn full_project() -> (
        Project,
        Constitution,
        Vec<Artifact>,
        Vec<ArtifactVersion>,
        Vec<Approval>,
        Vec<TraceLink>,
    ) {
        let project = make_project();
        let constitution = make_constitution();
        let artifacts = vec![
            make_artifact("const-1", ArtifactType::Constitution),
            make_artifact("wf-1", ArtifactType::UserFantasyWorkflows),
            make_artifact("feat-1", ArtifactType::FeatureMap),
            make_artifact("sys-1", ArtifactType::SystemArchitecture),
            make_artifact("ux-1", ArtifactType::UxStateMap),
            make_artifact("phase-1", ArtifactType::PhaseRoadmapContracts),
            make_artifact("check-1", ArtifactType::AcceptanceChecklists),
            make_artifact("drift-1", ArtifactType::DriftAlarmDefinitions),
        ];
        let versions: Vec<ArtifactVersion> =
            artifacts.iter().map(|a| make_version(&a.id)).collect();
        let approvals: Vec<Approval> = artifacts
            .iter()
            .filter(|a| a.artifact_type != ArtifactType::Constitution)
            .map(|a| make_approval(&a.id))
            .collect();
        let links = vec![
            make_link("wf-1", "const-1", TraceLinkType::DerivesFrom),
            make_link("feat-1", "wf-1", TraceLinkType::Justifies),
            make_link("sys-1", "feat-1", TraceLinkType::Implements),
            make_link("ux-1", "wf-1", TraceLinkType::DependsOn),
            make_link("ux-1", "sys-1", TraceLinkType::DependsOn),
            make_link("phase-1", "const-1", TraceLinkType::ValidatedBy),
            make_link("phase-1", "ux-1", TraceLinkType::DependsOn),
            make_link("check-1", "phase-1", TraceLinkType::DependsOn),
            make_link("drift-1", "const-1", TraceLinkType::InvalidatedBy),
        ];
        (project, constitution, artifacts, versions, approvals, links)
    }

    #[test]
    fn compile_succeeds_for_valid_project() {
        let (project, constitution, artifacts, versions, approvals, links) = full_project();
        let input = ExportInput {
            project: &project,
            constitution: &constitution,
            artifacts: &artifacts,
            versions: &versions,
            approvals: &approvals,
            links: &links,
            alarms: &[],
            amendments: &[],
            audit_events: &[],
        };

        let result = compile(&input);
        assert!(result.is_ok());

        let package = result.unwrap();
        assert_eq!(package.schema_version, SCHEMA_VERSION);

        // project.json + constitution.md + 7 artifact .md + 4 reports = 13
        assert_eq!(package.files.len(), 13);

        // Check specific files exist
        let paths: Vec<&str> = package.files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"project.json"));
        assert!(paths.contains(&"constitution.md"));
        assert!(paths.contains(&"artifacts/feature-map.md"));
        assert!(paths.contains(&"reports/traceability-matrix.md"));
        assert!(paths.contains(&"reports/audit-log.md"));
        assert!(paths.contains(&"reports/drift-report.md"));
        assert!(paths.contains(&"reports/execution-readiness-report.md"));
    }

    #[test]
    fn compile_blocked_when_draft_artifact_exists() {
        let (project, constitution, mut artifacts, versions, approvals, links) = full_project();
        artifacts[1].state = ArtifactState::Draft; // break workflows

        let input = ExportInput {
            project: &project,
            constitution: &constitution,
            artifacts: &artifacts,
            versions: &versions,
            approvals: &approvals,
            links: &links,
            alarms: &[],
            amendments: &[],
            audit_events: &[],
        };

        let result = compile(&input);
        assert!(result.is_err());

        let blocked = result.unwrap_err();
        assert_eq!(blocked.gate_evaluation.status, GateStatus::Blocked);
    }

    #[test]
    fn constitution_md_renders_all_sections() {
        let c = make_constitution();
        let file = render_constitution_md(&c);

        assert_eq!(file.path, "constitution.md");
        assert!(file.content.contains("# Product Constitution"));
        assert!(file.content.contains("Build something great"));
        assert!(file.content.contains("## Anti-Goals"));
        assert!(file.content.contains("No cloud dependency"));
        assert!(file.content.contains("Approved by **Test User**"));
    }

    #[test]
    fn project_json_is_valid_json() {
        let (project, constitution, artifacts, versions, approvals, links) = full_project();
        let input = ExportInput {
            project: &project,
            constitution: &constitution,
            artifacts: &artifacts,
            versions: &versions,
            approvals: &approvals,
            links: &links,
            alarms: &[],
            amendments: &[],
            audit_events: &[],
        };

        let file = render_project_json(&input);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&file.content);
        assert!(parsed.is_ok());

        let json = parsed.unwrap();
        assert_eq!(json["schemaVersion"], SCHEMA_VERSION);
        assert!(json["project"]["name"].is_string());
        assert!(json["constitution"]["oneSentencePromise"].is_string());
    }

    #[test]
    fn traceability_matrix_has_all_links() {
        let (_, _, artifacts, _, _, links) = full_project();
        let file = render_traceability_matrix(&artifacts, &links);

        assert!(file.content.contains("# Traceability Matrix"));
        // 9 data links + 1 header row that also contains " → "
        let data_rows: Vec<_> = file.content.lines()
            .filter(|l| l.starts_with("| ") && l.contains(" → "))
            .skip(1) // skip header
            .collect();
        assert_eq!(data_rows.len(), 9);
    }

    #[test]
    fn drift_report_renders_empty_state() {
        let file = render_drift_report(&[]);
        assert!(file.content.contains("**Active:** 0"));
        assert!(file.content.contains("**Total:** 0"));
    }

    #[test]
    fn readiness_report_shows_ready() {
        let (project, constitution, artifacts, versions, approvals, links) = full_project();
        let gate = readiness_gate::evaluate(
            &artifacts, &versions, &approvals, &links, &constitution, &[], &[],
        );
        let file = render_readiness_report(&gate);

        assert!(file.content.contains("**Status:** Ready"));
        assert!(file.content.contains("Ready for export"));
    }
}

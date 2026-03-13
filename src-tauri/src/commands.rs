//! Tauri command layer.
//!
//! The UI calls these. They call the law engine. The UI never
//! computes readiness, transitions, or export eligibility on its own.

use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::amendments;
use crate::audit_log;
use crate::diff;
use crate::domain::*;
use crate::editing;
use crate::export_compiler::{self, ExportInput};
use crate::impact;
use crate::link_authoring;
use crate::persistence;
use crate::readiness_gate::{self, GateEvaluation};
use crate::recovery;
use crate::state_machine;
use crate::store::ProjectStore;
use crate::traceability;
use crate::validation;

pub type AppState = Mutex<ProjectStore>;

// ─── Response Types ─────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub project: Project,
    pub artifacts: Vec<ArtifactRow>,
    pub gate_status: GateStatus,
    pub active_alarm_count: usize,
    pub stale_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRow {
    pub id: String,
    pub artifact_type: ArtifactType,
    pub title: String,
    pub state: ArtifactState,
    pub version_number: u32,
    pub has_approval: bool,
    pub upstream_count: usize,
    pub downstream_count: usize,
    pub alarm_count: usize,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDetailResponse {
    pub artifact: Artifact,
    pub version: Option<ArtifactVersion>,
    pub approval: Option<Approval>,
    pub outgoing_links: Vec<TraceLinkRow>,
    pub incoming_links: Vec<TraceLinkRow>,
    pub active_alarms: Vec<DriftAlarm>,
    pub legal_transitions: Vec<ArtifactState>,
}

/// Enriched trace link with resolved artifact titles.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceLinkRow {
    pub id: String,
    pub source_id: String,
    pub source_title: String,
    pub target_id: String,
    pub target_title: String,
    pub link_type: TraceLinkType,
    pub rationale: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreviewResponse {
    pub ready: bool,
    pub files: Vec<ExportFilePreview>,
    pub blocked_reason: Option<String>,
    pub blocking_reasons: Vec<BlockingReason>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFilePreview {
    pub path: String,
    pub size_bytes: usize,
    pub content_preview: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionResponse {
    pub success: bool,
    pub new_state: Option<ArtifactState>,
    pub error: Option<String>,
}

// ─── New Response Types (Step 10) ───────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditResponse {
    pub success: bool,
    pub new_version_number: Option<u32>,
    pub new_state: Option<ArtifactState>,
    pub stale_artifact_ids: Vec<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmendmentResponse {
    pub success: bool,
    pub amendment_id: Option<String>,
    pub status: Option<AmendmentStatus>,
    pub affected_artifact_ids: Vec<String>,
    pub impact_summary: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditTimelineResponse {
    pub events: Vec<AuditEventRow>,
    pub total_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventRow {
    pub id: String,
    pub event_type: AuditEventType,
    pub occurred_at: String,
    pub actor_name: String,
    pub summary: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveLoadResponse {
    pub success: bool,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

// ─── Read-Only Queries ──────────────────────────────────────

#[tauri::command]
pub fn get_project_snapshot(state: State<'_, AppState>) -> Result<ProjectSnapshot, String> {
    let store = state.lock().map_err(|e| e.to_string())?;

    let gate = readiness_gate::evaluate(
        &store.artifacts,
        &store.versions,
        &store.approvals,
        &store.links,
        &store.constitution,
        &store.alarms,
        &store.amendments,
    );

    let artifact_rows: Vec<ArtifactRow> = store
        .artifacts
        .iter()
        .map(|a| {
            let upstream = traceability::upstream_links(&a.id, &store.links);
            let downstream = traceability::downstream_links(&a.id, &store.links);
            let alarm_count = store
                .alarms
                .iter()
                .filter(|al| {
                    al.affected_node_ids.contains(&a.id)
                        && al.status == DriftAlarmStatus::Active
                })
                .count();
            let version = store.versions.iter().find(|v| v.id == a.current_version_id);
            let has_approval = store.approvals.iter().any(|ap| ap.artifact_id == a.id);

            ArtifactRow {
                id: a.id.clone(),
                artifact_type: a.artifact_type,
                title: a.title.clone(),
                state: a.state,
                version_number: version.map(|v| v.version_number).unwrap_or(0),
                has_approval,
                upstream_count: upstream.len(),
                downstream_count: downstream.len(),
                alarm_count,
                updated_at: a.updated_at.clone(),
            }
        })
        .collect();

    let stale_count = store
        .artifacts
        .iter()
        .filter(|a| a.state == ArtifactState::Stale)
        .count();
    let active_alarm_count = store
        .alarms
        .iter()
        .filter(|a| a.status == DriftAlarmStatus::Active)
        .count();

    Ok(ProjectSnapshot {
        project: store.project.clone(),
        artifacts: artifact_rows,
        gate_status: gate.status,
        active_alarm_count,
        stale_count,
    })
}

#[tauri::command]
pub fn get_artifact_detail(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<ArtifactDetailResponse, String> {
    let store = state.lock().map_err(|e| e.to_string())?;

    let artifact = store
        .artifacts
        .iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))?
        .clone();

    let version = store
        .versions
        .iter()
        .find(|v| v.id == artifact.current_version_id)
        .cloned();

    let approval = store
        .approvals
        .iter()
        .filter(|a| a.artifact_id == artifact_id)
        .max_by_key(|a| a.created_at.clone())
        .cloned();

    let outgoing_links: Vec<TraceLinkRow> = store
        .links
        .iter()
        .filter(|l| l.source_node_id == artifact_id)
        .map(|l| enrich_link(l, &store.artifacts))
        .collect();

    let incoming_links: Vec<TraceLinkRow> = store
        .links
        .iter()
        .filter(|l| l.target_node_id == artifact_id)
        .map(|l| enrich_link(l, &store.artifacts))
        .collect();

    let active_alarms: Vec<DriftAlarm> = store
        .alarms
        .iter()
        .filter(|a| {
            a.affected_node_ids.contains(&artifact_id)
                && a.status == DriftAlarmStatus::Active
        })
        .cloned()
        .collect();

    let legal_transitions = compute_legal_transitions(&artifact);

    Ok(ArtifactDetailResponse {
        artifact,
        version,
        approval,
        outgoing_links,
        incoming_links,
        active_alarms,
        legal_transitions,
    })
}

#[tauri::command]
pub fn get_readiness_gate(state: State<'_, AppState>) -> Result<GateEvaluation, String> {
    let store = state.lock().map_err(|e| e.to_string())?;

    Ok(readiness_gate::evaluate(
        &store.artifacts,
        &store.versions,
        &store.approvals,
        &store.links,
        &store.constitution,
        &store.alarms,
        &store.amendments,
    ))
}

#[tauri::command]
pub fn get_export_preview(state: State<'_, AppState>) -> Result<ExportPreviewResponse, String> {
    let store = state.lock().map_err(|e| e.to_string())?;

    let input = ExportInput {
        project: &store.project,
        constitution: &store.constitution,
        artifacts: &store.artifacts,
        versions: &store.versions,
        approvals: &store.approvals,
        links: &store.links,
        alarms: &store.alarms,
        amendments: &store.amendments,
        audit_events: &store.audit_events,
    };

    match export_compiler::compile(&input) {
        Ok(package) => {
            let files: Vec<ExportFilePreview> = package
                .files
                .iter()
                .map(|f| ExportFilePreview {
                    path: f.path.clone(),
                    size_bytes: f.content.len(),
                    content_preview: f.content.chars().take(200).collect(),
                })
                .collect();

            Ok(ExportPreviewResponse {
                ready: true,
                files,
                blocked_reason: None,
                blocking_reasons: vec![],
            })
        }
        Err(blocked) => Ok(ExportPreviewResponse {
            ready: false,
            files: vec![],
            blocked_reason: Some(blocked.gate_evaluation.readiness_summary.clone()),
            blocking_reasons: blocked.gate_evaluation.blocking_reasons.clone(),
        }),
    }
}

// ─── Mutation Commands ──────────────────────────────────────

#[tauri::command]
pub fn transition_artifact(
    state: State<'_, AppState>,
    artifact_id: String,
    target_state: ArtifactState,
) -> Result<TransitionResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;

    let artifact = store
        .artifacts
        .iter_mut()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))?;

    if !state_machine::is_legal_transition(artifact.state, target_state) {
        return Ok(TransitionResponse {
            success: false,
            new_state: None,
            error: Some(format!(
                "Transition {:?} → {:?} is not legal",
                artifact.state, target_state
            )),
        });
    }

    artifact.state = target_state;
    artifact.updated_at = "2026-03-13T12:00:00Z".into();

    Ok(TransitionResponse {
        success: true,
        new_state: Some(target_state),
        error: None,
    })
}

#[tauri::command]
pub fn approve_artifact(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<TransitionResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;

    let artifact = store
        .artifacts
        .iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))?;

    if artifact.state != ArtifactState::Valid {
        return Ok(TransitionResponse {
            success: false,
            new_state: None,
            error: Some(format!(
                "Cannot approve: artifact is {:?}, must be Valid",
                artifact.state
            )),
        });
    }

    let approval = Approval {
        id: format!("appr-{}-{}", artifact_id, store.approvals.len()),
        project_id: store.project.id.clone(),
        artifact_id: artifact_id.clone(),
        artifact_version_id: artifact.current_version_id.clone(),
        artifact_content_hash: "hash".into(),
        approval_type: ApprovalType::Standard,
        approver: store.project.created_by.clone(),
        rationale: None,
        created_at: "2026-03-13T12:00:00Z".into(),
    };
    store.approvals.push(approval);

    let artifact = store
        .artifacts
        .iter_mut()
        .find(|a| a.id == artifact_id)
        .unwrap();
    artifact.state = ArtifactState::Approved;
    artifact.updated_at = "2026-03-13T12:00:00Z".into();

    Ok(TransitionResponse {
        success: true,
        new_state: Some(ArtifactState::Approved),
        error: None,
    })
}

// ─── Step 10: Edit / Amend / Save / Load / History ──────────

#[tauri::command]
pub fn edit_artifact_content(
    state: State<'_, AppState>,
    artifact_id: String,
    content: serde_json::Value,
    content_hash: String,
) -> Result<EditResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let editor = store.project.created_by.clone();
    let timestamp = now_iso();

    match editing::edit_artifact(
        &mut store,
        &artifact_id,
        content,
        &content_hash,
        &editor,
        &timestamp,
    ) {
        Ok(result) => Ok(EditResponse {
            success: true,
            new_version_number: Some(result.new_version.version_number),
            new_state: Some(result.new_state),
            stale_artifact_ids: result.stale_artifact_ids,
            error: None,
        }),
        Err(e) => Ok(EditResponse {
            success: false,
            new_version_number: None,
            new_state: None,
            stale_artifact_ids: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn propose_amendment(
    state: State<'_, AppState>,
    reason: String,
    one_sentence_promise: Option<String>,
    user_fantasy: Option<String>,
    quality_bar: Option<String>,
    failure_condition: Option<String>,
) -> Result<AmendmentResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let proposer = store.project.created_by.clone();
    let timestamp = now_iso();
    let seq = store.amendments.len() + 1;

    let patch = ConstitutionPatch {
        one_sentence_promise,
        user_fantasy,
        non_negotiable_outcomes: None,
        anti_goals: None,
        quality_bar,
        failure_condition,
    };

    match amendments::propose_amendment(
        &store.project,
        &store.constitution,
        &store.amendments,
        patch,
        reason.clone(),
        proposer.clone(),
        &timestamp,
        seq,
    ) {
        Ok(amendment) => {
            let id = amendment.id.clone();
            store.amendments.push(amendment);

            // Audit event
            let evt = audit_log::amendment_started(
                &store.project.id,
                &id,
                &reason,
                &proposer,
                &timestamp,
                store.audit_events.len() + 1,
            );
            store.audit_events.push(evt);

            Ok(AmendmentResponse {
                success: true,
                amendment_id: Some(id),
                status: Some(AmendmentStatus::Proposed),
                affected_artifact_ids: vec![],
                impact_summary: None,
                error: None,
            })
        }
        Err(e) => Ok(AmendmentResponse {
            success: false,
            amendment_id: None,
            status: None,
            affected_artifact_ids: vec![],
            impact_summary: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn assess_amendment_impact(
    state: State<'_, AppState>,
    amendment_id: String,
) -> Result<AmendmentResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;

    let artifacts_snapshot: Vec<_> = store.artifacts.clone();

    let amendment = store
        .amendments
        .iter_mut()
        .find(|a| a.id == amendment_id)
        .ok_or_else(|| format!("Amendment not found: {}", amendment_id))?;

    match amendments::assess_impact(amendment, &artifacts_snapshot) {
        Ok(impact) => Ok(AmendmentResponse {
            success: true,
            amendment_id: Some(amendment_id),
            status: Some(AmendmentStatus::ImpactAssessed),
            affected_artifact_ids: impact.affected_artifact_ids,
            impact_summary: Some(impact.summary),
            error: None,
        }),
        Err(e) => Ok(AmendmentResponse {
            success: false,
            amendment_id: Some(amendment_id),
            status: None,
            affected_artifact_ids: vec![],
            impact_summary: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn apply_amendment(
    state: State<'_, AppState>,
    amendment_id: String,
) -> Result<AmendmentResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let timestamp = now_iso();

    // Find amendment index to work around borrow checker
    let amend_idx = store
        .amendments
        .iter()
        .position(|a| a.id == amendment_id)
        .ok_or_else(|| format!("Amendment not found: {}", amendment_id))?;

    let new_version_id = format!(
        "cv{}",
        store
            .constitution
            .version_id
            .trim_start_matches("cv")
            .parse::<u32>()
            .unwrap_or(1)
            + 1
    );

    // Take the amendment out to satisfy the borrow checker (need &mut amendment + &mut constitution + &mut artifacts)
    let mut amendment = store.amendments.remove(amend_idx);
    let mut constitution = store.constitution.clone();

    match amendments::apply_amendment(
        &mut amendment,
        &mut constitution,
        &mut store.artifacts,
        &timestamp,
        &new_version_id,
    ) {
        Ok(stale_ids) => {
            let actor = amendment.proposer.clone();
            let affected = amendment.invalidated_artifact_ids.clone();

            // Audit event for amendment applied
            let evt = audit_log::amendment_applied(
                &store.project.id,
                &amendment.id,
                &new_version_id,
                stale_ids.len(),
                &actor,
                &timestamp,
                store.audit_events.len() + 1,
            );
            store.audit_events.push(evt);

            // Audit events for each stale artifact
            for (i, stale_id) in stale_ids.iter().enumerate() {
                let reason = format!("Constitution amended: {}", amendment.id);
                let evt = audit_log::artifact_marked_stale(
                    &store.project.id,
                    stale_id,
                    &reason,
                    &timestamp,
                    store.audit_events.len() + i + 1,
                );
                store.audit_events.push(evt);
            }

            // Apply the updated constitution back
            store.constitution = constitution;

            // Update project's constitution version
            store.project.current_constitution_version_id = new_version_id;
            store.project.active_amendment_id = Some(amendment.id.clone());
            store.project.updated_at = timestamp;

            store.amendments.push(amendment);

            Ok(AmendmentResponse {
                success: true,
                amendment_id: Some(amendment_id),
                status: Some(AmendmentStatus::Applied),
                affected_artifact_ids: affected,
                impact_summary: Some(format!("{} artifact(s) marked stale", stale_ids.len())),
                error: None,
            })
        }
        Err(e) => {
            // Put constitution and amendment back unchanged
            store.constitution = constitution;
            store.amendments.push(amendment);
            Ok(AmendmentResponse {
                success: false,
                amendment_id: Some(amendment_id),
                status: None,
                affected_artifact_ids: vec![],
                impact_summary: None,
                error: Some(e.to_string()),
            })
        }
    }
}

#[tauri::command]
pub fn abandon_amendment(
    state: State<'_, AppState>,
    amendment_id: String,
) -> Result<AmendmentResponse, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let timestamp = now_iso();

    let amendment = store
        .amendments
        .iter_mut()
        .find(|a| a.id == amendment_id)
        .ok_or_else(|| format!("Amendment not found: {}", amendment_id))?;

    match amendments::abandon_amendment(amendment, &timestamp) {
        Ok(()) => Ok(AmendmentResponse {
            success: true,
            amendment_id: Some(amendment_id),
            status: Some(AmendmentStatus::Abandoned),
            affected_artifact_ids: vec![],
            impact_summary: None,
            error: None,
        }),
        Err(e) => Ok(AmendmentResponse {
            success: false,
            amendment_id: Some(amendment_id),
            status: None,
            affected_artifact_ids: vec![],
            impact_summary: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn get_audit_timeline(
    state: State<'_, AppState>,
) -> Result<AuditTimelineResponse, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    let events: Vec<AuditEventRow> = store
        .audit_events
        .iter()
        .map(|e| AuditEventRow {
            id: e.id.clone(),
            event_type: e.event_type,
            occurred_at: e.occurred_at.clone(),
            actor_name: match &e.actor {
                AuditActor::User(u) => u.display_name.clone(),
                AuditActor::System => "System".into(),
            },
            summary: format_event_summary(e),
        })
        .collect();

    let total = events.len();
    Ok(AuditTimelineResponse {
        events,
        total_count: total,
    })
}

#[tauri::command]
pub fn get_artifact_history(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<AuditTimelineResponse, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    let filtered = audit_log::events_for_artifact(&artifact_id, &store.audit_events);
    let events: Vec<AuditEventRow> = filtered
        .iter()
        .map(|e| AuditEventRow {
            id: e.id.clone(),
            event_type: e.event_type,
            occurred_at: e.occurred_at.clone(),
            actor_name: match &e.actor {
                AuditActor::User(u) => u.display_name.clone(),
                AuditActor::System => "System".into(),
            },
            summary: format_event_summary(e),
        })
        .collect();

    let total = events.len();
    Ok(AuditTimelineResponse {
        events,
        total_count: total,
    })
}

#[tauri::command]
pub fn save_project(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<SaveLoadResponse, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    let path = std::path::Path::new(&file_path);

    match persistence::save_project(&store, path) {
        Ok(saved_path) => Ok(SaveLoadResponse {
            success: true,
            file_path: Some(saved_path.to_string_lossy().into_owned()),
            error: None,
        }),
        Err(e) => Ok(SaveLoadResponse {
            success: false,
            file_path: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn load_project(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<SaveLoadResponse, String> {
    let path = std::path::Path::new(&file_path);

    match persistence::load_project(path) {
        Ok(loaded_store) => {
            let mut store = state.lock().map_err(|e| e.to_string())?;
            *store = loaded_store;
            Ok(SaveLoadResponse {
                success: true,
                file_path: Some(file_path),
                error: None,
            })
        }
        Err(e) => Ok(SaveLoadResponse {
            success: false,
            file_path: None,
            error: Some(e.to_string()),
        }),
    }
}

// ─── Step 11: Explainability & Recovery Commands ────────────

#[tauri::command]
pub fn get_validation_report(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<validation::ValidationReport, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    validation::validate_artifact(
        &artifact_id,
        &store.artifacts,
        &store.versions,
        &store.approvals,
        &store.links,
        &store.constitution,
    )
    .ok_or_else(|| format!("Artifact not found: {}", artifact_id))
}

#[tauri::command]
pub fn get_version_diff(
    state: State<'_, AppState>,
    artifact_id: String,
    from_version_id: String,
    to_version_id: String,
) -> Result<diff::VersionDiff, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    diff::diff_versions(
        &artifact_id,
        &from_version_id,
        &to_version_id,
        &store.versions,
        &store.approvals,
    )
    .ok_or_else(|| "Version(s) not found".into())
}

#[tauri::command]
pub fn get_latest_diff(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<Option<diff::VersionDiff>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    Ok(diff::diff_latest(
        &artifact_id,
        &store.artifacts,
        &store.versions,
        &store.approvals,
    ))
}

#[tauri::command]
pub fn get_edit_impact(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<impact::ImpactReport, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    impact::impact_of_edit(
        &artifact_id,
        &store.artifacts,
        &store.links,
        &store.approvals,
    )
    .ok_or_else(|| format!("Artifact not found: {}", artifact_id))
}

#[tauri::command]
pub fn get_amendment_impact(
    state: State<'_, AppState>,
) -> Result<impact::ImpactReport, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    Ok(impact::impact_of_amendment(
        &store.artifacts,
        &store.links,
        &store.approvals,
        &store.constitution,
    ))
}

#[tauri::command]
pub fn dry_run_import(
    file_path: String,
) -> Result<persistence::ImportDiagnostic, String> {
    let path = std::path::Path::new(&file_path);
    Ok(persistence::dry_run_load(path))
}

#[tauri::command]
pub fn load_project_with_repair(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<ImportWithRepairResponse, String> {
    let path = std::path::Path::new(&file_path);
    match persistence::load_project_with_repair(path) {
        Ok((loaded_store, issues)) => {
            let mut store = state.lock().map_err(|e| e.to_string())?;
            *store = loaded_store;
            Ok(ImportWithRepairResponse {
                success: true,
                file_path: Some(file_path),
                issues,
                error: None,
            })
        }
        Err(e) => Ok(ImportWithRepairResponse {
            success: false,
            file_path: None,
            issues: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWithRepairResponse {
    pub success: bool,
    pub file_path: Option<String>,
    pub issues: Vec<persistence::ImportIssue>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn switch_demo_scenario(
    state: State<'_, AppState>,
    scenario_name: String,
) -> Result<SwitchScenarioResponse, String> {
    match ProjectStore::load_scenario(&scenario_name) {
        Some(new_store) => {
            let project_name = new_store.project.name.clone();
            let artifact_count = new_store.artifacts.len();
            let mut store = state.lock().map_err(|e| e.to_string())?;
            *store = new_store;
            Ok(SwitchScenarioResponse {
                success: true,
                scenario_name: scenario_name.clone(),
                project_name,
                artifact_count,
                error: None,
            })
        }
        None => Ok(SwitchScenarioResponse {
            success: false,
            scenario_name,
            project_name: String::new(),
            artifact_count: 0,
            error: Some(format!(
                "Unknown scenario. Available: {}",
                crate::store::DEMO_SCENARIOS.join(", ")
            )),
        }),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchScenarioResponse {
    pub success: bool,
    pub scenario_name: String,
    pub project_name: String,
    pub artifact_count: usize,
    pub error: Option<String>,
}

#[tauri::command]
pub fn list_demo_scenarios() -> Vec<ScenarioInfo> {
    vec![
        ScenarioInfo {
            id: "forge-quest".into(),
            name: "Forge Quest".into(),
            description: "Mixed states — gate blocked, some artifacts approved, some in progress".into(),
            flavor: "The original demo. A crafting RPG with constitutional governance.".into(),
        },
        ScenarioInfo {
            id: "crystal-sanctum".into(),
            name: "Crystal Sanctum".into(),
            description: "Healthy project — all artifacts approved, gate ready for export".into(),
            flavor: "What a finished project looks like. Everything green.".into(),
        },
        ScenarioInfo {
            id: "shadow-protocol".into(),
            name: "Shadow Protocol".into(),
            description: "Broken traceability — missing links, orphan artifacts, drift alarms".into(),
            flavor: "What happens when you skip the governance. The law catches you.".into(),
        },
        ScenarioInfo {
            id: "ember-saga".into(),
            name: "Ember Saga".into(),
            description: "Post-amendment fallout — constitution changed, mass stale propagation".into(),
            flavor: "The nuclear option happened. Now reconicle everything.".into(),
        },
    ]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub flavor: String,
}

// ─── Helpers ────────────────────────────────────────────────

fn enrich_link(link: &TraceLink, artifacts: &[Artifact]) -> TraceLinkRow {
    let source_title = artifacts
        .iter()
        .find(|a| a.id == link.source_node_id)
        .map(|a| a.title.clone())
        .unwrap_or_else(|| link.source_node_id.clone());
    let target_title = artifacts
        .iter()
        .find(|a| a.id == link.target_node_id)
        .map(|a| a.title.clone())
        .unwrap_or_else(|| link.target_node_id.clone());

    TraceLinkRow {
        id: link.id.clone(),
        source_id: link.source_node_id.clone(),
        source_title,
        target_id: link.target_node_id.clone(),
        target_title,
        link_type: link.link_type,
        rationale: link.rationale.clone(),
    }
}

fn compute_legal_transitions(artifact: &Artifact) -> Vec<ArtifactState> {
    let all_states = [
        ArtifactState::Draft,
        ArtifactState::Complete,
        ArtifactState::Valid,
        ArtifactState::Approved,
        ArtifactState::Stale,
    ];
    all_states
        .iter()
        .filter(|&&target| state_machine::is_legal_transition(artifact.state, target))
        .copied()
        .collect()
}

fn format_event_summary(event: &AuditEvent) -> String {
    let artifact_id = event
        .payload
        .get("artifactId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match event.event_type {
        AuditEventType::ProjectCreated => "Project created".into(),
        AuditEventType::ConstitutionLocked => "Constitution locked".into(),
        AuditEventType::ArtifactCreated => format!("Artifact created: {}", artifact_id),
        AuditEventType::ArtifactUpdated => {
            let version = event
                .payload
                .get("versionNumber")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("Artifact {} updated to v{}", artifact_id, version)
        }
        AuditEventType::ArtifactCompleted => format!("Artifact {} completed", artifact_id),
        AuditEventType::ArtifactValidated => format!("Artifact {} validated", artifact_id),
        AuditEventType::ArtifactApproved => format!("Artifact {} approved", artifact_id),
        AuditEventType::ArtifactMarkedStale => {
            let reason = event
                .payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("upstream change");
            format!("Artifact {} marked stale: {}", artifact_id, reason)
        }
        AuditEventType::TraceLinkCreated => "Trace link created".into(),
        AuditEventType::TraceLinkRemoved => "Trace link removed".into(),
        AuditEventType::AmendmentStarted => {
            let reason = event
                .payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Amendment started: {}", reason)
        }
        AuditEventType::AmendmentImpactAssessed => "Amendment impact assessed".into(),
        AuditEventType::AmendmentApplied => {
            let count = event
                .payload
                .get("invalidatedArtifactCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("Amendment applied — {} artifacts invalidated", count)
        }
        AuditEventType::DriftAlarmRaised => {
            let explanation = event
                .payload
                .get("explanation")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Drift alarm: {}", explanation)
        }
        AuditEventType::DriftAlarmResolved => "Drift alarm resolved".into(),
        AuditEventType::ExportBlocked => "Export blocked".into(),
        AuditEventType::ReadinessGateComputed => "Readiness gate evaluated".into(),
        AuditEventType::ReadinessGatePassed => "Readiness gate passed".into(),
        AuditEventType::ProjectExported => "Project exported".into(),
    }
}

fn now_iso() -> String {
    // In a real app this would use chrono or std::time.
    // For now, a placeholder that the UI can parse.
    "2026-03-13T12:00:00Z".into()
}

// ─── Step 12: Operator Fluency Commands ─────────────────────

#[tauri::command]
pub fn get_project_health(
    state: State<'_, AppState>,
) -> Result<recovery::ProjectHealth, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    Ok(recovery::project_health(
        &store.artifacts,
        &store.versions,
        &store.approvals,
        &store.links,
        &store.constitution,
        &store.alarms,
        &store.amendments,
    ))
}

#[tauri::command]
pub fn get_recovery_actions(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<Vec<recovery::RecoveryAction>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    let artifact = store
        .artifacts
        .iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))?;
    Ok(recovery::next_actions_for_artifact(
        artifact,
        &store.artifacts,
        &store.versions,
        &store.approvals,
        &store.links,
        &store.alarms,
    ))
}

#[tauri::command]
pub fn get_allowed_links(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<link_authoring::AllowedLinks, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    link_authoring::get_allowed_links(&artifact_id, &store.artifacts, &store.links)
        .ok_or_else(|| format!("Artifact not found: {}", artifact_id))
}

#[tauri::command]
pub fn get_missing_links(
    state: State<'_, AppState>,
) -> Result<Vec<link_authoring::LinkSuggestion>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    Ok(link_authoring::get_missing_links(&store.artifacts, &store.links))
}

#[tauri::command]
pub fn add_trace_link(
    state: State<'_, AppState>,
    source_id: String,
    target_id: String,
    link_type: TraceLinkType,
    rationale: String,
) -> Result<link_authoring::AddLinkResult, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let actor = LocalIdentity {
        id: "user-1".into(),
        display_name: "Operator".into(),
    };
    let result = link_authoring::add_link(
        &source_id, &target_id, link_type, &rationale,
        &store.artifacts, &store.links, &actor,
    );
    if result.success {
        if let Some(ref link) = result.link {
            store.links.push(link.clone());
            // Emit audit event
            let seq = store.audit_events.len() + 1;
            let pid = store.project.id.clone();
            let evt = audit_log::emit(
                &pid,
                AuditEventType::TraceLinkCreated,
                AuditActor::User(actor),
                serde_json::json!({
                    "linkId": link.id,
                    "sourceId": source_id,
                    "targetId": target_id,
                    "linkType": link_type,
                }),
                &now_iso(),
                seq,
            );
            store.audit_events.push(evt);
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn remove_trace_link(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<link_authoring::RemoveLinkResult, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let result = link_authoring::check_removal_impact(&link_id, &store.artifacts, &store.links);
    if result.success {
        let removed = store.links.iter().find(|l| l.id == link_id).cloned();
        store.links.retain(|l| l.id != link_id);
        if let Some(link) = removed {
            let seq = store.audit_events.len() + 1;
            let pid = store.project.id.clone();
            let evt = audit_log::emit(
                &pid,
                AuditEventType::TraceLinkRemoved,
                AuditActor::System,
                serde_json::json!({
                    "linkId": link.id,
                    "sourceId": link.source_node_id,
                    "targetId": link.target_node_id,
                }),
                &now_iso(),
                seq,
            );
            store.audit_events.push(evt);
        }
    }
    Ok(result)
}

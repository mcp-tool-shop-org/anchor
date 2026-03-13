//! Tauri command layer.
//!
//! The UI calls these. They call the law engine. The UI never
//! computes readiness, transitions, or export eligibility on its own.

use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::domain::*;
use crate::export_compiler::{self, ExportInput};
use crate::readiness_gate::{self, GateEvaluation};
use crate::state_machine;
use crate::store::ProjectStore;
use crate::traceability;

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

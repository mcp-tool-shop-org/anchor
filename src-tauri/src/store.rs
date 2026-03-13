//! In-memory project store.
//!
//! Holds the full project state. Seeded with a demo project
//! so the UI has meaningful data on first launch.

use crate::domain::*;

pub struct ProjectStore {
    pub project: Project,
    pub constitution: Constitution,
    pub artifacts: Vec<Artifact>,
    pub versions: Vec<ArtifactVersion>,
    pub approvals: Vec<Approval>,
    pub links: Vec<TraceLink>,
    pub alarms: Vec<DriftAlarm>,
    pub amendments: Vec<Amendment>,
    pub audit_events: Vec<AuditEvent>,
}

impl ProjectStore {
    /// Build a demo project with artifacts in mixed states.
    /// The gate will be blocked — which is the interesting state for the UI.
    pub fn demo() -> Self {
        let designer = LocalIdentity {
            id: "user-1".into(),
            display_name: "Designer".into(),
        };

        let project = Project {
            id: "proj-1".into(),
            schema_version: SCHEMA_VERSION.into(),
            name: "Forge Quest".into(),
            slug: "forge-quest".into(),
            description: "A narrative RPG crafting system with constitutional design governance"
                .into(),
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            created_by: designer.clone(),
            current_constitution_version_id: "cv1".into(),
            artifact_ids: vec![
                "art-const".into(),
                "art-wf".into(),
                "art-feat".into(),
                "art-sys".into(),
                "art-ux".into(),
                "art-phase".into(),
                "art-check".into(),
                "art-drift".into(),
            ],
            active_amendment_id: None,
            settings: ProjectSettings::default(),
        };

        let constitution = Constitution {
            id: "const-1".into(),
            artifact_id: "art-const".into(),
            version_id: "cv1".into(),
            project_id: "proj-1".into(),
            one_sentence_promise: "Forge Quest delivers a crafting RPG where every recipe, \
                material, and progression path is narratively justified and mechanically coherent."
                .into(),
            user_fantasy: "You open the forge, select rare materials you've gathered through \
                story quests, and craft weapons that reflect both your choices and the world's \
                lore — not just stat sticks from a spreadsheet."
                .into(),
            non_negotiable_outcomes: vec![
                "Every craftable item traces to a narrative source".into(),
                "Material acquisition is gated by story progression, not grinding".into(),
                "The crafting system respects the world's internal logic".into(),
            ],
            anti_goals: vec![
                "No generic loot tables".into(),
                "No pay-to-skip crafting".into(),
                "No disconnected stat optimization".into(),
            ],
            quality_bar: "A player should never craft something that feels arbitrary or \
                world-breaking"
                .into(),
            failure_condition: "If a player can craft endgame gear without engaging with the \
                narrative, the system has failed"
                .into(),
            locked: true,
            content_hash: "hash-const-v1".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            approved_at: Some("2026-03-13T00:00:00Z".into()),
            approved_by: Some(designer.clone()),
            parent_version_id: None,
        };

        // Mixed states: first 5 approved, phase=Valid, check=Complete, drift=Draft
        let artifacts = vec![
            make_artifact("art-const", ArtifactType::Constitution, "Product Constitution", ArtifactState::Approved),
            make_artifact("art-wf", ArtifactType::UserFantasyWorkflows, "User Fantasy + Core Workflows", ArtifactState::Approved),
            make_artifact("art-feat", ArtifactType::FeatureMap, "Feature Map", ArtifactState::Approved),
            make_artifact("art-sys", ArtifactType::SystemArchitecture, "System Architecture", ArtifactState::Approved),
            make_artifact("art-ux", ArtifactType::UxStateMap, "UX State Map", ArtifactState::Approved),
            make_artifact("art-phase", ArtifactType::PhaseRoadmapContracts, "Phase Roadmap + Contracts", ArtifactState::Valid),
            make_artifact("art-check", ArtifactType::AcceptanceChecklists, "Acceptance Checklists", ArtifactState::Complete),
            make_artifact("art-drift", ArtifactType::DriftAlarmDefinitions, "Drift Alarm Definitions", ArtifactState::Draft),
        ];

        let versions: Vec<ArtifactVersion> = artifacts
            .iter()
            .map(|a| ArtifactVersion {
                id: format!("{}-v1", a.id),
                artifact_id: a.id.clone(),
                project_id: "proj-1".into(),
                version_number: 1,
                constitution_version_id: "cv1".into(),
                content: serde_json::json!({"placeholder": true}),
                content_hash: format!("hash-{}", a.id),
                parent_version_id: None,
                created_at: "2026-03-13T00:00:00Z".into(),
                created_by: designer.clone(),
            })
            .collect();

        // Approvals only for Approved artifacts (excluding Constitution)
        let approvals: Vec<Approval> = artifacts
            .iter()
            .filter(|a| {
                a.state == ArtifactState::Approved
                    && a.artifact_type != ArtifactType::Constitution
            })
            .map(|a| Approval {
                id: format!("appr-{}", a.id),
                project_id: "proj-1".into(),
                artifact_id: a.id.clone(),
                artifact_version_id: format!("{}-v1", a.id),
                artifact_content_hash: format!("hash-{}", a.id),
                approval_type: ApprovalType::Standard,
                approver: designer.clone(),
                rationale: None,
                created_at: "2026-03-13T00:00:00Z".into(),
            })
            .collect();

        // Trace links with correct types per §8.1
        let links = vec![
            make_link("art-wf", "art-const", TraceLinkType::DerivesFrom, &designer),
            make_link("art-feat", "art-wf", TraceLinkType::Justifies, &designer),
            make_link("art-sys", "art-feat", TraceLinkType::Implements, &designer),
            make_link("art-ux", "art-wf", TraceLinkType::DependsOn, &designer),
            make_link("art-ux", "art-sys", TraceLinkType::DependsOn, &designer),
            make_link("art-phase", "art-const", TraceLinkType::ValidatedBy, &designer),
            make_link("art-phase", "art-ux", TraceLinkType::DependsOn, &designer),
            make_link("art-check", "art-phase", TraceLinkType::DependsOn, &designer),
            make_link("art-drift", "art-const", TraceLinkType::InvalidatedBy, &designer),
        ];

        ProjectStore {
            project,
            constitution,
            artifacts,
            versions,
            approvals,
            links,
            alarms: vec![],
            amendments: vec![],
            audit_events: vec![],
        }
    }
}

fn make_artifact(id: &str, art_type: ArtifactType, title: &str, state: ArtifactState) -> Artifact {
    Artifact {
        id: id.into(),
        project_id: "proj-1".into(),
        artifact_type: art_type,
        title: title.into(),
        current_version_id: format!("{}-v1", id),
        state,
        validation_summary: ValidationSummary::default(),
        latest_approval_id: None,
        stale_reason: None,
        created_at: "2026-03-13T00:00:00Z".into(),
        updated_at: "2026-03-13T00:00:00Z".into(),
    }
}

fn make_link(
    source: &str,
    target: &str,
    link_type: TraceLinkType,
    identity: &LocalIdentity,
) -> TraceLink {
    TraceLink {
        id: format!("link-{}-{}", source, target),
        project_id: "proj-1".into(),
        source_node_id: source.into(),
        target_node_id: target.into(),
        link_type,
        rationale: "Justified per constitution".into(),
        created_by: identity.clone(),
        created_at: "2026-03-13T00:00:00Z".into(),
    }
}

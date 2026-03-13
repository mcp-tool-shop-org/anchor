//! In-memory project store.
//!
//! Holds the full project state. Seeded with a demo project
//! so the UI has meaningful data on first launch.
//!
//! Four demo scenarios for Step 11:
//! 1. "Forge Quest" — mixed states (original, gate blocked)
//! 2. "Crystal Sanctum" — healthy, all approved, gate ready
//! 3. "Shadow Protocol" — blocked by missing traceability
//! 4. "Ember Saga" — post-amendment fallout (mass stale)

use crate::domain::*;

/// Available demo scenario names.
pub const DEMO_SCENARIOS: &[&str] = &[
    "forge-quest",
    "crystal-sanctum",
    "shadow-protocol",
    "ember-saga",
];

#[derive(Debug)]
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
        Self::demo_forge_quest()
    }

    /// Switch to a named demo scenario.
    pub fn load_scenario(name: &str) -> Option<Self> {
        match name {
            "forge-quest" => Some(Self::demo_forge_quest()),
            "crystal-sanctum" => Some(Self::demo_crystal_sanctum()),
            "shadow-protocol" => Some(Self::demo_shadow_protocol()),
            "ember-saga" => Some(Self::demo_ember_saga()),
            _ => None,
        }
    }

    // ─── Scenario 1: Forge Quest (original) ─────────────────

    fn demo_forge_quest() -> Self {
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
                "art-gate".into(),
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
            make_artifact("art-gate", ArtifactType::ExecutionReadinessGate, "Execution Readiness Gate", ArtifactState::Draft),
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

    // ─── Scenario 2: Crystal Sanctum (healthy, gate ready) ──

    fn demo_crystal_sanctum() -> Self {
        let designer = LocalIdentity {
            id: "user-2".into(),
            display_name: "Architect".into(),
        };
        let proj_id = "proj-2";

        let project = Project {
            id: proj_id.into(),
            schema_version: SCHEMA_VERSION.into(),
            name: "Crystal Sanctum".into(),
            slug: "crystal-sanctum".into(),
            description: "A puzzle RPG where every crystal resonance path is designed and approved".into(),
            created_at: "2026-03-10T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            created_by: designer.clone(),
            current_constitution_version_id: "cs-cv1".into(),
            artifact_ids: (0..9).map(|_| String::new()).collect(), // filled below
            active_amendment_id: None,
            settings: ProjectSettings::default(),
        };

        let constitution = Constitution {
            id: "cs-const-1".into(),
            artifact_id: "cs-const".into(),
            version_id: "cs-cv1".into(),
            project_id: proj_id.into(),
            one_sentence_promise: "Crystal Sanctum delivers a puzzle RPG where crystal resonance mechanics are internally consistent and narratively grounded.".into(),
            user_fantasy: "You discover crystal harmonics by experimenting with combinations — each puzzle solution feels earned through understanding, not guessing.".into(),
            non_negotiable_outcomes: vec![
                "Every puzzle traces to a crystal mechanic defined in the lore".into(),
                "No brute-force solutions — understanding is required".into(),
            ],
            anti_goals: vec![
                "No random trial-and-error puzzles".into(),
                "No difficulty walls without learning paths".into(),
            ],
            quality_bar: "Players should feel clever, not lucky".into(),
            failure_condition: "If a puzzle can be solved without understanding the underlying crystal system".into(),
            locked: true,
            content_hash: "hash-cs-const-v1".into(),
            created_at: "2026-03-10T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            approved_at: Some("2026-03-12T00:00:00Z".into()),
            approved_by: Some(designer.clone()),
            parent_version_id: None,
        };

        // All artifacts Approved — gate should be ready
        let artifact_defs = [
            ("cs-const", ArtifactType::Constitution, "Product Constitution"),
            ("cs-wf", ArtifactType::UserFantasyWorkflows, "Crystal Discovery Workflows"),
            ("cs-feat", ArtifactType::FeatureMap, "Resonance Feature Map"),
            ("cs-sys", ArtifactType::SystemArchitecture, "Harmonic Engine Architecture"),
            ("cs-ux", ArtifactType::UxStateMap, "Puzzle UX Flow"),
            ("cs-phase", ArtifactType::PhaseRoadmapContracts, "Phase Contracts"),
            ("cs-check", ArtifactType::AcceptanceChecklists, "Crystal Acceptance Tests"),
            ("cs-drift", ArtifactType::DriftAlarmDefinitions, "Resonance Drift Guards"),
            ("cs-gate", ArtifactType::ExecutionReadinessGate, "Execution Readiness Gate"),
        ];

        let artifacts: Vec<Artifact> = artifact_defs.iter().map(|(id, art_type, title)| {
            make_artifact_for(id, *art_type, title, ArtifactState::Approved, proj_id)
        }).collect();

        let versions: Vec<ArtifactVersion> = artifacts.iter().map(|a| ArtifactVersion {
            id: format!("{}-v1", a.id),
            artifact_id: a.id.clone(),
            project_id: proj_id.into(),
            version_number: 1,
            constitution_version_id: "cs-cv1".into(),
            content: serde_json::json!({"authored": true, "scope": "crystal resonance"}),
            content_hash: format!("hash-{}", a.id),
            parent_version_id: None,
            created_at: "2026-03-12T00:00:00Z".into(),
            created_by: designer.clone(),
        }).collect();

        let approvals: Vec<Approval> = artifacts.iter()
            .filter(|a| a.artifact_type != ArtifactType::Constitution)
            .map(|a| Approval {
                id: format!("appr-{}", a.id),
                project_id: proj_id.into(),
                artifact_id: a.id.clone(),
                artifact_version_id: format!("{}-v1", a.id),
                artifact_content_hash: format!("hash-{}", a.id),
                approval_type: ApprovalType::Standard,
                approver: designer.clone(),
                rationale: None,
                created_at: "2026-03-13T00:00:00Z".into(),
            })
            .collect();

        let links = vec![
            make_link_for("cs-wf", "cs-const", TraceLinkType::DerivesFrom, &designer, proj_id),
            make_link_for("cs-feat", "cs-wf", TraceLinkType::Justifies, &designer, proj_id),
            make_link_for("cs-sys", "cs-feat", TraceLinkType::Implements, &designer, proj_id),
            make_link_for("cs-ux", "cs-wf", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("cs-ux", "cs-sys", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("cs-phase", "cs-const", TraceLinkType::ValidatedBy, &designer, proj_id),
            make_link_for("cs-phase", "cs-ux", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("cs-check", "cs-phase", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("cs-drift", "cs-const", TraceLinkType::InvalidatedBy, &designer, proj_id),
        ];

        let mut project = project;
        project.artifact_ids = artifacts.iter().map(|a| a.id.clone()).collect();

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

    // ─── Scenario 3: Shadow Protocol (broken traceability) ──

    fn demo_shadow_protocol() -> Self {
        let designer = LocalIdentity {
            id: "user-3".into(),
            display_name: "Operator".into(),
        };
        let proj_id = "proj-3";

        let project = Project {
            id: proj_id.into(),
            schema_version: SCHEMA_VERSION.into(),
            name: "Shadow Protocol".into(),
            slug: "shadow-protocol".into(),
            description: "A stealth action game with governance failures — missing trace links, orphan artifacts".into(),
            created_at: "2026-03-08T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            created_by: designer.clone(),
            current_constitution_version_id: "sp-cv1".into(),
            artifact_ids: vec![],
            active_amendment_id: None,
            settings: ProjectSettings::default(),
        };

        let constitution = Constitution {
            id: "sp-const-1".into(),
            artifact_id: "sp-const".into(),
            version_id: "sp-cv1".into(),
            project_id: proj_id.into(),
            one_sentence_promise: "Shadow Protocol delivers stealth gameplay where every mission feels like a puzzle with multiple valid approaches.".into(),
            user_fantasy: "You case the compound, choose your approach, and execute — if you're seen, it's because you made a choice, not because the game cheated.".into(),
            non_negotiable_outcomes: vec![
                "Every mission has at least 3 viable approaches".into(),
                "Detection is always the player's fault, never RNG".into(),
            ],
            anti_goals: vec![
                "No mandatory combat".into(),
                "No single optimal path".into(),
            ],
            quality_bar: "A skilled player should feel invisible, not lucky".into(),
            failure_condition: "If any mission has only one valid approach".into(),
            locked: true,
            content_hash: "hash-sp-const-v1".into(),
            created_at: "2026-03-08T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            approved_at: Some("2026-03-09T00:00:00Z".into()),
            approved_by: Some(designer.clone()),
            parent_version_id: None,
        };

        // Mix of states. Some artifacts have no trace links (the disease).
        let artifacts = vec![
            make_artifact_for("sp-const", ArtifactType::Constitution, "Product Constitution", ArtifactState::Approved, proj_id),
            make_artifact_for("sp-wf", ArtifactType::UserFantasyWorkflows, "Stealth Workflows", ArtifactState::Approved, proj_id),
            make_artifact_for("sp-feat", ArtifactType::FeatureMap, "Infiltration Feature Map", ArtifactState::Valid, proj_id),
            // System Architecture has NO upstream links — broken traceability!
            make_artifact_for("sp-sys", ArtifactType::SystemArchitecture, "Detection Engine", ArtifactState::Complete, proj_id),
            // UX State Map references a nonexistent artifact
            make_artifact_for("sp-ux", ArtifactType::UxStateMap, "Mission Planning UX", ArtifactState::Complete, proj_id),
            make_artifact_for("sp-phase", ArtifactType::PhaseRoadmapContracts, "Phase Contracts", ArtifactState::Draft, proj_id),
            make_artifact_for("sp-check", ArtifactType::AcceptanceChecklists, "Stealth Test Suite", ArtifactState::Draft, proj_id),
            make_artifact_for("sp-drift", ArtifactType::DriftAlarmDefinitions, "Detection Drift Alarms", ArtifactState::Draft, proj_id),
            make_artifact_for("sp-gate", ArtifactType::ExecutionReadinessGate, "Execution Readiness Gate", ArtifactState::Draft, proj_id),
        ];

        let versions: Vec<ArtifactVersion> = artifacts.iter().map(|a| ArtifactVersion {
            id: format!("{}-v1", a.id),
            artifact_id: a.id.clone(),
            project_id: proj_id.into(),
            version_number: 1,
            constitution_version_id: "sp-cv1".into(),
            content: serde_json::json!({"placeholder": true}),
            content_hash: format!("hash-{}", a.id),
            parent_version_id: None,
            created_at: "2026-03-10T00:00:00Z".into(),
            created_by: designer.clone(),
        }).collect();

        let approvals: Vec<Approval> = vec![
            Approval {
                id: "appr-sp-wf".into(),
                project_id: proj_id.into(),
                artifact_id: "sp-wf".into(),
                artifact_version_id: "sp-wf-v1".into(),
                artifact_content_hash: "hash-sp-wf".into(),
                approval_type: ApprovalType::Standard,
                approver: designer.clone(),
                rationale: None,
                created_at: "2026-03-11T00:00:00Z".into(),
            },
        ];

        // Deliberately sparse — missing required links
        let links = vec![
            make_link_for("sp-wf", "sp-const", TraceLinkType::DerivesFrom, &designer, proj_id),
            make_link_for("sp-feat", "sp-wf", TraceLinkType::Justifies, &designer, proj_id),
            // NO link for sp-sys (should implement sp-feat)
            // sp-ux depends on nonexistent artifact
            make_link_for("sp-ux", "sp-wf", TraceLinkType::DependsOn, &designer, proj_id),
            // sp-phase has no link to constitution
            // sp-drift has no link
        ];

        // Drift alarms for the traceability failures
        let alarms = vec![
            DriftAlarm {
                id: "alarm-sp-1".into(),
                project_id: proj_id.into(),
                alarm_type: DriftAlarmType::TraceabilityDrift,
                severity: DriftAlarmSeverity::Blocking,
                source_artifact_id: "sp-sys".into(),
                affected_node_ids: vec!["sp-sys".into()],
                violated_rule_id: "trace-003".into(),
                rule_provenance: RuleProvenance {
                    source_artifact_type: SourceArtifactType::SystemRule,
                    source_clause: "§8.1 — Required upstream trace links".into(),
                    human_label: "Systems must trace to features via implements".into(),
                },
                explanation: "System Architecture has no upstream trace link — it floats disconnected from the constitution.".into(),
                remediation_path: vec![
                    "Add an 'implements' link from Detection Engine to Infiltration Feature Map".into(),
                ],
                status: DriftAlarmStatus::Active,
                created_at: "2026-03-13T00:00:00Z".into(),
                resolved_at: None,
            },
            DriftAlarm {
                id: "alarm-sp-2".into(),
                project_id: proj_id.into(),
                alarm_type: DriftAlarmType::TraceabilityDrift,
                severity: DriftAlarmSeverity::Error,
                source_artifact_id: "sp-phase".into(),
                affected_node_ids: vec!["sp-phase".into()],
                violated_rule_id: "trace-005".into(),
                rule_provenance: RuleProvenance {
                    source_artifact_type: SourceArtifactType::SystemRule,
                    source_clause: "§8.1 — Required upstream trace links".into(),
                    human_label: "Phases must trace to constitutional obligations via validated_by".into(),
                },
                explanation: "Phase Contracts has no link to the Constitution — phases must be validated against constitutional obligations.".into(),
                remediation_path: vec![
                    "Add a 'validated_by' link from Phase Contracts to the Constitution".into(),
                ],
                status: DriftAlarmStatus::Active,
                created_at: "2026-03-13T00:00:00Z".into(),
                resolved_at: None,
            },
        ];

        let mut project = project;
        project.artifact_ids = artifacts.iter().map(|a| a.id.clone()).collect();

        ProjectStore {
            project,
            constitution,
            artifacts,
            versions,
            approvals,
            links,
            alarms,
            amendments: vec![],
            audit_events: vec![],
        }
    }

    // ─── Scenario 4: Ember Saga (post-amendment fallout) ────

    fn demo_ember_saga() -> Self {
        let designer = LocalIdentity {
            id: "user-4".into(),
            display_name: "Director".into(),
        };
        let proj_id = "proj-4";

        let project = Project {
            id: proj_id.into(),
            schema_version: SCHEMA_VERSION.into(),
            name: "Ember Saga".into(),
            slug: "ember-saga".into(),
            description: "A narrative RPG that underwent a constitution amendment — mass stale fallout".into(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            created_by: designer.clone(),
            current_constitution_version_id: "es-cv2".into(),
            artifact_ids: vec![],
            active_amendment_id: None,
            settings: ProjectSettings::default(),
        };

        let constitution = Constitution {
            id: "es-const-1".into(),
            artifact_id: "es-const".into(),
            version_id: "es-cv2".into(),
            project_id: proj_id.into(),
            one_sentence_promise: "Ember Saga delivers a narrative RPG where fire magic is a living ecosystem, not a damage type.".into(),
            user_fantasy: "You tend a flame that grows with you — it remembers what you burned and what you protected.".into(),
            non_negotiable_outcomes: vec![
                "Fire magic has consequences and memory".into(),
                "The world reacts to how you use flame".into(),
                "AMENDED: Companion spirits are now bound to flame affinity".into(),
            ],
            anti_goals: vec![
                "No fire-and-forget combat".into(),
                "No consequence-free destruction".into(),
            ],
            quality_bar: "Every flame action should make the player think twice".into(),
            failure_condition: "If fire is just another mana cost".into(),
            locked: true,
            content_hash: "hash-es-const-v2".into(),
            created_at: "2026-03-01T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            approved_at: Some("2026-03-02T00:00:00Z".into()),
            approved_by: Some(designer.clone()),
            parent_version_id: Some("es-cv1".into()),
        };

        // Everything was Approved, but the amendment made most things Stale
        let artifacts = vec![
            {
                let mut a = make_artifact_for("es-const", ArtifactType::Constitution, "Product Constitution (Amended)", ArtifactState::Approved, proj_id);
                a.current_version_id = "es-const-v2".into();
                a
            },
            {
                let mut a = make_artifact_for("es-wf", ArtifactType::UserFantasyWorkflows, "Flame Keeper Workflows", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Constitution amended: companion spirits now bound to flame affinity".into());
                a
            },
            {
                let mut a = make_artifact_for("es-feat", ArtifactType::FeatureMap, "Fire Ecosystem Features", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Upstream UserFantasyWorkflows became stale due to constitution amendment".into());
                a
            },
            {
                let mut a = make_artifact_for("es-sys", ArtifactType::SystemArchitecture, "Flame Memory Architecture", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Upstream FeatureMap became stale — transitive propagation from constitution amendment".into());
                a
            },
            {
                let mut a = make_artifact_for("es-ux", ArtifactType::UxStateMap, "Flame Tending UX", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Upstream UserFantasyWorkflows became stale due to constitution amendment".into());
                a
            },
            {
                let mut a = make_artifact_for("es-phase", ArtifactType::PhaseRoadmapContracts, "Phase Contracts", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Constitution amended — phases must be reconciled".into());
                a
            },
            {
                let mut a = make_artifact_for("es-check", ArtifactType::AcceptanceChecklists, "Flame Tests", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Upstream PhaseRoadmapContracts became stale".into());
                a
            },
            {
                let mut a = make_artifact_for("es-drift", ArtifactType::DriftAlarmDefinitions, "Flame Drift Guards", ArtifactState::Stale, proj_id);
                a.stale_reason = Some("Constitution amended — drift definitions must align with new clauses".into());
                a
            },
            make_artifact_for("es-gate", ArtifactType::ExecutionReadinessGate, "Execution Readiness Gate", ArtifactState::Draft, proj_id),
        ];

        // Two versions for constitution (pre and post amendment)
        let mut versions: Vec<ArtifactVersion> = vec![
            ArtifactVersion {
                id: "es-const-v1".into(),
                artifact_id: "es-const".into(),
                project_id: proj_id.into(),
                version_number: 1,
                constitution_version_id: "es-cv1".into(),
                content: serde_json::json!({"promise": "fire magic with consequences", "version": "original"}),
                content_hash: "hash-es-const-v1".into(),
                parent_version_id: None,
                created_at: "2026-03-01T00:00:00Z".into(),
                created_by: designer.clone(),
            },
            ArtifactVersion {
                id: "es-const-v2".into(),
                artifact_id: "es-const".into(),
                project_id: proj_id.into(),
                version_number: 2,
                constitution_version_id: "es-cv2".into(),
                content: serde_json::json!({"promise": "fire magic with consequences", "amendment": "companion spirits bound to flame affinity", "version": "amended"}),
                content_hash: "hash-es-const-v2".into(),
                parent_version_id: Some("es-const-v1".into()),
                created_at: "2026-03-13T00:00:00Z".into(),
                created_by: designer.clone(),
            },
        ];

        // Other artifacts have v1 authored against the OLD constitution
        for a in artifacts.iter().filter(|a| a.artifact_type != ArtifactType::Constitution) {
            versions.push(ArtifactVersion {
                id: format!("{}-v1", a.id),
                artifact_id: a.id.clone(),
                project_id: proj_id.into(),
                version_number: 1,
                constitution_version_id: "es-cv1".into(), // old constitution!
                content: serde_json::json!({"authored": true, "constitution": "v1"}),
                content_hash: format!("hash-{}", a.id),
                parent_version_id: None,
                created_at: "2026-03-05T00:00:00Z".into(),
                created_by: designer.clone(),
            });
        }

        // Approvals that are now invalid (approved against old constitution)
        let approvals: Vec<Approval> = artifacts.iter()
            .filter(|a| a.artifact_type != ArtifactType::Constitution && a.artifact_type != ArtifactType::ExecutionReadinessGate)
            .map(|a| Approval {
                id: format!("appr-{}", a.id),
                project_id: proj_id.into(),
                artifact_id: a.id.clone(),
                artifact_version_id: format!("{}-v1", a.id),
                artifact_content_hash: format!("hash-{}", a.id),
                approval_type: ApprovalType::Standard,
                approver: designer.clone(),
                rationale: None,
                created_at: "2026-03-06T00:00:00Z".into(),
            })
            .collect();

        let links = vec![
            make_link_for("es-wf", "es-const", TraceLinkType::DerivesFrom, &designer, proj_id),
            make_link_for("es-feat", "es-wf", TraceLinkType::Justifies, &designer, proj_id),
            make_link_for("es-sys", "es-feat", TraceLinkType::Implements, &designer, proj_id),
            make_link_for("es-ux", "es-wf", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("es-ux", "es-sys", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("es-phase", "es-const", TraceLinkType::ValidatedBy, &designer, proj_id),
            make_link_for("es-phase", "es-ux", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("es-check", "es-phase", TraceLinkType::DependsOn, &designer, proj_id),
            make_link_for("es-drift", "es-const", TraceLinkType::InvalidatedBy, &designer, proj_id),
        ];

        // The amendment that caused this
        let amendments = vec![
            Amendment {
                id: "amend-es-1".into(),
                project_id: proj_id.into(),
                target_constitution_version_id: "es-cv1".into(),
                proposed_changes: ConstitutionPatch {
                    one_sentence_promise: None,
                    user_fantasy: None,
                    non_negotiable_outcomes: Some(vec![
                        "Fire magic has consequences and memory".into(),
                        "The world reacts to how you use flame".into(),
                        "Companion spirits are now bound to flame affinity".into(),
                    ]),
                    anti_goals: None,
                    quality_bar: None,
                    failure_condition: None,
                },
                reason: "Companion spirits must be bound to flame affinity for narrative coherence".into(),
                expected_impact_summary: "7 artifacts invalidated — full reconciliation required".into(),
                invalidated_artifact_ids: artifacts.iter()
                    .filter(|a| a.state == ArtifactState::Stale)
                    .map(|a| a.id.clone())
                    .collect(),
                resulting_constitution_version_id: Some("es-cv2".into()),
                proposer: designer.clone(),
                status: AmendmentStatus::Applied,
                created_at: "2026-03-12T00:00:00Z".into(),
                updated_at: "2026-03-13T00:00:00Z".into(),
                applied_at: Some("2026-03-13T00:00:00Z".into()),
            },
        ];

        let mut project = project;
        project.artifact_ids = artifacts.iter().map(|a| a.id.clone()).collect();

        ProjectStore {
            project,
            constitution,
            artifacts,
            versions,
            approvals,
            links,
            alarms: vec![],
            amendments,
            audit_events: vec![],
        }
    }
}

fn make_artifact(id: &str, art_type: ArtifactType, title: &str, state: ArtifactState) -> Artifact {
    make_artifact_for(id, art_type, title, state, "proj-1")
}

fn make_artifact_for(id: &str, art_type: ArtifactType, title: &str, state: ArtifactState, proj_id: &str) -> Artifact {
    Artifact {
        id: id.into(),
        project_id: proj_id.into(),
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
    make_link_for(source, target, link_type, identity, "proj-1")
}

fn make_link_for(
    source: &str,
    target: &str,
    link_type: TraceLinkType,
    identity: &LocalIdentity,
    proj_id: &str,
) -> TraceLink {
    TraceLink {
        id: format!("link-{}-{}", source, target),
        project_id: proj_id.into(),
        source_node_id: source.into(),
        target_node_id: target.into(),
        link_type,
        rationale: "Justified per constitution".into(),
        created_by: identity.clone(),
        created_at: "2026-03-13T00:00:00Z".into(),
    }
}

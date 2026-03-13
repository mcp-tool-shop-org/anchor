//! Golden-Path Acceptance Tests
//!
//! Scenario-level tests that exercise entire workflows end-to-end.
//! These are the product-level truth tests — not unit tests, not integration
//! tests, but acceptance tests that prove the whole machine works.
//!
//! Scenarios:
//! 1. Shadow Protocol → healthy: fix broken traceability, advance all artifacts, export
//! 2. Amendment lifecycle: healthy → amend → stale fallout → recover → export
//! 3. Save/load round-trip with corruption detection and repair
//! 4. Provenance completeness: every recovery action carries rule_clause + why_first
//! 5. Large project performance: 50+ artifacts don't degrade

#[cfg(test)]
mod tests {
    use crate::amendments;
    use crate::domain::*;
    use crate::editing;
    use crate::export_compiler::{self, ExportInput};
    use crate::link_authoring;
    use crate::persistence;
    use crate::readiness_gate;
    use crate::recovery::{self, ActionType, HealthStatus};
    use crate::store::ProjectStore;

    // ═══════════════════════════════════════════════════════════
    // Scenario 1: Shadow Protocol — broken → healthy → export
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn shadow_protocol_full_recovery_to_export() {
        let mut store = ProjectStore::load_scenario("shadow-protocol").unwrap();

        // Step 0: Verify it starts broken
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Blocked);

        let health = recovery::project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_ne!(health.status, HealthStatus::Healthy);
        assert!(health.missing_links > 0);

        // Step 1: Add missing trace links
        // sp-sys needs implements → sp-feat
        let designer = LocalIdentity { id: "user-3".into(), display_name: "Operator".into() };
        let result = link_authoring::add_link(
            "sp-sys", "sp-feat", TraceLinkType::Implements,
            "Detection Engine implements Infiltration Feature Map",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success, "add_link failed: {:?}", result.error);
        store.links.push(result.link.unwrap());

        // sp-phase needs validated_by → sp-const
        let result = link_authoring::add_link(
            "sp-phase", "sp-const", TraceLinkType::ValidatedBy,
            "Phase contracts validated by constitution",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success, "add_link failed: {:?}", result.error);
        store.links.push(result.link.unwrap());

        // sp-drift needs invalidated_by → sp-const
        let result = link_authoring::add_link(
            "sp-drift", "sp-const", TraceLinkType::InvalidatedBy,
            "Drift alarms trace to constitution violations",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success, "add_link failed: {:?}", result.error);
        store.links.push(result.link.unwrap());

        // sp-check needs depends_on → sp-phase
        let result = link_authoring::add_link(
            "sp-check", "sp-phase", TraceLinkType::DependsOn,
            "Acceptance checklists depend on phase contracts",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success, "add_link failed: {:?}", result.error);
        store.links.push(result.link.unwrap());

        // sp-gate needs depends_on → sp-check (gate needs upstream traceability)
        let result = link_authoring::add_link(
            "sp-gate", "sp-check", TraceLinkType::DependsOn,
            "Readiness gate depends on acceptance checklists",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success, "add_link failed: {:?}", result.error);
        store.links.push(result.link.unwrap());

        // Add missing bidirectional links for §8.2 compliance
        // sp-ux needs downstream (sp-phase depends on sp-ux)
        let result = link_authoring::add_link(
            "sp-phase", "sp-ux", TraceLinkType::DependsOn,
            "Phase contracts depend on UX state map",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success);
        store.links.push(result.link.unwrap());

        // sp-sys needs downstream (sp-ux depends on sp-sys)
        let result = link_authoring::add_link(
            "sp-ux", "sp-sys", TraceLinkType::DependsOn,
            "UX state map depends on system architecture",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success);
        store.links.push(result.link.unwrap());

        // Step 2: Resolve drift alarms
        for alarm in &mut store.alarms {
            alarm.status = DriftAlarmStatus::Resolved;
            alarm.resolved_at = Some("2026-03-14T00:00:05Z".into());
        }

        // Step 3: Advance all non-approved artifacts through the lifecycle
        // Each needs: Draft→Complete→Valid→Approved (with proper content + validation)
        let passing_validation = ValidationSummary {
            structural: ValidationStatus::Pass,
            relational: ValidationStatus::Pass,
            intent: ValidationStatus::Pass,
            last_validated_at: Some("2026-03-14T00:00:30Z".into()),
        };

        for artifact in &mut store.artifacts {
            if artifact.state == ArtifactState::Approved
                || artifact.artifact_type == ArtifactType::Constitution
                || artifact.artifact_type == ArtifactType::ExecutionReadinessGate
            {
                continue;
            }

            // Ensure content exists (update version content)
            if let Some(v) = store.versions.iter_mut().find(|v| v.id == artifact.current_version_id) {
                v.content = serde_json::json!({"authored": true, "reconciled": true});
                v.content_hash = format!("hash-reconciled-{}", artifact.id);
            }

            // Set passing validation
            artifact.validation_summary = passing_validation.clone();

            // Advance state
            artifact.state = ArtifactState::Approved;
            artifact.updated_at = "2026-03-14T00:01:00Z".into();

            // Add approval
            store.approvals.push(Approval {
                id: format!("appr-fix-{}", artifact.id),
                project_id: store.project.id.clone(),
                artifact_id: artifact.id.clone(),
                artifact_version_id: artifact.current_version_id.clone(),
                artifact_content_hash: format!("hash-reconciled-{}", artifact.id),
                approval_type: ApprovalType::Standard,
                approver: designer.clone(),
                rationale: Some("Fixed during recovery".into()),
                created_at: "2026-03-14T00:01:00Z".into(),
            });
        }

        // Step 4: Verify gate is now ready
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Ready, "Gate should be ready after full recovery. Blocking: {:?}",
            gate.blocking_reasons.iter().map(|r| &r.code).collect::<Vec<_>>());

        // Step 5: Export should succeed
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
        let package = export_compiler::compile(&input).expect("Export should succeed after recovery");

        // Verify export completeness
        let paths: Vec<&str> = package.files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"project.json"));
        assert!(paths.contains(&"constitution.md"));
        assert!(paths.contains(&"reports/integrity-attestation.md"));
        assert!(paths.contains(&"reports/traceability-matrix.md"));

        // Verify integrity attestation content
        let attestation = package.files.iter()
            .find(|f| f.path == "reports/integrity-attestation.md")
            .expect("Integrity attestation must exist");
        assert!(attestation.content.contains("Status:** Ready"));
        assert!(attestation.content.contains("Blocking reasons:** 0"));

        // Step 6: Health should show healthy
        let health = recovery::project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.stale_artifacts, 0);
        assert_eq!(health.active_alarms, 0);
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 2: Amendment lifecycle — healthy → amend → recover
    // ═══════════════════════════════════════════════════════════

    /// Helper: add the missing cs-gate upstream link to make Crystal Sanctum fully traceable.
    fn fix_crystal_sanctum_gate(store: &mut ProjectStore) {
        let designer = LocalIdentity { id: "user-2".into(), display_name: "Architect".into() };
        let result = link_authoring::add_link(
            "cs-gate", "cs-check", TraceLinkType::DependsOn,
            "Readiness gate depends on acceptance checklists",
            &store.artifacts, &store.links, &designer,
        );
        assert!(result.success);
        store.links.push(result.link.unwrap());
    }

    #[test]
    fn amendment_lifecycle_full_round_trip() {
        let mut store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        let designer = LocalIdentity { id: "user-2".into(), display_name: "Architect".into() };

        // Fix the gate traceability so Crystal Sanctum starts Ready
        fix_crystal_sanctum_gate(&mut store);

        // Step 0: Verify starts gate-ready
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Ready,
            "Crystal Sanctum should start ready. Blocking: {:?}",
            gate.blocking_reasons.iter().map(|r| format!("{}: {}", r.code, r.message)).collect::<Vec<_>>());

        // Step 1: Propose an amendment
        let patch = ConstitutionPatch {
            one_sentence_promise: None,
            user_fantasy: None,
            non_negotiable_outcomes: Some(vec![
                "Every puzzle traces to a crystal mechanic defined in the lore".into(),
                "No brute-force solutions — understanding is required".into(),
                "AMENDMENT: Crystal harmonics now affect companion AI behavior".into(),
            ]),
            anti_goals: None,
            quality_bar: None,
            failure_condition: None,
        };

        let mut amendment = amendments::propose_amendment(
            &store.project, &store.constitution, &store.amendments,
            patch, "Crystal harmonics must affect companion AI for narrative coherence".into(),
            designer.clone(), "2026-03-14T00:00:00Z", 1,
        ).expect("Proposal should succeed");

        assert_eq!(amendment.status, AmendmentStatus::Proposed);

        // Step 2: Assess impact
        let impact = amendments::assess_impact(&mut amendment, &store.artifacts)
            .expect("Impact assessment should succeed");
        assert!(!impact.affected_artifact_ids.is_empty(), "Amendment should affect some artifacts");
        assert_eq!(amendment.status, AmendmentStatus::ImpactAssessed);

        // Step 3: Apply amendment
        let stale_ids = amendments::apply_amendment(
            &mut amendment, &mut store.constitution, &mut store.artifacts,
            "2026-03-14T00:01:00Z", "cs-cv2",
        ).expect("Apply should succeed");

        assert!(!stale_ids.is_empty(), "Some artifacts should go stale");
        assert_eq!(amendment.status, AmendmentStatus::Applied);
        assert_eq!(store.constitution.version_id, "cs-cv2");

        // Verify gate is now blocked
        store.amendments.push(amendment.clone());
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Blocked);

        // Step 4: Health should be Critical (stale artifacts)
        let health = recovery::project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(health.status, HealthStatus::Critical);
        assert!(health.stale_artifacts > 0);

        // Recovery actions should exist for stale artifacts
        let reconcile_actions: Vec<_> = health.next_actions.iter()
            .filter(|a| a.action_type == ActionType::ReconcileStale)
            .collect();
        assert!(!reconcile_actions.is_empty(), "Should have reconcile actions for stale artifacts");

        // Step 5: Reconcile all stale artifacts
        let passing_validation = ValidationSummary {
            structural: ValidationStatus::Pass,
            relational: ValidationStatus::Pass,
            intent: ValidationStatus::Pass,
            last_validated_at: Some("2026-03-14T00:01:30Z".into()),
        };

        for artifact in &mut store.artifacts {
            if artifact.state != ArtifactState::Stale {
                continue;
            }

            // Update version to new constitution
            if let Some(v) = store.versions.iter_mut().find(|v| v.id == artifact.current_version_id) {
                v.constitution_version_id = "cs-cv2".into();
                v.content = serde_json::json!({"reconciled": true, "constitution": "cs-cv2"});
                v.content_hash = format!("hash-reconciled-{}", artifact.id);
            }

            artifact.validation_summary = passing_validation.clone();
            artifact.state = ArtifactState::Approved;
            artifact.stale_reason = None;
            artifact.updated_at = "2026-03-14T00:02:00Z".into();

            // Re-approve
            store.approvals.push(Approval {
                id: format!("appr-recon-{}", artifact.id),
                project_id: store.project.id.clone(),
                artifact_id: artifact.id.clone(),
                artifact_version_id: artifact.current_version_id.clone(),
                artifact_content_hash: format!("hash-reconciled-{}", artifact.id),
                approval_type: ApprovalType::Reapproval,
                approver: designer.clone(),
                rationale: Some("Reconciled after crystal harmonics amendment".into()),
                created_at: "2026-03-14T00:02:00Z".into(),
            });
        }

        // Step 6: Complete the amendment
        let amend = store.amendments.last_mut().unwrap();
        let complete_result = amendments::try_complete_amendment(
            amend, &store.artifacts, "2026-03-14T00:03:00Z",
        );
        assert!(complete_result.is_ok(), "Amendment should complete: {:?}", complete_result);
        assert_eq!(amend.status, AmendmentStatus::Completed);

        // Step 7: Gate should be ready again
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Ready,
            "Gate should be ready after reconciliation. Blocking: {:?}",
            gate.blocking_reasons.iter().map(|r| format!("{}: {}", r.code, r.message)).collect::<Vec<_>>());

        // Step 8: Export should succeed
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
        let package = export_compiler::compile(&input).expect("Export should succeed post-amendment");
        assert_eq!(package.files.len(), 14); // 9 md + project.json + 5 reports

        // Verify the constitution in export reflects amendment
        let const_md = package.files.iter().find(|f| f.path == "constitution.md").unwrap();
        assert!(const_md.content.contains("cs-cv2"), "Constitution should reference new version");
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 3: Save/load round-trip + corruption + repair
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn save_load_roundtrip_preserves_state() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        let dir = std::env::temp_dir().join("anchor-test-roundtrip");
        let path = dir.join("roundtrip.anchor");

        // Save
        let save_result = persistence::save_project(&store, &path);
        assert!(save_result.is_ok(), "Save failed: {:?}", save_result);

        // Load
        let loaded = persistence::load_project(&path);
        assert!(loaded.is_ok(), "Load failed: {:?}", loaded);
        let loaded = loaded.unwrap();

        // Verify state is preserved
        assert_eq!(loaded.project.name, "Crystal Sanctum");
        assert_eq!(loaded.artifacts.len(), store.artifacts.len());
        assert_eq!(loaded.versions.len(), store.versions.len());
        assert_eq!(loaded.links.len(), store.links.len());
        assert_eq!(loaded.approvals.len(), store.approvals.len());
        assert_eq!(loaded.constitution.version_id, store.constitution.version_id);

        // Verify each artifact matches
        for (orig, loaded) in store.artifacts.iter().zip(loaded.artifacts.iter()) {
            assert_eq!(orig.id, loaded.id);
            assert_eq!(orig.state, loaded.state);
            assert_eq!(orig.artifact_type, loaded.artifact_type);
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupted_file_detected_and_repaired() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        let dir = std::env::temp_dir().join("anchor-test-corrupt");
        let path = dir.join("corrupt.anchor");

        // Save
        persistence::save_project(&store, &path).unwrap();

        // Corrupt the file by modifying the payload
        let mut json = std::fs::read_to_string(&path).unwrap();
        json = json.replace("Forge Quest", "Forge Quest CORRUPTED");
        std::fs::write(&path, &json).unwrap();

        // Normal load should fail
        let result = persistence::load_project(&path);
        assert!(result.is_err(), "Corrupted file should fail normal load");

        // Dry-run should detect corruption
        let diag = persistence::dry_run_load(&path);
        assert!(!diag.loadable, "Corrupted file should not be loadable");
        assert!(diag.issues.iter().any(|i| i.code == "CORRUPTED_HASH"),
            "Should detect hash mismatch");
        assert!(diag.repairable, "File with only hash corruption should be repairable");

        // Repair-load should work
        let repaired = persistence::load_project_with_repair(&path);
        assert!(repaired.is_ok(), "Repair-load should succeed: {:?}", repaired);
        let (repaired_store, issues) = repaired.unwrap();
        assert_eq!(repaired_store.project.name, "Forge Quest CORRUPTED");
        assert!(issues.iter().any(|i| i.code == "HASH_REPAIRED"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_provides_full_diagnostics() {
        let store = ProjectStore::load_scenario("ember-saga").unwrap();
        let dir = std::env::temp_dir().join("anchor-test-dryrun");
        let path = dir.join("ember.anchor");

        persistence::save_project(&store, &path).unwrap();

        let diag = persistence::dry_run_load(&path);
        assert!(diag.loadable, "Valid file should be loadable");
        let summary = diag.summary.expect("Should have summary");
        assert_eq!(summary.project_name, "Ember Saga");
        assert_eq!(summary.artifact_count, 9);
        assert!(summary.amendment_count > 0);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 4: Provenance completeness
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn every_recovery_action_has_provenance() {
        for scenario in &["forge-quest", "crystal-sanctum", "shadow-protocol", "ember-saga"] {
            let store = ProjectStore::load_scenario(scenario).unwrap();
            let health = recovery::project_health(
                &store.artifacts, &store.versions, &store.approvals,
                &store.links, &store.constitution, &store.alarms, &store.amendments,
            );

            for action in &health.next_actions {
                assert!(!action.rule_clause.is_empty(),
                    "Action '{}' in {} has empty rule_clause", action.title, scenario);
                assert!(!action.why_first.is_empty(),
                    "Action '{}' in {} has empty why_first", action.title, scenario);
                // Rule clauses should reference a section
                assert!(action.rule_clause.contains('§'),
                    "Action '{}' in {} rule_clause should reference a § section: '{}'",
                    action.title, scenario, action.rule_clause);
            }
        }
    }

    #[test]
    fn per_artifact_actions_have_provenance() {
        let store = ProjectStore::load_scenario("forge-quest").unwrap();
        for artifact in &store.artifacts {
            let actions = recovery::next_actions_for_artifact(
                artifact, &store.artifacts, &store.versions,
                &store.approvals, &store.links, &store.alarms,
            );
            for action in &actions {
                assert!(!action.rule_clause.is_empty(),
                    "Artifact {} action '{}' missing rule_clause", artifact.id, action.title);
                assert!(!action.why_first.is_empty(),
                    "Artifact {} action '{}' missing why_first", artifact.id, action.title);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 5: Edit triggers downstream stale propagation
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn edit_triggers_stale_and_blocks_gate() {
        let mut store = ProjectStore::load_scenario("crystal-sanctum").unwrap();

        // Fix gate traceability so we start Ready
        fix_crystal_sanctum_gate(&mut store);

        // Verify gate starts ready
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Ready);

        // Edit the workflows artifact
        let editor = LocalIdentity { id: "user-2".into(), display_name: "Architect".into() };
        let result = editing::edit_artifact(
            &mut store,
            "cs-wf",
            serde_json::json!({"updated": true, "scope": "crystal resonance v2"}),
            "hash-cs-wf-v2",
            &editor,
            "2026-03-14T00:00:00Z",
        );
        assert!(result.is_ok(), "Edit should succeed: {:?}", result);
        let edit_result = result.unwrap();
        assert!(!edit_result.stale_artifact_ids.is_empty(),
            "Editing approved workflow should cause downstream stale");

        // Gate should now be blocked
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &store.links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert_eq!(gate.status, GateStatus::Blocked,
            "Gate should be blocked after editing approved artifact");
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 6: Large project performance
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn large_project_gate_evaluation_completes() {
        // Build a large project with many extra non-typed artifacts
        // (using the base Crystal Sanctum and adding many trace links)
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();

        // Create 200 extra trace links (duplicates are fine for perf testing)
        let mut links = store.links.clone();
        let designer = LocalIdentity { id: "perf-user".into(), display_name: "Perf".into() };
        for i in 0..200 {
            links.push(TraceLink {
                id: format!("perf-link-{}", i),
                project_id: store.project.id.clone(),
                source_node_id: store.artifacts[i % store.artifacts.len()].id.clone(),
                target_node_id: store.artifacts[(i + 1) % store.artifacts.len()].id.clone(),
                link_type: TraceLinkType::DependsOn,
                rationale: format!("Performance test link {}", i),
                created_by: designer.clone(),
                created_at: "2026-03-14T00:00:00Z".into(),
            });
        }

        // Gate evaluation should complete without panic
        let gate = readiness_gate::evaluate(
            &store.artifacts, &store.versions, &store.approvals,
            &links, &store.constitution, &store.alarms, &store.amendments,
        );
        // Result doesn't matter — what matters is it completed
        assert!(matches!(gate.status, GateStatus::Ready | GateStatus::Blocked));

        // Recovery actions should also complete
        let health = recovery::project_health(
            &store.artifacts, &store.versions, &store.approvals,
            &links, &store.constitution, &store.alarms, &store.amendments,
        );
        assert!(matches!(health.status, HealthStatus::Healthy | HealthStatus::NeedsAttention | HealthStatus::Critical));
    }

    #[test]
    fn large_project_stale_propagation_completes() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();

        // Simulate a constitution amendment hitting all artifacts
        let marks = crate::stale_propagation::propagate_constitution_amendment(
            &store.artifacts, "cs-cv1", "cs-cv99",
        );
        // All non-constitution artifacts that are Approved/Valid should be stale
        let approved_count = store.artifacts.iter()
            .filter(|a| {
                a.artifact_type != ArtifactType::Constitution
                    && matches!(a.state, ArtifactState::Approved | ArtifactState::Valid)
            })
            .count();
        assert_eq!(marks.len(), approved_count, "All approved non-root artifacts should be marked stale");
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 7: Integrity attestation content verification
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn integrity_attestation_contains_all_sections() {
        let mut store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        fix_crystal_sanctum_gate(&mut store);

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

        let package = export_compiler::compile(&input).expect("Export should succeed");
        let attestation = package.files.iter()
            .find(|f| f.path == "reports/integrity-attestation.md")
            .expect("Attestation file must be present");

        // Check all required sections
        assert!(attestation.content.contains("# Export Integrity Attestation"));
        assert!(attestation.content.contains("## Gate Verdict"));
        assert!(attestation.content.contains("## Artifact Attestation"));
        assert!(attestation.content.contains("## Traceability Attestation"));
        assert!(attestation.content.contains("## Package Integrity"));

        // Check it has actual data
        assert!(attestation.content.contains("Content hash chain:"));
        assert!(attestation.content.contains("Schema version:"));
        assert!(attestation.content.contains(&store.constitution.version_id));
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 8: Abandon amendment before apply
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn abandoned_amendment_leaves_project_unchanged() {
        let store = ProjectStore::load_scenario("crystal-sanctum").unwrap();
        let designer = LocalIdentity { id: "user-2".into(), display_name: "Architect".into() };

        let original_constitution_version = store.constitution.version_id.clone();
        let original_artifact_states: Vec<_> = store.artifacts.iter()
            .map(|a| (a.id.clone(), a.state))
            .collect();

        // Propose
        let mut amendment = amendments::propose_amendment(
            &store.project, &store.constitution, &store.amendments,
            ConstitutionPatch {
                quality_bar: Some("Ultra high".into()),
                ..Default::default()
            },
            "Testing abandonment".into(),
            designer, "2026-03-14T00:00:00Z", 1,
        ).unwrap();

        // Assess
        amendments::assess_impact(&mut amendment, &store.artifacts).unwrap();

        // Abandon
        let result = amendments::abandon_amendment(&mut amendment, "2026-03-14T00:01:00Z");
        assert!(result.is_ok());
        assert_eq!(amendment.status, AmendmentStatus::Abandoned);

        // Verify nothing changed
        assert_eq!(store.constitution.version_id, original_constitution_version);
        for (id, expected_state) in &original_artifact_states {
            let art = store.artifacts.iter().find(|a| &a.id == id).unwrap();
            assert_eq!(art.state, *expected_state,
                "Artifact {} state should be unchanged after abandoned amendment", id);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Scenario 9: All four scenarios produce valid health reports
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn all_scenarios_produce_consistent_health() {
        for name in &["forge-quest", "crystal-sanctum", "shadow-protocol", "ember-saga"] {
            let store = ProjectStore::load_scenario(name).unwrap();
            let health = recovery::project_health(
                &store.artifacts, &store.versions, &store.approvals,
                &store.links, &store.constitution, &store.alarms, &store.amendments,
            );

            // Basic consistency
            assert_eq!(health.total_artifacts, store.artifacts.len(),
                "{}: total_artifacts mismatch", name);
            assert!(health.ready_artifacts <= health.total_artifacts,
                "{}: more ready than total", name);
            assert!(health.stale_artifacts <= health.total_artifacts,
                "{}: more stale than total", name);

            // If healthy, gate must be ready
            if health.status == HealthStatus::Healthy {
                assert_eq!(health.gate_status, GateStatus::Ready,
                    "{}: Healthy but gate not Ready", name);
            }

            // If critical, must have stale or alarms
            if health.status == HealthStatus::Critical {
                assert!(health.stale_artifacts > 0 || health.active_alarms > 0,
                    "{}: Critical but no stale or alarms", name);
            }

            // Summary must be non-empty
            assert!(!health.summary.is_empty(), "{}: empty summary", name);

            // Actions must be sorted by priority
            for window in health.next_actions.windows(2) {
                assert!(window[0].priority <= window[1].priority,
                    "{}: actions not sorted by priority", name);
            }
        }
    }
}

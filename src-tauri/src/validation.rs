//! Anchor Law Engine — Explicit Validation
//!
//! Every artifact has a validation story: why it's valid, or exactly
//! why it's not, and what the user should do about it.
//!
//! This module produces per-artifact validation reports with:
//! - Per-layer (structural, relational, intent) results
//! - Specific failure reasons with human-readable explanations
//! - Resolution steps: what must happen for this check to pass
//! - Rule provenance: which constitutional clause requires this

use crate::domain::*;
use crate::traceability;

// ─── Validation Report ──────────────────────────────────────

/// Complete validation report for a single artifact.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub artifact_id: String,
    pub artifact_type: ArtifactType,
    pub artifact_state: ArtifactState,
    pub overall_status: ValidationVerdict,
    pub checks: Vec<ValidationCheck>,
    pub resolution_summary: Option<String>,
}

/// Overall verdict: is this artifact on the path to Approved?
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationVerdict {
    /// All checks pass. Ready to advance.
    AllClear,
    /// Some checks fail but the artifact can still progress.
    HasWarnings,
    /// Blocking failures that must be resolved.
    Blocked,
}

/// A single validation check result.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationCheck {
    pub check_id: String,
    pub layer: ValidationLayer,
    pub status: CheckStatus,
    pub title: String,
    pub explanation: String,
    pub resolution_steps: Vec<String>,
    pub affected_artifact_ids: Vec<String>,
    pub rule_clause: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    NotApplicable,
}

// ─── Validate Artifact ──────────────────────────────────────

/// Produce a complete validation report for a single artifact.
pub fn validate_artifact(
    artifact_id: &str,
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    approvals: &[Approval],
    links: &[TraceLink],
    constitution: &Constitution,
) -> Option<ValidationReport> {
    let artifact = artifacts.iter().find(|a| a.id == artifact_id)?;
    let mut checks = Vec::new();

    // ── Structural checks ───────────────────────────────────
    checks.push(check_has_content(artifact, versions));
    checks.push(check_content_not_placeholder(artifact, versions));
    checks.push(check_version_exists(artifact, versions));

    // ── Relational checks ───────────────────────────────────
    checks.push(check_upstream_traceability(artifact, links, artifacts));
    checks.push(check_no_broken_links(artifact, links, artifacts));
    checks.push(check_constitution_alignment(artifact, versions, constitution));

    // ── Intent checks ───────────────────────────────────────
    checks.push(check_not_stale(artifact));
    checks.push(check_approval_current(artifact, approvals, versions, constitution));
    checks.push(check_downstream_health(artifact, links, artifacts));

    // ── State-specific checks ───────────────────────────────
    if artifact.artifact_type == ArtifactType::ExecutionReadinessGate {
        checks.push(ValidationCheck {
            check_id: "gate-computed".into(),
            layer: ValidationLayer::Structural,
            status: CheckStatus::NotApplicable,
            title: "Readiness Gate is computed".into(),
            explanation: "This artifact is auto-computed by the engine, not authored.".into(),
            resolution_steps: vec![],
            affected_artifact_ids: vec![],
            rule_clause: "§13 — Gate is derived".into(),
        });
    }

    let overall_status = compute_verdict(&checks);
    let resolution_summary = build_resolution_summary(&checks, artifact);

    Some(ValidationReport {
        artifact_id: artifact.id.clone(),
        artifact_type: artifact.artifact_type,
        artifact_state: artifact.state,
        overall_status,
        checks,
        resolution_summary,
    })
}

// ─── Individual Checks ──────────────────────────────────────

fn check_has_content(artifact: &Artifact, versions: &[ArtifactVersion]) -> ValidationCheck {
    let version = versions.iter().find(|v| v.id == artifact.current_version_id);
    let has_content = version.map_or(false, |v| !v.content.is_null());

    ValidationCheck {
        check_id: "structural-content-exists".into(),
        layer: ValidationLayer::Structural,
        status: if has_content { CheckStatus::Pass } else { CheckStatus::Fail },
        title: "Artifact has content".into(),
        explanation: if has_content {
            "Content is present in the current version.".into()
        } else {
            "No content found. Every artifact must have authored content before it can advance past Draft.".into()
        },
        resolution_steps: if has_content {
            vec![]
        } else {
            vec!["Open the artifact editor and provide content.".into()]
        },
        affected_artifact_ids: vec![artifact.id.clone()],
        rule_clause: "§5.1 — All artifacts must have content".into(),
    }
}

fn check_content_not_placeholder(artifact: &Artifact, versions: &[ArtifactVersion]) -> ValidationCheck {
    let version = versions.iter().find(|v| v.id == artifact.current_version_id);
    let is_placeholder = version.map_or(true, |v| {
        v.content.get("placeholder").and_then(|p| p.as_bool()) == Some(true)
    });

    ValidationCheck {
        check_id: "structural-not-placeholder".into(),
        layer: ValidationLayer::Structural,
        status: if is_placeholder { CheckStatus::Warning } else { CheckStatus::Pass },
        title: "Content is not placeholder".into(),
        explanation: if is_placeholder {
            "Content appears to be placeholder data. Replace with real authored content.".into()
        } else {
            "Content appears to be authored, not placeholder.".into()
        },
        resolution_steps: if is_placeholder {
            vec![
                "Edit the artifact to replace placeholder content.".into(),
                "Ensure all required fields are populated with real data.".into(),
            ]
        } else {
            vec![]
        },
        affected_artifact_ids: vec![artifact.id.clone()],
        rule_clause: "§5.2 — Placeholder content cannot be validated".into(),
    }
}

fn check_version_exists(artifact: &Artifact, versions: &[ArtifactVersion]) -> ValidationCheck {
    let has_version = versions.iter().any(|v| v.id == artifact.current_version_id);

    ValidationCheck {
        check_id: "structural-version-exists".into(),
        layer: ValidationLayer::Structural,
        status: if has_version { CheckStatus::Pass } else { CheckStatus::Fail },
        title: "Version record exists".into(),
        explanation: if has_version {
            "Current version record is present and linked.".into()
        } else {
            format!(
                "Version {} referenced by artifact but not found. This indicates data corruption.",
                artifact.current_version_id
            )
        },
        resolution_steps: if has_version {
            vec![]
        } else {
            vec!["Re-save the project file. If the error persists, the project may need repair.".into()]
        },
        affected_artifact_ids: vec![artifact.id.clone()],
        rule_clause: "§5 — Version integrity".into(),
    }
}

fn check_upstream_traceability(
    artifact: &Artifact,
    links: &[TraceLink],
    artifacts: &[Artifact],
) -> ValidationCheck {
    // Constitution doesn't need upstream links
    if artifact.artifact_type == ArtifactType::Constitution {
        return ValidationCheck {
            check_id: "relational-upstream".into(),
            layer: ValidationLayer::Relational,
            status: CheckStatus::NotApplicable,
            title: "Upstream traceability".into(),
            explanation: "Constitution is the root — no upstream links required.".into(),
            resolution_steps: vec![],
            affected_artifact_ids: vec![],
            rule_clause: "§8.1 — Constitution is root".into(),
        };
    }

    // ExecutionReadinessGate is computed
    if artifact.artifact_type == ArtifactType::ExecutionReadinessGate {
        return ValidationCheck {
            check_id: "relational-upstream".into(),
            layer: ValidationLayer::Relational,
            status: CheckStatus::NotApplicable,
            title: "Upstream traceability".into(),
            explanation: "Readiness Gate is computed, not traced.".into(),
            resolution_steps: vec![],
            affected_artifact_ids: vec![],
            rule_clause: "§13 — Gate is derived".into(),
        };
    }

    let upstream = traceability::upstream_links(&artifact.id, links);
    let has_upstream = !upstream.is_empty();

    // Check if upstream targets actually exist
    let missing: Vec<String> = upstream
        .iter()
        .filter(|l| !artifacts.iter().any(|a| a.id == l.target_node_id))
        .map(|l| l.target_node_id.clone())
        .collect();

    let status = if !has_upstream {
        CheckStatus::Fail
    } else if !missing.is_empty() {
        CheckStatus::Fail
    } else {
        CheckStatus::Pass
    };

    let explanation = if !has_upstream {
        format!(
            "{:?} requires upstream trace links. No links found pointing from this artifact to its justification sources.",
            artifact.artifact_type
        )
    } else if !missing.is_empty() {
        format!(
            "Trace links exist but reference missing artifacts: {}",
            missing.join(", ")
        )
    } else {
        format!(
            "Has {} upstream trace link(s) to valid artifacts.",
            upstream.len()
        )
    };

    let resolution_steps = if !has_upstream {
        vec![
            format!("Add a trace link from {} to its upstream justification source.", artifact.id),
            "Required link types depend on the artifact type (see §8.1).".into(),
        ]
    } else if !missing.is_empty() {
        vec![
            "Fix or remove trace links that reference non-existent artifacts.".into(),
        ]
    } else {
        vec![]
    };

    ValidationCheck {
        check_id: "relational-upstream".into(),
        layer: ValidationLayer::Relational,
        status,
        title: "Upstream traceability".into(),
        explanation,
        resolution_steps,
        affected_artifact_ids: if !missing.is_empty() { missing } else { vec![artifact.id.clone()] },
        rule_clause: "§8.1 — Required upstream trace links".into(),
    }
}

fn check_no_broken_links(
    artifact: &Artifact,
    links: &[TraceLink],
    artifacts: &[Artifact],
) -> ValidationCheck {
    let outgoing = traceability::upstream_links(&artifact.id, links);
    let incoming = traceability::downstream_links(&artifact.id, links);
    let all_links: Vec<_> = outgoing.into_iter().chain(incoming).collect();

    let broken: Vec<String> = all_links
        .iter()
        .filter(|l| {
            !artifacts.iter().any(|a| a.id == l.source_node_id)
                || !artifacts.iter().any(|a| a.id == l.target_node_id)
        })
        .map(|l| l.id.clone())
        .collect();

    ValidationCheck {
        check_id: "relational-no-broken-links".into(),
        layer: ValidationLayer::Relational,
        status: if broken.is_empty() { CheckStatus::Pass } else { CheckStatus::Fail },
        title: "No broken trace links".into(),
        explanation: if broken.is_empty() {
            "All trace links reference existing artifacts.".into()
        } else {
            format!("{} trace link(s) reference non-existent artifacts: {}", broken.len(), broken.join(", "))
        },
        resolution_steps: if broken.is_empty() {
            vec![]
        } else {
            vec!["Remove or repair broken trace links.".into()]
        },
        affected_artifact_ids: broken,
        rule_clause: "§8 — Link integrity".into(),
    }
}

fn check_constitution_alignment(
    artifact: &Artifact,
    versions: &[ArtifactVersion],
    constitution: &Constitution,
) -> ValidationCheck {
    let version = versions.iter().find(|v| v.id == artifact.current_version_id);
    let aligned = version.map_or(false, |v| {
        v.constitution_version_id == constitution.version_id
    });

    ValidationCheck {
        check_id: "relational-constitution-alignment".into(),
        layer: ValidationLayer::Relational,
        status: if aligned { CheckStatus::Pass } else { CheckStatus::Warning },
        title: "Constitution version alignment".into(),
        explanation: if aligned {
            format!(
                "Current version was authored against constitution {}.",
                constitution.version_id
            )
        } else {
            format!(
                "Version was authored against constitution {}, but current is {}. Content may need review.",
                version.map_or("unknown", |v| v.constitution_version_id.as_str()),
                constitution.version_id,
            )
        },
        resolution_steps: if aligned {
            vec![]
        } else {
            vec![
                "Review content against the current constitution version.".into(),
                "Edit and re-save to update the constitution reference.".into(),
            ]
        },
        affected_artifact_ids: vec![artifact.id.clone()],
        rule_clause: "§9 — Constitution alignment".into(),
    }
}

fn check_not_stale(artifact: &Artifact) -> ValidationCheck {
    let is_stale = artifact.state == ArtifactState::Stale;

    ValidationCheck {
        check_id: "intent-not-stale".into(),
        layer: ValidationLayer::Intent,
        status: if is_stale { CheckStatus::Fail } else { CheckStatus::Pass },
        title: "Artifact is not stale".into(),
        explanation: if is_stale {
            format!(
                "Marked stale: {}. Must be reconciled before validation or approval.",
                artifact.stale_reason.as_deref().unwrap_or("upstream change")
            )
        } else {
            "Artifact is not stale.".into()
        },
        resolution_steps: if is_stale {
            vec![
                "Review the stale reason and check what changed upstream.".into(),
                "Update content to reconcile with upstream changes.".into(),
                "Transition back to Draft, then re-author through Complete → Valid → Approved.".into(),
            ]
        } else {
            vec![]
        },
        affected_artifact_ids: vec![artifact.id.clone()],
        rule_clause: "§10 — Stale artifacts must be reconciled".into(),
    }
}

fn check_approval_current(
    artifact: &Artifact,
    approvals: &[Approval],
    versions: &[ArtifactVersion],
    constitution: &Constitution,
) -> ValidationCheck {
    let approval = approvals.iter().find(|a| a.artifact_id == artifact.id);
    let version = versions.iter().find(|v| v.id == artifact.current_version_id);

    match (approval, version) {
        (Some(appr), Some(ver)) => {
            let version_match = appr.artifact_version_id == ver.id;
            let hash_match = appr.artifact_content_hash == ver.content_hash;
            let const_match = ver.constitution_version_id == constitution.version_id;

            if version_match && hash_match && const_match {
                ValidationCheck {
                    check_id: "intent-approval-current".into(),
                    layer: ValidationLayer::Intent,
                    status: CheckStatus::Pass,
                    title: "Approval is current".into(),
                    explanation: format!(
                        "Approved at version {} with matching content hash.",
                        appr.artifact_version_id
                    ),
                    resolution_steps: vec![],
                    affected_artifact_ids: vec![],
                    rule_clause: "§11 — Approval validity".into(),
                }
            } else {
                let mut reasons = Vec::new();
                if !version_match {
                    reasons.push(format!(
                        "Approval was for version {}, current is {}",
                        appr.artifact_version_id, ver.id
                    ));
                }
                if !hash_match {
                    reasons.push("Content hash has changed since approval".into());
                }
                if !const_match {
                    reasons.push(format!(
                        "Constitution changed: version was authored against {}, current is {}",
                        ver.constitution_version_id, constitution.version_id
                    ));
                }
                ValidationCheck {
                    check_id: "intent-approval-current".into(),
                    layer: ValidationLayer::Intent,
                    status: CheckStatus::Fail,
                    title: "Approval is outdated".into(),
                    explanation: format!("Approval is no longer valid: {}", reasons.join("; ")),
                    resolution_steps: vec![
                        "Review the content changes since the last approval.".into(),
                        "Re-validate and re-approve the artifact.".into(),
                    ],
                    affected_artifact_ids: vec![artifact.id.clone()],
                    rule_clause: "§11 — Approval must match current version and constitution".into(),
                }
            }
        }
        (None, _) if artifact.state == ArtifactState::Approved => {
            ValidationCheck {
                check_id: "intent-approval-current".into(),
                layer: ValidationLayer::Intent,
                status: CheckStatus::Fail,
                title: "Approval record missing".into(),
                explanation: "Artifact is in Approved state but no approval record found.".into(),
                resolution_steps: vec!["Re-approve the artifact.".into()],
                affected_artifact_ids: vec![artifact.id.clone()],
                rule_clause: "§11 — Approved artifacts must have approval records".into(),
            }
        }
        _ => {
            ValidationCheck {
                check_id: "intent-approval-current".into(),
                layer: ValidationLayer::Intent,
                status: CheckStatus::NotApplicable,
                title: "Approval status".into(),
                explanation: "Artifact is not yet in Approved state.".into(),
                resolution_steps: vec![],
                affected_artifact_ids: vec![],
                rule_clause: "§11 — Approval".into(),
            }
        }
    }
}

fn check_downstream_health(
    artifact: &Artifact,
    links: &[TraceLink],
    artifacts: &[Artifact],
) -> ValidationCheck {
    let downstream = traceability::downstream_links(&artifact.id, links);
    let stale_downstream: Vec<String> = downstream
        .iter()
        .filter_map(|l| {
            artifacts.iter().find(|a| {
                a.id == l.source_node_id && a.state == ArtifactState::Stale
            })
        })
        .map(|a| a.id.clone())
        .collect();

    ValidationCheck {
        check_id: "intent-downstream-health".into(),
        layer: ValidationLayer::Intent,
        status: if stale_downstream.is_empty() { CheckStatus::Pass } else { CheckStatus::Warning },
        title: "Downstream artifact health".into(),
        explanation: if stale_downstream.is_empty() {
            format!("{} downstream dependent(s) — none are stale.", downstream.len())
        } else {
            format!(
                "{} downstream dependent(s) are stale: {}. Changes to this artifact may have caused this.",
                stale_downstream.len(),
                stale_downstream.join(", ")
            )
        },
        resolution_steps: if stale_downstream.is_empty() {
            vec![]
        } else {
            vec![
                "Review stale downstream artifacts.".into(),
                "Each must be reconciled and re-approved independently.".into(),
            ]
        },
        affected_artifact_ids: stale_downstream,
        rule_clause: "§10 — Stale propagation awareness".into(),
    }
}

// ─── Verdict Computation ────────────────────────────────────

fn compute_verdict(checks: &[ValidationCheck]) -> ValidationVerdict {
    let has_fail = checks.iter().any(|c| c.status == CheckStatus::Fail);
    let has_warning = checks.iter().any(|c| c.status == CheckStatus::Warning);

    if has_fail {
        ValidationVerdict::Blocked
    } else if has_warning {
        ValidationVerdict::HasWarnings
    } else {
        ValidationVerdict::AllClear
    }
}

fn build_resolution_summary(checks: &[ValidationCheck], artifact: &Artifact) -> Option<String> {
    let failing: Vec<&ValidationCheck> = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail || c.status == CheckStatus::Warning)
        .collect();

    if failing.is_empty() {
        return None;
    }

    let fail_count = checks.iter().filter(|c| c.status == CheckStatus::Fail).count();
    let warn_count = checks.iter().filter(|c| c.status == CheckStatus::Warning).count();

    let mut parts = Vec::new();
    if fail_count > 0 {
        parts.push(format!("{} blocking failure(s)", fail_count));
    }
    if warn_count > 0 {
        parts.push(format!("{} warning(s)", warn_count));
    }

    let next_step = match artifact.state {
        ArtifactState::Draft => "Resolve failures, then transition to Complete.",
        ArtifactState::Complete => "Resolve failures, then transition to Valid.",
        ArtifactState::Valid => "Resolve warnings, then approve.",
        ArtifactState::Approved => "Content may have changed since approval — re-review.",
        ArtifactState::Stale => "Reconcile with upstream changes, then restart from Draft.",
    };

    Some(format!("{} — {}", parts.join(", "), next_step))
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ProjectStore;

    fn demo() -> ProjectStore {
        ProjectStore::demo()
    }

    #[test]
    fn approved_artifact_validates_clean() {
        let s = demo();
        let report = validate_artifact(
            "art-wf", &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
        ).unwrap();
        // Approved artifact with proper links: should have no blocking failures
        let fail_count = report.checks.iter().filter(|c| c.status == CheckStatus::Fail).count();
        assert_eq!(fail_count, 0, "Approved artifact should have no failures: {:?}", report);
    }

    #[test]
    fn draft_artifact_has_warnings() {
        let s = demo();
        let report = validate_artifact(
            "art-drift", &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
        ).unwrap();
        // Draft with placeholder content should at least have warnings
        assert!(
            report.checks.iter().any(|c| c.status == CheckStatus::Warning || c.status == CheckStatus::Fail),
            "Draft should have issues: {:?}", report
        );
    }

    #[test]
    fn constitution_skips_upstream_check() {
        let s = demo();
        let report = validate_artifact(
            "art-const", &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
        ).unwrap();
        let upstream_check = report.checks.iter().find(|c| c.check_id == "relational-upstream").unwrap();
        assert_eq!(upstream_check.status, CheckStatus::NotApplicable);
    }

    #[test]
    fn gate_artifact_marks_not_applicable() {
        let s = demo();
        let report = validate_artifact(
            "art-gate", &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
        ).unwrap();
        let gate_check = report.checks.iter().find(|c| c.check_id == "gate-computed");
        assert!(gate_check.is_some());
        assert_eq!(gate_check.unwrap().status, CheckStatus::NotApplicable);
    }

    #[test]
    fn nonexistent_artifact_returns_none() {
        let s = demo();
        let report = validate_artifact(
            "does-not-exist", &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
        );
        assert!(report.is_none());
    }

    #[test]
    fn stale_artifact_fails_intent_check() {
        let mut s = demo();
        // Force an artifact stale
        if let Some(a) = s.artifacts.iter_mut().find(|a| a.id == "art-feat") {
            a.state = ArtifactState::Stale;
            a.stale_reason = Some("Constitution amended".into());
        }
        let report = validate_artifact(
            "art-feat", &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
        ).unwrap();
        let stale_check = report.checks.iter().find(|c| c.check_id == "intent-not-stale").unwrap();
        assert_eq!(stale_check.status, CheckStatus::Fail);
    }

    #[test]
    fn verdict_all_clear_when_no_failures() {
        let checks = vec![
            ValidationCheck {
                check_id: "test".into(),
                layer: ValidationLayer::Structural,
                status: CheckStatus::Pass,
                title: "test".into(),
                explanation: "test".into(),
                resolution_steps: vec![],
                affected_artifact_ids: vec![],
                rule_clause: "test".into(),
            },
        ];
        assert_eq!(compute_verdict(&checks), ValidationVerdict::AllClear);
    }

    #[test]
    fn verdict_blocked_when_failure_present() {
        let checks = vec![
            ValidationCheck {
                check_id: "test".into(),
                layer: ValidationLayer::Structural,
                status: CheckStatus::Fail,
                title: "test".into(),
                explanation: "test".into(),
                resolution_steps: vec![],
                affected_artifact_ids: vec![],
                rule_clause: "test".into(),
            },
        ];
        assert_eq!(compute_verdict(&checks), ValidationVerdict::Blocked);
    }

    #[test]
    fn verdict_warnings_without_failures() {
        let checks = vec![
            ValidationCheck {
                check_id: "test".into(),
                layer: ValidationLayer::Structural,
                status: CheckStatus::Warning,
                title: "test".into(),
                explanation: "test".into(),
                resolution_steps: vec![],
                affected_artifact_ids: vec![],
                rule_clause: "test".into(),
            },
        ];
        assert_eq!(compute_verdict(&checks), ValidationVerdict::HasWarnings);
    }

    #[test]
    fn all_artifacts_can_be_validated() {
        let s = demo();
        for artifact in &s.artifacts {
            let report = validate_artifact(
                &artifact.id, &s.artifacts, &s.versions, &s.approvals, &s.links, &s.constitution,
            );
            assert!(report.is_some(), "Validation failed for {}", artifact.id);
        }
    }
}

//! Anchor Law Engine — Version Diffing
//!
//! Content diff between artifact versions. Metadata diff.
//! Approval invalidation tied to exact changes.
//!
//! Answers: "What changed?" and "Why did my approval break?"

use serde::Serialize;

use crate::domain::*;

// ─── Diff Result ────────────────────────────────────────────

/// Complete diff between two versions of an artifact.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDiff {
    pub artifact_id: String,
    pub from_version: VersionSummary,
    pub to_version: VersionSummary,
    pub content_changes: Vec<ContentChange>,
    pub metadata_changes: Vec<MetadataChange>,
    pub approval_impact: ApprovalImpact,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub version_id: String,
    pub version_number: u32,
    pub content_hash: String,
    pub constitution_version_id: String,
    pub created_at: String,
}

/// A single content field change between versions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentChange {
    pub field_path: String,
    pub change_type: ChangeType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
}

/// A metadata change between versions (non-content).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

/// How the change affects existing approvals.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalImpact {
    pub approval_invalidated: bool,
    pub reason: Option<String>,
    pub approval_was_for_version: Option<String>,
    pub changes_since_approval: Vec<String>,
}

// ─── Diff Engine ────────────────────────────────────────────

/// Compute a diff between two versions of the same artifact.
pub fn diff_versions(
    artifact_id: &str,
    from_version_id: &str,
    to_version_id: &str,
    versions: &[ArtifactVersion],
    approvals: &[Approval],
) -> Option<VersionDiff> {
    let from = versions.iter().find(|v| v.id == from_version_id && v.artifact_id == artifact_id)?;
    let to = versions.iter().find(|v| v.id == to_version_id && v.artifact_id == artifact_id)?;

    let content_changes = diff_json_content(&from.content, &to.content, "");
    let metadata_changes = diff_metadata(from, to);
    let approval_impact = compute_approval_impact(artifact_id, from, to, approvals, &content_changes);

    Some(VersionDiff {
        artifact_id: artifact_id.into(),
        from_version: VersionSummary {
            version_id: from.id.clone(),
            version_number: from.version_number,
            content_hash: from.content_hash.clone(),
            constitution_version_id: from.constitution_version_id.clone(),
            created_at: from.created_at.clone(),
        },
        to_version: VersionSummary {
            version_id: to.id.clone(),
            version_number: to.version_number,
            content_hash: to.content_hash.clone(),
            constitution_version_id: to.constitution_version_id.clone(),
            created_at: to.created_at.clone(),
        },
        content_changes,
        metadata_changes,
        approval_impact,
    })
}

/// Compute a diff between an artifact's current version and its previous version.
/// Returns None if there's only one version.
pub fn diff_latest(
    artifact_id: &str,
    artifacts: &[Artifact],
    versions: &[ArtifactVersion],
    approvals: &[Approval],
) -> Option<VersionDiff> {
    let artifact = artifacts.iter().find(|a| a.id == artifact_id)?;
    let artifact_versions: Vec<&ArtifactVersion> = {
        let mut vs: Vec<_> = versions.iter().filter(|v| v.artifact_id == artifact_id).collect();
        vs.sort_by_key(|v| v.version_number);
        vs
    };

    if artifact_versions.len() < 2 {
        return None; // Only one version, no diff possible
    }

    let current = artifact_versions.iter().find(|v| v.id == artifact.current_version_id)?;
    let previous = artifact_versions.iter().rev()
        .find(|v| v.version_number < current.version_number)?;

    diff_versions(artifact_id, &previous.id, &current.id, versions, approvals)
}

// ─── JSON Content Diffing ───────────────────────────────────

fn diff_json_content(
    old: &serde_json::Value,
    new: &serde_json::Value,
    prefix: &str,
) -> Vec<ContentChange> {
    let mut changes = Vec::new();

    match (old, new) {
        (serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
            // Check for removed/modified keys
            for (key, old_val) in old_map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                match new_map.get(key) {
                    None => {
                        changes.push(ContentChange {
                            field_path: path,
                            change_type: ChangeType::Removed,
                            old_value: Some(value_preview(old_val)),
                            new_value: None,
                        });
                    }
                    Some(new_val) if old_val != new_val => {
                        // Recurse into nested objects
                        if old_val.is_object() && new_val.is_object() {
                            changes.extend(diff_json_content(old_val, new_val, &path));
                        } else {
                            changes.push(ContentChange {
                                field_path: path,
                                change_type: ChangeType::Modified,
                                old_value: Some(value_preview(old_val)),
                                new_value: Some(value_preview(new_val)),
                            });
                        }
                    }
                    _ => {} // unchanged
                }
            }
            // Check for added keys
            for (key, new_val) in new_map {
                if !old_map.contains_key(key) {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    changes.push(ContentChange {
                        field_path: path,
                        change_type: ChangeType::Added,
                        old_value: None,
                        new_value: Some(value_preview(new_val)),
                    });
                }
            }
        }
        _ if old != new => {
            let path = if prefix.is_empty() { "(root)".into() } else { prefix.into() };
            changes.push(ContentChange {
                field_path: path,
                change_type: ChangeType::Modified,
                old_value: Some(value_preview(old)),
                new_value: Some(value_preview(new)),
            });
        }
        _ => {} // equal non-objects
    }

    changes
}

fn value_preview(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => {
            if s.len() > 100 {
                format!("{}...", &s[..100])
            } else {
                s.clone()
            }
        }
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => {
            let s = serde_json::to_string(val).unwrap_or_default();
            if s.len() > 120 {
                format!("{}...", &s[..120])
            } else {
                s
            }
        }
    }
}

// ─── Metadata Diffing ───────────────────────────────────────

fn diff_metadata(from: &ArtifactVersion, to: &ArtifactVersion) -> Vec<MetadataChange> {
    let mut changes = Vec::new();

    if from.constitution_version_id != to.constitution_version_id {
        changes.push(MetadataChange {
            field: "constitutionVersionId".into(),
            old_value: from.constitution_version_id.clone(),
            new_value: to.constitution_version_id.clone(),
        });
    }

    if from.content_hash != to.content_hash {
        changes.push(MetadataChange {
            field: "contentHash".into(),
            old_value: from.content_hash.clone(),
            new_value: to.content_hash.clone(),
        });
    }

    changes
}

// ─── Approval Impact ────────────────────────────────────────

fn compute_approval_impact(
    artifact_id: &str,
    _from: &ArtifactVersion,
    to: &ArtifactVersion,
    approvals: &[Approval],
    content_changes: &[ContentChange],
) -> ApprovalImpact {
    let approval = approvals.iter().find(|a| a.artifact_id == artifact_id);

    match approval {
        Some(appr) => {
            let was_for = &appr.artifact_version_id;
            let invalidated = appr.artifact_version_id != to.id
                || appr.artifact_content_hash != to.content_hash;

            let changes_since: Vec<String> = content_changes
                .iter()
                .map(|c| match c.change_type {
                    ChangeType::Added => format!("Added: {}", c.field_path),
                    ChangeType::Removed => format!("Removed: {}", c.field_path),
                    ChangeType::Modified => format!("Changed: {}", c.field_path),
                })
                .collect();

            let reason = if invalidated {
                if appr.artifact_version_id != to.id && !content_changes.is_empty() {
                    Some(format!(
                        "Content changed across {} field(s) since approval at version {}",
                        content_changes.len(),
                        was_for
                    ))
                } else if appr.artifact_content_hash != to.content_hash {
                    Some(format!(
                        "Content hash changed from {} to {}",
                        appr.artifact_content_hash, to.content_hash
                    ))
                } else {
                    Some(format!("Version changed from {} to {}", was_for, to.id))
                }
            } else {
                None
            };

            ApprovalImpact {
                approval_invalidated: invalidated,
                reason,
                approval_was_for_version: Some(was_for.clone()),
                changes_since_approval: changes_since,
            }
        }
        None => ApprovalImpact {
            approval_invalidated: false,
            reason: None,
            approval_was_for_version: None,
            changes_since_approval: vec![],
        },
    }
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
    fn json_diff_detects_added_field() {
        let old = serde_json::json!({"a": 1});
        let new = serde_json::json!({"a": 1, "b": 2});
        let changes = diff_json_content(&old, &new, "");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Added);
        assert_eq!(changes[0].field_path, "b");
    }

    #[test]
    fn json_diff_detects_removed_field() {
        let old = serde_json::json!({"a": 1, "b": 2});
        let new = serde_json::json!({"a": 1});
        let changes = diff_json_content(&old, &new, "");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Removed);
        assert_eq!(changes[0].field_path, "b");
    }

    #[test]
    fn json_diff_detects_modified_field() {
        let old = serde_json::json!({"a": 1});
        let new = serde_json::json!({"a": 2});
        let changes = diff_json_content(&old, &new, "");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn json_diff_recurses_into_nested_objects() {
        let old = serde_json::json!({"outer": {"inner": "old"}});
        let new = serde_json::json!({"outer": {"inner": "new"}});
        let changes = diff_json_content(&old, &new, "");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_path, "outer.inner");
        assert_eq!(changes[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn json_diff_empty_when_equal() {
        let v = serde_json::json!({"a": 1, "b": [1, 2, 3]});
        let changes = diff_json_content(&v, &v, "");
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_versions_returns_none_for_missing_version() {
        let s = demo();
        let result = diff_versions("art-wf", "nonexistent", "also-nonexistent", &s.versions, &s.approvals);
        assert!(result.is_none());
    }

    #[test]
    fn diff_versions_works_with_same_version() {
        let s = demo();
        let result = diff_versions("art-wf", "art-wf-v1", "art-wf-v1", &s.versions, &s.approvals);
        assert!(result.is_some());
        let diff = result.unwrap();
        assert!(diff.content_changes.is_empty(), "Same version should have no content changes");
        assert!(diff.metadata_changes.is_empty(), "Same version should have no metadata changes");
    }

    #[test]
    fn diff_latest_returns_none_for_single_version() {
        let s = demo();
        // All demo artifacts have exactly 1 version
        let result = diff_latest("art-wf", &s.artifacts, &s.versions, &s.approvals);
        assert!(result.is_none(), "Single-version artifact should return None for diff_latest");
    }

    #[test]
    fn diff_latest_with_two_versions() {
        let mut s = demo();
        let designer = LocalIdentity {
            id: "user-1".into(),
            display_name: "Designer".into(),
        };
        // Add a second version with different content
        s.versions.push(ArtifactVersion {
            id: "art-wf-v2".into(),
            artifact_id: "art-wf".into(),
            project_id: "proj-1".into(),
            version_number: 2,
            constitution_version_id: "cv1".into(),
            content: serde_json::json!({"updated": true, "detail": "real content"}),
            content_hash: "hash-v2".into(),
            parent_version_id: Some("art-wf-v1".into()),
            created_at: "2026-03-14T00:00:00Z".into(),
            created_by: designer,
        });
        // Point artifact to new version
        s.artifacts.iter_mut().find(|a| a.id == "art-wf").unwrap().current_version_id = "art-wf-v2".into();

        let result = diff_latest("art-wf", &s.artifacts, &s.versions, &s.approvals);
        assert!(result.is_some());
        let diff = result.unwrap();
        assert!(!diff.content_changes.is_empty(), "Should detect content changes");
        assert!(diff.approval_impact.approval_invalidated, "Approval should be invalidated");
    }

    #[test]
    fn approval_impact_detects_invalidation() {
        let old_version = ArtifactVersion {
            id: "v1".into(),
            artifact_id: "art-1".into(),
            project_id: "proj-1".into(),
            version_number: 1,
            constitution_version_id: "cv1".into(),
            content: serde_json::json!({"a": 1}),
            content_hash: "hash-old".into(),
            parent_version_id: None,
            created_at: "2026-03-13T00:00:00Z".into(),
            created_by: LocalIdentity { id: "u".into(), display_name: "U".into() },
        };
        let new_version = ArtifactVersion {
            id: "v2".into(),
            artifact_id: "art-1".into(),
            project_id: "proj-1".into(),
            version_number: 2,
            constitution_version_id: "cv1".into(),
            content: serde_json::json!({"a": 2}),
            content_hash: "hash-new".into(),
            parent_version_id: Some("v1".into()),
            created_at: "2026-03-14T00:00:00Z".into(),
            created_by: LocalIdentity { id: "u".into(), display_name: "U".into() },
        };
        let approval = Approval {
            id: "appr-1".into(),
            project_id: "proj-1".into(),
            artifact_id: "art-1".into(),
            artifact_version_id: "v1".into(),
            artifact_content_hash: "hash-old".into(),
            approval_type: ApprovalType::Standard,
            approver: LocalIdentity { id: "u".into(), display_name: "U".into() },
            rationale: None,
            created_at: "2026-03-13T00:00:00Z".into(),
        };

        let changes = diff_json_content(&old_version.content, &new_version.content, "");
        let impact = compute_approval_impact("art-1", &old_version, &new_version, &[approval], &changes);
        assert!(impact.approval_invalidated);
        assert!(impact.reason.is_some());
        assert!(!impact.changes_since_approval.is_empty());
    }

    #[test]
    fn value_preview_truncates_long_strings() {
        let long_string = "x".repeat(200);
        let val = serde_json::Value::String(long_string.clone());
        let preview = value_preview(&val);
        assert!(preview.len() < long_string.len());
        assert!(preview.ends_with("..."));
    }
}

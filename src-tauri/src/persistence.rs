//! Anchor Law Engine — Persistence
//!
//! Durable project storage with explicit load/save boundaries.
//! Projects are saved as versioned JSON files with integrity checking.
//!
//! File format:
//! - Top-level object with `anchor_file_version` + `schema_version`
//! - Full project state serialized deterministically
//! - Content hash for corruption detection
//!
//! Migration: `anchor_file_version` tracks the persistence format.
//! `schema_version` tracks the domain model version.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::*;
use crate::store::ProjectStore;

/// Current persistence file format version.
pub const ANCHOR_FILE_VERSION: &str = "1.0.0";

// ─── Project File Format ────────────────────────────────────

/// The on-disk representation. This is the contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorProjectFile {
    /// Persistence format version (for migration).
    pub anchor_file_version: String,
    /// Domain schema version.
    pub schema_version: String,
    /// SHA-256 hex digest of the `payload` field when serialized.
    pub content_hash: String,
    /// The actual project data.
    pub payload: ProjectPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPayload {
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

// ─── Errors ─────────────────────────────────────────────────

#[derive(Debug)]
pub enum PersistenceError {
    /// File I/O failure.
    Io(io::Error),
    /// JSON parse/serialize failure.
    SerdeJson(serde_json::Error),
    /// The file was readable but the content hash doesn't match.
    CorruptedFile {
        expected_hash: String,
        actual_hash: String,
    },
    /// File version is newer than this binary supports.
    UnsupportedFileVersion {
        file_version: String,
        max_supported: String,
    },
    /// Schema version mismatch (future: migration needed).
    SchemaMismatch {
        file_schema: String,
        engine_schema: String,
    },
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::SerdeJson(e) => write!(f, "JSON error: {}", e),
            Self::CorruptedFile { expected_hash, actual_hash } => {
                write!(
                    f,
                    "File corrupted: expected hash {}, got {}",
                    expected_hash, actual_hash
                )
            }
            Self::UnsupportedFileVersion { file_version, max_supported } => {
                write!(
                    f,
                    "Unsupported file version: {} (max supported: {})",
                    file_version, max_supported
                )
            }
            Self::SchemaMismatch { file_schema, engine_schema } => {
                write!(
                    f,
                    "Schema mismatch: file has {}, engine expects {}",
                    file_schema, engine_schema
                )
            }
        }
    }
}

impl From<io::Error> for PersistenceError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for PersistenceError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e)
    }
}

// ─── Content Hashing ────────────────────────────────────────

/// Deterministic hash of the payload for integrity checking.
/// Uses a simple djb2-style hash to avoid pulling in a crypto crate.
/// The hash covers the canonical JSON serialization of the payload.
fn compute_content_hash(payload: &ProjectPayload) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string(payload)?;
    let hash = djb2_hash(&json);
    Ok(format!("{:016x}", hash))
}

/// djb2 hash — deterministic, fast, no external deps.
pub fn djb2_hash(data: &str) -> u64 {
    let mut hash: u64 = 5381;
    for &byte in data.as_bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

// ─── Save ───────────────────────────────────────────────────

/// Save the project store to a file. Creates parent directories if needed.
pub fn save_project(store: &ProjectStore, path: &Path) -> Result<PathBuf, PersistenceError> {
    let payload = store_to_payload(store);
    let content_hash = compute_content_hash(&payload)?;

    let file = AnchorProjectFile {
        anchor_file_version: ANCHOR_FILE_VERSION.into(),
        schema_version: SCHEMA_VERSION.into(),
        content_hash,
        payload,
    };

    let json = serde_json::to_string_pretty(&file)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write to temp file first, then rename for atomic save.
    let temp_path = path.with_extension("anchor.tmp");
    std::fs::write(&temp_path, &json)?;
    std::fs::rename(&temp_path, path)?;

    Ok(path.to_path_buf())
}

// ─── Load ───────────────────────────────────────────────────

/// Load a project file from disk. Validates version and integrity.
pub fn load_project(path: &Path) -> Result<ProjectStore, PersistenceError> {
    let json = std::fs::read_to_string(path)?;
    load_project_from_str(&json)
}

/// Load from a JSON string (useful for testing without disk).
pub fn load_project_from_str(json: &str) -> Result<ProjectStore, PersistenceError> {
    let file: AnchorProjectFile = serde_json::from_str(json)?;

    // Version gate: reject files from the future.
    if file.anchor_file_version != ANCHOR_FILE_VERSION {
        return Err(PersistenceError::UnsupportedFileVersion {
            file_version: file.anchor_file_version,
            max_supported: ANCHOR_FILE_VERSION.into(),
        });
    }

    // Schema version check (for now: must match exactly).
    if file.schema_version != SCHEMA_VERSION {
        return Err(PersistenceError::SchemaMismatch {
            file_schema: file.schema_version,
            engine_schema: SCHEMA_VERSION.into(),
        });
    }

    // Integrity check.
    let actual_hash = compute_content_hash(&file.payload)?;
    if actual_hash != file.content_hash {
        return Err(PersistenceError::CorruptedFile {
            expected_hash: file.content_hash,
            actual_hash,
        });
    }

    Ok(payload_to_store(file.payload))
}

// ─── Dry-Run Load (Import Diagnostics) ──────────────────────

/// Result of a dry-run load: what would happen if we loaded this file?
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDiagnostic {
    /// Can this file be loaded at all?
    pub loadable: bool,
    /// Human-readable summary of what this file contains.
    pub summary: Option<ImportSummary>,
    /// All issues found during validation.
    pub issues: Vec<ImportIssue>,
    /// Can we repair this file automatically?
    pub repairable: bool,
    /// What repair would do.
    pub repair_description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub project_name: String,
    pub file_version: String,
    pub schema_version: String,
    pub artifact_count: usize,
    pub version_count: usize,
    pub link_count: usize,
    pub amendment_count: usize,
    pub audit_event_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// File can still be loaded.
    Warning,
    /// File cannot be loaded without repair.
    Error,
    /// File cannot be loaded or repaired — fatal.
    Fatal,
}

/// Dry-run load: analyze a file without actually replacing the current project.
/// Returns diagnostics about what would happen if you loaded this file.
pub fn dry_run_load(path: &Path) -> ImportDiagnostic {
    let json = match std::fs::read_to_string(path) {
        Ok(j) => j,
        Err(e) => {
            return ImportDiagnostic {
                loadable: false,
                summary: None,
                issues: vec![ImportIssue {
                    severity: IssueSeverity::Fatal,
                    code: "IO_ERROR".into(),
                    message: format!("Cannot read file: {}", e),
                    detail: Some(format!("Path: {}", path.display())),
                }],
                repairable: false,
                repair_description: None,
            };
        }
    };
    dry_run_load_from_str(&json)
}

/// Dry-run load from a JSON string (useful for testing).
pub fn dry_run_load_from_str(json: &str) -> ImportDiagnostic {
    let mut issues = Vec::new();

    // Step 1: Can we parse JSON at all?
    let file: AnchorProjectFile = match serde_json::from_str(json) {
        Ok(f) => f,
        Err(e) => {
            return ImportDiagnostic {
                loadable: false,
                summary: None,
                issues: vec![ImportIssue {
                    severity: IssueSeverity::Fatal,
                    code: "JSON_PARSE_ERROR".into(),
                    message: "File is not valid JSON or doesn't match the Anchor project format.".into(),
                    detail: Some(format!("Parser error: {}", e)),
                }],
                repairable: false,
                repair_description: None,
            };
        }
    };

    let summary = ImportSummary {
        project_name: file.payload.project.name.clone(),
        file_version: file.anchor_file_version.clone(),
        schema_version: file.schema_version.clone(),
        artifact_count: file.payload.artifacts.len(),
        version_count: file.payload.versions.len(),
        link_count: file.payload.links.len(),
        amendment_count: file.payload.amendments.len(),
        audit_event_count: file.payload.audit_events.len(),
    };

    // Step 2: Version compatibility
    if file.anchor_file_version != ANCHOR_FILE_VERSION {
        issues.push(ImportIssue {
            severity: IssueSeverity::Error,
            code: "UNSUPPORTED_FILE_VERSION".into(),
            message: format!(
                "File version {} is not supported (engine supports {}).",
                file.anchor_file_version, ANCHOR_FILE_VERSION
            ),
            detail: Some("The file was created by a newer or older version of Anchor.".into()),
        });
    }

    // Step 3: Schema version
    if file.schema_version != SCHEMA_VERSION {
        issues.push(ImportIssue {
            severity: IssueSeverity::Error,
            code: "SCHEMA_MISMATCH".into(),
            message: format!(
                "Schema version {} doesn't match engine version {}.",
                file.schema_version, SCHEMA_VERSION
            ),
            detail: Some("The domain model has changed. Migration may be possible in future versions.".into()),
        });
    }

    // Step 4: Integrity check
    let _hash_ok = match compute_content_hash(&file.payload) {
        Ok(actual) => {
            if actual != file.content_hash {
                issues.push(ImportIssue {
                    severity: IssueSeverity::Error,
                    code: "CORRUPTED_HASH".into(),
                    message: format!(
                        "Content hash mismatch: file says {}, computed {}.",
                        file.content_hash, actual
                    ),
                    detail: Some(
                        "The file contents have been modified outside of Anchor, or the file was partially written."
                            .into(),
                    ),
                });
                false
            } else {
                true
            }
        }
        Err(e) => {
            issues.push(ImportIssue {
                severity: IssueSeverity::Error,
                code: "HASH_COMPUTE_ERROR".into(),
                message: format!("Cannot compute content hash: {}", e),
                detail: None,
            });
            false
        }
    };

    // Step 5: Data consistency checks (warnings, not fatal)
    validate_payload_consistency(&file.payload, &mut issues);

    // Determine loadability and repairability
    let has_fatal = issues.iter().any(|i| i.severity == IssueSeverity::Fatal);
    let has_error = issues.iter().any(|i| i.severity == IssueSeverity::Error);

    // Repairable: if the only error is a corrupted hash, we can recompute it
    let repairable = !has_fatal && has_error
        && issues.iter().filter(|i| i.severity == IssueSeverity::Error).all(|i| i.code == "CORRUPTED_HASH");

    let repair_description = if repairable {
        Some("Recompute the content hash from the payload data. This repairs files that were edited externally.".into())
    } else {
        None
    };

    ImportDiagnostic {
        loadable: !has_fatal && !has_error,
        summary: Some(summary),
        issues,
        repairable,
        repair_description,
    }
}

/// Validate internal consistency of the payload (dangling references, etc.)
fn validate_payload_consistency(payload: &ProjectPayload, issues: &mut Vec<ImportIssue>) {
    let artifact_ids: Vec<&str> = payload.artifacts.iter().map(|a| a.id.as_str()).collect();

    // Check for artifacts referenced in versions but not in artifact list
    for version in &payload.versions {
        if !artifact_ids.contains(&version.artifact_id.as_str()) {
            issues.push(ImportIssue {
                severity: IssueSeverity::Warning,
                code: "ORPHAN_VERSION".into(),
                message: format!(
                    "Version {} references artifact {} which doesn't exist.",
                    version.id, version.artifact_id
                ),
                detail: None,
            });
        }
    }

    // Check for trace links pointing to nonexistent artifacts
    for link in &payload.links {
        if !artifact_ids.contains(&link.source_node_id.as_str()) {
            issues.push(ImportIssue {
                severity: IssueSeverity::Warning,
                code: "BROKEN_LINK_SOURCE".into(),
                message: format!(
                    "Trace link {} references missing source artifact {}.",
                    link.id, link.source_node_id
                ),
                detail: None,
            });
        }
        if !artifact_ids.contains(&link.target_node_id.as_str()) {
            issues.push(ImportIssue {
                severity: IssueSeverity::Warning,
                code: "BROKEN_LINK_TARGET".into(),
                message: format!(
                    "Trace link {} references missing target artifact {}.",
                    link.id, link.target_node_id
                ),
                detail: None,
            });
        }
    }

    // Check for approvals referencing nonexistent artifacts
    for approval in &payload.approvals {
        if !artifact_ids.contains(&approval.artifact_id.as_str()) {
            issues.push(ImportIssue {
                severity: IssueSeverity::Warning,
                code: "ORPHAN_APPROVAL".into(),
                message: format!(
                    "Approval {} references artifact {} which doesn't exist.",
                    approval.id, approval.artifact_id
                ),
                detail: None,
            });
        }
    }

    // Check artifact count matches ArtifactType::ALL
    let type_count = ArtifactType::ALL.len();
    if payload.artifacts.len() < type_count {
        issues.push(ImportIssue {
            severity: IssueSeverity::Warning,
            code: "MISSING_ARTIFACTS".into(),
            message: format!(
                "Project has {} artifacts but {} types are defined. Some artifact types may be missing.",
                payload.artifacts.len(), type_count
            ),
            detail: None,
        });
    }
}

/// Load with repair: if the file has a corrupted hash but is otherwise valid,
/// recompute the hash and load anyway.
pub fn load_project_with_repair(path: &Path) -> Result<(ProjectStore, Vec<ImportIssue>), PersistenceError> {
    let json = std::fs::read_to_string(path)?;
    load_project_with_repair_from_str(&json)
}

/// Load with repair from a string.
pub fn load_project_with_repair_from_str(json: &str) -> Result<(ProjectStore, Vec<ImportIssue>), PersistenceError> {
    let file: AnchorProjectFile = serde_json::from_str(json)?;
    let mut issues = Vec::new();

    // Version gate still enforced
    if file.anchor_file_version != ANCHOR_FILE_VERSION {
        return Err(PersistenceError::UnsupportedFileVersion {
            file_version: file.anchor_file_version,
            max_supported: ANCHOR_FILE_VERSION.into(),
        });
    }

    if file.schema_version != SCHEMA_VERSION {
        return Err(PersistenceError::SchemaMismatch {
            file_schema: file.schema_version,
            engine_schema: SCHEMA_VERSION.into(),
        });
    }

    // Check hash — repair instead of reject
    let actual_hash = compute_content_hash(&file.payload)?;
    if actual_hash != file.content_hash {
        issues.push(ImportIssue {
            severity: IssueSeverity::Warning,
            code: "HASH_REPAIRED".into(),
            message: format!(
                "Content hash was incorrect (expected {}, got {}). Loaded anyway with recomputed hash.",
                file.content_hash, actual_hash
            ),
            detail: Some("The file may have been edited externally. Data integrity cannot be guaranteed.".into()),
        });
    }

    // Consistency warnings
    validate_payload_consistency(&file.payload, &mut issues);

    Ok((payload_to_store(file.payload), issues))
}

// ─── Conversions ────────────────────────────────────────────

fn store_to_payload(store: &ProjectStore) -> ProjectPayload {
    ProjectPayload {
        project: store.project.clone(),
        constitution: store.constitution.clone(),
        artifacts: store.artifacts.clone(),
        versions: store.versions.clone(),
        approvals: store.approvals.clone(),
        links: store.links.clone(),
        alarms: store.alarms.clone(),
        amendments: store.amendments.clone(),
        audit_events: store.audit_events.clone(),
    }
}

fn payload_to_store(payload: ProjectPayload) -> ProjectStore {
    ProjectStore {
        project: payload.project,
        constitution: payload.constitution,
        artifacts: payload.artifacts,
        versions: payload.versions,
        approvals: payload.approvals,
        links: payload.links,
        alarms: payload.alarms,
        amendments: payload.amendments,
        audit_events: payload.audit_events,
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_store() -> ProjectStore {
        ProjectStore::demo()
    }

    #[test]
    fn round_trip_save_load() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: ANCHOR_FILE_VERSION.into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: compute_content_hash(&payload).unwrap(),
            payload,
        })
        .unwrap();

        let loaded = load_project_from_str(&json).unwrap();

        assert_eq!(store.project.id, loaded.project.id);
        assert_eq!(store.project.name, loaded.project.name);
        assert_eq!(store.artifacts.len(), loaded.artifacts.len());
        assert_eq!(store.versions.len(), loaded.versions.len());
        assert_eq!(store.approvals.len(), loaded.approvals.len());
        assert_eq!(store.links.len(), loaded.links.len());
        assert_eq!(store.amendments.len(), loaded.amendments.len());
        assert_eq!(store.audit_events.len(), loaded.audit_events.len());
    }

    #[test]
    fn deterministic_hash() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let hash1 = compute_content_hash(&payload).unwrap();
        let hash2 = compute_content_hash(&payload).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn detects_corruption() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: ANCHOR_FILE_VERSION.into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: "0000000000000000".into(), // wrong hash
            payload,
        })
        .unwrap();

        match load_project_from_str(&json) {
            Err(PersistenceError::CorruptedFile { .. }) => {} // expected
            other => panic!("Expected CorruptedFile, got {:?}", other),
        }
    }

    #[test]
    fn rejects_unsupported_file_version() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: "99.0.0".into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: compute_content_hash(&payload).unwrap(),
            payload,
        })
        .unwrap();

        match load_project_from_str(&json) {
            Err(PersistenceError::UnsupportedFileVersion { .. }) => {} // expected
            other => panic!("Expected UnsupportedFileVersion, got {:?}", other),
        }
    }

    #[test]
    fn rejects_schema_mismatch() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: ANCHOR_FILE_VERSION.into(),
            schema_version: "99.0.0".into(),
            content_hash: compute_content_hash(&payload).unwrap(),
            payload,
        })
        .unwrap();

        match load_project_from_str(&json) {
            Err(PersistenceError::SchemaMismatch { .. }) => {} // expected
            other => panic!("Expected SchemaMismatch, got {:?}", other),
        }
    }

    #[test]
    fn save_to_disk_and_load_back() {
        let store = demo_store();
        let dir = std::env::temp_dir().join("anchor_test_persistence");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-project.anchor.json");

        save_project(&store, &path).unwrap();
        assert!(path.exists());

        let loaded = load_project(&path).unwrap();
        assert_eq!(store.project.id, loaded.project.id);
        assert_eq!(store.artifacts.len(), loaded.artifacts.len());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn payload_preserves_all_artifact_states() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let restored = payload_to_store(payload);

        for (orig, rest) in store.artifacts.iter().zip(restored.artifacts.iter()) {
            assert_eq!(orig.id, rest.id);
            assert_eq!(orig.state, rest.state);
            assert_eq!(orig.artifact_type, rest.artifact_type);
            assert_eq!(orig.title, rest.title);
        }
    }

    #[test]
    fn content_hash_changes_on_mutation() {
        let store = demo_store();
        let payload1 = store_to_payload(&store);
        let hash1 = compute_content_hash(&payload1).unwrap();

        let mut payload2 = store_to_payload(&store);
        payload2.project.name = "Modified Name".into();
        let hash2 = compute_content_hash(&payload2).unwrap();

        assert_ne!(hash1, hash2);
    }

    // ─── Dry-Run / Import Diagnostic Tests ──────────────────

    #[test]
    fn dry_run_valid_file() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: ANCHOR_FILE_VERSION.into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: compute_content_hash(&payload).unwrap(),
            payload,
        })
        .unwrap();

        let diag = dry_run_load_from_str(&json);
        assert!(diag.loadable, "Valid file should be loadable: {:?}", diag.issues);
        assert!(diag.summary.is_some());
        let summary = diag.summary.unwrap();
        assert_eq!(summary.project_name, "Forge Quest");
        assert_eq!(summary.artifact_count, 9);
    }

    #[test]
    fn dry_run_corrupted_hash() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: ANCHOR_FILE_VERSION.into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: "0000000000000000".into(),
            payload,
        })
        .unwrap();

        let diag = dry_run_load_from_str(&json);
        assert!(!diag.loadable);
        assert!(diag.repairable, "Corrupted hash should be repairable");
        assert!(diag.issues.iter().any(|i| i.code == "CORRUPTED_HASH"));
    }

    #[test]
    fn dry_run_invalid_json() {
        let diag = dry_run_load_from_str("not json at all {{{");
        assert!(!diag.loadable);
        assert!(!diag.repairable);
        assert!(diag.issues.iter().any(|i| i.code == "JSON_PARSE_ERROR"));
    }

    #[test]
    fn dry_run_version_mismatch() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: "99.0.0".into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: compute_content_hash(&payload).unwrap(),
            payload,
        })
        .unwrap();

        let diag = dry_run_load_from_str(&json);
        assert!(!diag.loadable);
        assert!(diag.issues.iter().any(|i| i.code == "UNSUPPORTED_FILE_VERSION"));
    }

    #[test]
    fn load_with_repair_fixes_hash() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: ANCHOR_FILE_VERSION.into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: "0000000000000000".into(),
            payload,
        })
        .unwrap();

        // Normal load should fail
        assert!(load_project_from_str(&json).is_err());

        // Repair load should succeed
        let (loaded, issues) = load_project_with_repair_from_str(&json).unwrap();
        assert_eq!(loaded.project.name, "Forge Quest");
        assert!(issues.iter().any(|i| i.code == "HASH_REPAIRED"));
    }

    #[test]
    fn load_with_repair_still_rejects_version_mismatch() {
        let store = demo_store();
        let payload = store_to_payload(&store);
        let json = serde_json::to_string_pretty(&AnchorProjectFile {
            anchor_file_version: "99.0.0".into(),
            schema_version: SCHEMA_VERSION.into(),
            content_hash: compute_content_hash(&payload).unwrap(),
            payload,
        })
        .unwrap();

        assert!(load_project_with_repair_from_str(&json).is_err());
    }
}

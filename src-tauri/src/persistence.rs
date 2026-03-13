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
    let hash = djb2_hash(json.as_bytes());
    Ok(format!("{:016x}", hash))
}

/// djb2 hash — deterministic, fast, no external deps.
fn djb2_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for &byte in data {
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
}

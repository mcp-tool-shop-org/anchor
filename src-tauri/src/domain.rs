//! Anchor Canonical Schema Pack v1 — Rust Domain
//!
//! Mirrors `packages/schema/src/anchor-domain.ts` exactly.
//! Rust backend is final authority for validation, hashing,
//! state transitions, and export compilation.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0.0";

// ─── Canonical Enums ────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Constitution,
    UserFantasyWorkflows,
    FeatureMap,
    SystemArchitecture,
    UxStateMap,
    PhaseRoadmapContracts,
    AcceptanceChecklists,
    DriftAlarmDefinitions,
    ExecutionReadinessGate,
}

impl ArtifactType {
    pub const ALL: &[ArtifactType] = &[
        ArtifactType::Constitution,
        ArtifactType::UserFantasyWorkflows,
        ArtifactType::FeatureMap,
        ArtifactType::SystemArchitecture,
        ArtifactType::UxStateMap,
        ArtifactType::PhaseRoadmapContracts,
        ArtifactType::AcceptanceChecklists,
        ArtifactType::DriftAlarmDefinitions,
        ArtifactType::ExecutionReadinessGate,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Draft,
    Complete,
    Valid,
    Approved,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationLayer {
    Structural,
    Relational,
    Intent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pending,
    Pass,
    Fail,
    NeedsAmendment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftAlarmType {
    TraceabilityDrift,
    ConstitutionDrift,
    SequenceDrift,
    QualityDrift,
    ScopeDrift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftAlarmSeverity {
    Info,
    Warning,
    Error,
    Blocking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceLinkType {
    Justifies,
    DerivesFrom,
    Implements,
    DependsOn,
    ValidatedBy,
    InvalidatedBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalType {
    Standard,
    Reapproval,
    GateApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmendmentStatus {
    Proposed,
    ImpactAssessed,
    Applied,
    ReconciliationPending,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Blocked,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    ProjectCreated,
    ConstitutionLocked,
    ArtifactCreated,
    ArtifactUpdated,
    ArtifactCompleted,
    ArtifactValidated,
    ArtifactApproved,
    ArtifactMarkedStale,
    TraceLinkCreated,
    TraceLinkRemoved,
    AmendmentStarted,
    AmendmentImpactAssessed,
    AmendmentApplied,
    DriftAlarmRaised,
    DriftAlarmResolved,
    ExportBlocked,
    ReadinessGateComputed,
    ReadinessGatePassed,
    ProjectExported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftAlarmStatus {
    Active,
    Resolved,
    DismissedForAmendmentContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatedBy {
    Frontend,
    Backend,
    HumanReview,
}

// ─── Supporting Types ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalIdentity {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub export_format: String, // "json_and_markdown"
    pub optional_sqlite_indexing: bool,
    pub require_intent_review_rationale_on_failure: bool,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            export_format: "json_and_markdown".into(),
            optional_sqlite_indexing: false,
            require_intent_review_rationale_on_failure: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionPatch {
    pub one_sentence_promise: Option<String>,
    pub user_fantasy: Option<String>,
    pub non_negotiable_outcomes: Option<Vec<String>>,
    pub anti_goals: Option<Vec<String>>,
    pub quality_bar: Option<String>,
    pub failure_condition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    pub structural: ValidationStatus,
    pub relational: ValidationStatus,
    pub intent: ValidationStatus,
    pub last_validated_at: Option<String>,
}

impl Default for ValidationSummary {
    fn default() -> Self {
        Self {
            structural: ValidationStatus::Pending,
            relational: ValidationStatus::Pending,
            intent: ValidationStatus::Pending,
            last_validated_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleProvenance {
    pub source_artifact_type: SourceArtifactType,
    pub source_clause: String,
    pub human_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceArtifactType {
    Artifact(ArtifactType),
    SystemRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleResult {
    pub rule_id: String,
    pub status: RuleResultStatus,
    pub message: String,
    pub rule_provenance: RuleProvenance,
    pub affected_node_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleResultStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockingReason {
    pub code: String,
    pub message: String,
    pub affected_artifact_ids: Vec<String>,
    pub rule_provenance: RuleProvenance,
    pub remediation_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFileEntry {
    pub path: String,
    pub kind: ExportFileKind,
    pub derived_from_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFileKind {
    Json,
    Markdown,
    Report,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportManifestPreview {
    pub file_count: usize,
    pub files: Vec<ExportFileEntry>,
}

// ─── Domain Entities ────────────────────────────────────────

/// 4.1 Project — root aggregate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub schema_version: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: LocalIdentity,
    pub current_constitution_version_id: String,
    pub artifact_ids: Vec<String>,
    pub active_amendment_id: Option<String>,
    pub settings: ProjectSettings,
}

/// 4.2 Constitution — the throne, versioned, changes only via Amendment Protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constitution {
    pub id: String,
    pub artifact_id: String,
    pub version_id: String,
    pub project_id: String,
    pub one_sentence_promise: String,
    pub user_fantasy: String,
    pub non_negotiable_outcomes: Vec<String>,
    pub anti_goals: Vec<String>,
    pub quality_bar: String,
    pub failure_condition: String,
    pub locked: bool,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub approved_at: Option<String>,
    pub approved_by: Option<LocalIdentity>,
    pub parent_version_id: Option<String>,
}

/// 4.3 Artifact — typed, stateful, versioned node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub project_id: String,
    pub artifact_type: ArtifactType,
    pub title: String,
    pub current_version_id: String,
    pub state: ArtifactState,
    pub validation_summary: ValidationSummary,
    pub latest_approval_id: Option<String>,
    pub stale_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 4.4 ArtifactVersion — specific content snapshot + constitutional binding
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersion {
    pub id: String,
    pub artifact_id: String,
    pub project_id: String,
    pub version_number: u32,
    pub constitution_version_id: String,
    pub content: serde_json::Value,
    pub content_hash: String,
    pub parent_version_id: Option<String>,
    pub created_at: String,
    pub created_by: LocalIdentity,
}

/// 4.5 Approval — binds a specific version hash to a human sign-off
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub project_id: String,
    pub artifact_id: String,
    pub artifact_version_id: String,
    pub artifact_content_hash: String,
    pub approval_type: ApprovalType,
    pub approver: LocalIdentity,
    pub rationale: Option<String>,
    pub created_at: String,
}

/// 4.6 Amendment — formal transition, never a casual edit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Amendment {
    pub id: String,
    pub project_id: String,
    pub target_constitution_version_id: String,
    pub proposed_changes: ConstitutionPatch,
    pub reason: String,
    pub expected_impact_summary: String,
    pub invalidated_artifact_ids: Vec<String>,
    pub resulting_constitution_version_id: Option<String>,
    pub proposer: LocalIdentity,
    pub status: AmendmentStatus,
    pub created_at: String,
    pub updated_at: String,
    pub applied_at: Option<String>,
}

/// 4.7 TraceLink — explicit, typed, bidirectional graph link
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceLink {
    pub id: String,
    pub project_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub link_type: TraceLinkType,
    pub rationale: String,
    pub created_by: LocalIdentity,
    pub created_at: String,
}

/// 4.8 DriftAlarm — concrete violation with remediation path
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftAlarm {
    pub id: String,
    pub project_id: String,
    pub alarm_type: DriftAlarmType,
    pub severity: DriftAlarmSeverity,
    pub source_artifact_id: String,
    pub affected_node_ids: Vec<String>,
    pub violated_rule_id: String,
    pub rule_provenance: RuleProvenance,
    pub explanation: String,
    pub remediation_path: Vec<String>,
    pub status: DriftAlarmStatus,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

/// 4.9 ValidationResult — per-layer with evidence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub id: String,
    pub project_id: String,
    pub artifact_id: String,
    pub artifact_version_id: String,
    pub layer: ValidationLayer,
    pub status: ValidationStatus,
    pub rule_results: Vec<RuleResult>,
    pub validated_at: String,
    pub validated_by: ValidatedBy,
}

/// 4.10 ExecutionReadinessGate — computed, not freely edited
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReadinessGate {
    pub id: String,
    pub project_id: String,
    pub artifact_id: String,
    pub constitution_version_id: String,
    pub status: GateStatus,
    pub blocking_reasons: Vec<BlockingReason>,
    pub readiness_summary: String,
    pub export_manifest_preview: ExportManifestPreview,
    pub computed_at: String,
    pub approved_at: Option<String>,
    pub approved_by: Option<LocalIdentity>,
}

/// 4.11 AuditEvent — observable project history
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: String,
    pub project_id: String,
    pub event_type: AuditEventType,
    pub actor: AuditActor,
    pub occurred_at: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuditActor {
    User(LocalIdentity),
    System,
}

// ─── Artifact Content Shapes ────────────────────────────────

/// 6.1 ConstitutionContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionContent {
    pub one_sentence_promise: String,
    pub user_fantasy: String,
    pub non_negotiable_outcomes: Vec<String>,
    pub anti_goals: Vec<String>,
    pub quality_bar: String,
    pub failure_condition: String,
}

/// 6.2 UserFantasyWorkflowContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFantasyWorkflowContent {
    pub fantasy_narrative: String,
    pub core_workflows: Vec<WorkflowDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub trigger: String,
    pub steps: Vec<String>,
    pub intended_outcome: String,
    pub linked_constitution_clauses: Vec<String>,
}

/// 6.3 FeatureMapContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureMapContent {
    pub features: Vec<FeatureDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub linked_workflow_ids: Vec<String>,
    pub linked_constitution_clauses: Vec<String>,
    pub anti_goal_conflicts: Vec<String>,
}

/// 6.4 SystemArchitectureContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemArchitectureContent {
    pub systems: Vec<SystemDefinition>,
    pub boundaries: Vec<BoundaryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDefinition {
    pub id: String,
    pub name: String,
    pub responsibility: String,
    pub linked_feature_ids: Vec<String>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryDefinition {
    pub id: String,
    pub source_system_id: String,
    pub target_system_id: String,
    pub interaction: String,
    pub invariants: Vec<String>,
}

/// 6.5 UxStateMapContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UxStateMapContent {
    pub states: Vec<UxStateDefinition>,
    pub transitions: Vec<UxTransitionDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UxStateDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entry_conditions: Vec<String>,
    pub blocked_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UxTransitionDefinition {
    pub id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub trigger: String,
    pub validation_conditions: Vec<String>,
}

/// 6.6 PhaseRoadmapContractsContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseRoadmapContractsContent {
    pub phases: Vec<PhaseContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseContract {
    pub id: String,
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub invariants: Vec<String>,
    pub forbidden_compromises: Vec<String>,
    pub drift_risks: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub linked_artifact_types: Vec<ArtifactType>,
}

/// 6.7 AcceptanceChecklistsContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceChecklistsContent {
    pub checklist_groups: Vec<AcceptanceChecklistGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceChecklistGroup {
    pub id: String,
    pub name: String,
    pub linked_phase_id: String,
    pub items: Vec<AcceptanceChecklistItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceChecklistItem {
    pub id: String,
    pub question: String,
    pub response: ChecklistResponse,
    pub rationale: Option<String>,
    pub linked_constitution_clauses: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistResponse {
    Pass,
    Fail,
    NeedsAmendment,
    Pending,
}

/// 6.8 DriftAlarmDefinitionsContent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftAlarmDefinitionsContent {
    pub definitions: Vec<DriftAlarmDefinitionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftAlarmDefinitionEntry {
    pub id: String,
    pub alarm_type: DriftAlarmType,
    pub description: String,
    pub trigger_conditions: Vec<String>,
    pub default_severity: DriftAlarmSeverity,
    pub remediation_template: Vec<String>,
}

/// 6.9 ExecutionReadinessGateContent — computed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReadinessGateContent {
    pub computed: bool, // always true
    pub status: GateStatus,
    pub blocking_reasons: Vec<BlockingReason>,
    pub readiness_summary: String,
    pub export_manifest_preview: ExportManifestPreview,
}

// ─── Rule Engine Contract ───────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRule {
    pub id: String,
    pub name: String,
    pub layer: ValidationLayer,
    pub description: String,
    pub evaluate: String, // "backend_only"
    pub provenance: RuleProvenance,
}

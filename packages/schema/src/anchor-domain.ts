/**
 * Anchor Canonical Schema Pack v1
 *
 * Source of truth for the TypeScript frontend, Rust backend,
 * validation engine, audit model, and export compiler.
 *
 * No additional first-class planning artifact types may be
 * introduced without a schema version upgrade.
 */

export const SCHEMA_VERSION = '1.0.0';

// ─── 3. Canonical Enums ──────────────────────────────────────

export type ArtifactType =
  | 'constitution'
  | 'user_fantasy_workflows'
  | 'feature_map'
  | 'system_architecture'
  | 'ux_state_map'
  | 'phase_roadmap_contracts'
  | 'acceptance_checklists'
  | 'drift_alarm_definitions'
  | 'execution_readiness_gate';

export const ARTIFACT_TYPES: readonly ArtifactType[] = [
  'constitution',
  'user_fantasy_workflows',
  'feature_map',
  'system_architecture',
  'ux_state_map',
  'phase_roadmap_contracts',
  'acceptance_checklists',
  'drift_alarm_definitions',
  'execution_readiness_gate',
] as const;

export type ArtifactState =
  | 'draft'
  | 'complete'
  | 'valid'
  | 'approved'
  | 'stale';

export type ValidationLayer =
  | 'structural'
  | 'relational'
  | 'intent';

export type ValidationStatus =
  | 'pending'
  | 'pass'
  | 'fail'
  | 'needs_amendment';

export type DriftAlarmType =
  | 'traceability_drift'
  | 'constitution_drift'
  | 'sequence_drift'
  | 'quality_drift'
  | 'scope_drift';

export type DriftAlarmSeverity =
  | 'info'
  | 'warning'
  | 'error'
  | 'blocking';

export type TraceLinkType =
  | 'justifies'
  | 'derives_from'
  | 'implements'
  | 'depends_on'
  | 'validated_by'
  | 'invalidated_by';

export type ApprovalType =
  | 'standard'
  | 'reapproval'
  | 'gate_approval';

export type AmendmentStatus =
  | 'proposed'
  | 'impact_assessed'
  | 'applied'
  | 'reconciliation_pending'
  | 'completed'
  | 'abandoned';

export type GateStatus = 'blocked' | 'ready';

export type AuditEventType =
  | 'project_created'
  | 'constitution_locked'
  | 'artifact_created'
  | 'artifact_updated'
  | 'artifact_completed'
  | 'artifact_validated'
  | 'artifact_approved'
  | 'artifact_marked_stale'
  | 'trace_link_created'
  | 'trace_link_removed'
  | 'amendment_started'
  | 'amendment_impact_assessed'
  | 'amendment_applied'
  | 'drift_alarm_raised'
  | 'drift_alarm_resolved'
  | 'export_blocked'
  | 'readiness_gate_computed'
  | 'readiness_gate_passed'
  | 'project_exported';

export type DriftAlarmStatus =
  | 'active'
  | 'resolved'
  | 'dismissed_for_amendment_context';

// ─── 5. Supporting Types ─────────────────────────────────────

export interface LocalIdentity {
  id: string;
  displayName: string;
}

export interface ProjectSettings {
  exportFormat: 'json_and_markdown';
  optionalSqliteIndexing: boolean;
  requireIntentReviewRationaleOnFailure: boolean;
}

export interface ConstitutionPatch {
  oneSentencePromise?: string;
  userFantasy?: string;
  nonNegotiableOutcomes?: string[];
  antiGoals?: string[];
  qualityBar?: string;
  failureCondition?: string;
}

export interface ValidationSummary {
  structural: ValidationStatus;
  relational: ValidationStatus;
  intent: ValidationStatus;
  lastValidatedAt?: string;
}

export interface RuleProvenance {
  sourceArtifactType: ArtifactType | 'system_rule';
  sourceClause: string;
  humanLabel: string;
}

export interface RuleResult {
  ruleId: string;
  status: 'pass' | 'fail';
  message: string;
  ruleProvenance: RuleProvenance;
  affectedNodeIds: string[];
}

export interface BlockingReason {
  code: string;
  message: string;
  affectedArtifactIds: string[];
  ruleProvenance: RuleProvenance;
  remediationSteps: string[];
}

export interface ExportFileEntry {
  path: string;
  kind: 'json' | 'markdown' | 'report';
  derivedFromArtifactIds: string[];
}

export interface ExportManifestPreview {
  fileCount: number;
  files: ExportFileEntry[];
}

// ─── 4. Domain Entities ──────────────────────────────────────

/** 4.1 Project — root aggregate */
export interface Project {
  id: string;
  schemaVersion: string;
  name: string;
  slug: string;
  description: string;
  createdAt: string;
  updatedAt: string;
  createdBy: LocalIdentity;
  currentConstitutionVersionId: string;
  artifactIds: string[];
  activeAmendmentId?: string;
  settings: ProjectSettings;
}

/** 4.2 Constitution — the throne, versioned, changes only via Amendment Protocol */
export interface Constitution {
  id: string;
  artifactId: string;
  versionId: string;
  projectId: string;
  oneSentencePromise: string;
  userFantasy: string;
  nonNegotiableOutcomes: string[];
  antiGoals: string[];
  qualityBar: string;
  failureCondition: string;
  locked: boolean;
  contentHash: string;
  createdAt: string;
  updatedAt: string;
  approvedAt?: string;
  approvedBy?: LocalIdentity;
  parentVersionId?: string;
}

/** 4.3 Artifact — typed, stateful, versioned node */
export interface Artifact {
  id: string;
  projectId: string;
  type: ArtifactType;
  title: string;
  currentVersionId: string;
  state: ArtifactState;
  validationSummary: ValidationSummary;
  latestApprovalId?: string;
  staleReason?: string;
  createdAt: string;
  updatedAt: string;
}

/** 4.4 ArtifactVersion — specific content snapshot + constitutional binding */
export interface ArtifactVersion {
  id: string;
  artifactId: string;
  projectId: string;
  versionNumber: number;
  constitutionVersionId: string;
  content: ArtifactContent;
  contentHash: string;
  parentVersionId?: string;
  createdAt: string;
  createdBy: LocalIdentity;
}

/** 4.5 Approval — binds a specific version hash to a human sign-off */
export interface Approval {
  id: string;
  projectId: string;
  artifactId: string;
  artifactVersionId: string;
  artifactContentHash: string;
  approvalType: ApprovalType;
  approver: LocalIdentity;
  rationale?: string;
  createdAt: string;
}

/** 4.6 Amendment — formal transition, never a casual edit */
export interface Amendment {
  id: string;
  projectId: string;
  targetConstitutionVersionId: string;
  proposedChanges: ConstitutionPatch;
  reason: string;
  expectedImpactSummary: string;
  invalidatedArtifactIds: string[];
  resultingConstitutionVersionId?: string;
  proposer: LocalIdentity;
  status: AmendmentStatus;
  createdAt: string;
  updatedAt: string;
  appliedAt?: string;
}

/** 4.7 TraceLink — explicit, typed, bidirectional graph link */
export interface TraceLink {
  id: string;
  projectId: string;
  sourceNodeId: string;
  targetNodeId: string;
  linkType: TraceLinkType;
  rationale: string;
  createdBy: LocalIdentity;
  createdAt: string;
}

/** 4.8 DriftAlarm — concrete violation with remediation path */
export interface DriftAlarm {
  id: string;
  projectId: string;
  type: DriftAlarmType;
  severity: DriftAlarmSeverity;
  sourceArtifactId: string;
  affectedNodeIds: string[];
  violatedRuleId: string;
  ruleProvenance: RuleProvenance;
  explanation: string;
  remediationPath: string[];
  status: DriftAlarmStatus;
  createdAt: string;
  resolvedAt?: string;
}

/** 4.9 ValidationResult — per-layer with evidence */
export interface ValidationResult {
  id: string;
  projectId: string;
  artifactId: string;
  artifactVersionId: string;
  layer: ValidationLayer;
  status: ValidationStatus;
  ruleResults: RuleResult[];
  validatedAt: string;
  validatedBy: 'frontend' | 'backend' | 'human_review';
}

/** 4.10 ExecutionReadinessGate — computed, not freely edited */
export interface ExecutionReadinessGate {
  id: string;
  projectId: string;
  artifactId: string;
  constitutionVersionId: string;
  status: GateStatus;
  blockingReasons: BlockingReason[];
  readinessSummary: string;
  exportManifestPreview: ExportManifestPreview;
  computedAt: string;
  approvedAt?: string;
  approvedBy?: LocalIdentity;
}

/** 4.11 AuditEvent — observable project history */
export interface AuditEvent {
  id: string;
  projectId: string;
  type: AuditEventType;
  actor: LocalIdentity | 'system';
  occurredAt: string;
  payload: Record<string, unknown>;
}

// ─── 6. Artifact Content Shapes ──────────────────────────────

/** 6.1 constitution */
export interface ConstitutionContent {
  oneSentencePromise: string;
  userFantasy: string;
  nonNegotiableOutcomes: string[];
  antiGoals: string[];
  qualityBar: string;
  failureCondition: string;
}

/** 6.2 user_fantasy_workflows */
export interface UserFantasyWorkflowContent {
  fantasyNarrative: string;
  coreWorkflows: WorkflowDefinition[];
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  trigger: string;
  steps: string[];
  intendedOutcome: string;
  linkedConstitutionClauses: string[];
}

/** 6.3 feature_map */
export interface FeatureMapContent {
  features: FeatureDefinition[];
}

export interface FeatureDefinition {
  id: string;
  name: string;
  description: string;
  linkedWorkflowIds: string[];
  linkedConstitutionClauses: string[];
  antiGoalConflicts: string[];
}

/** 6.4 system_architecture */
export interface SystemArchitectureContent {
  systems: SystemDefinition[];
  boundaries: BoundaryDefinition[];
}

export interface SystemDefinition {
  id: string;
  name: string;
  responsibility: string;
  linkedFeatureIds: string[];
  constraints: string[];
}

export interface BoundaryDefinition {
  id: string;
  sourceSystemId: string;
  targetSystemId: string;
  interaction: string;
  invariants: string[];
}

/** 6.5 ux_state_map */
export interface UxStateMapContent {
  states: UxStateDefinition[];
  transitions: UxTransitionDefinition[];
}

export interface UxStateDefinition {
  id: string;
  name: string;
  description: string;
  entryConditions: string[];
  blockedActions: string[];
}

export interface UxTransitionDefinition {
  id: string;
  fromStateId: string;
  toStateId: string;
  trigger: string;
  validationConditions: string[];
}

/** 6.6 phase_roadmap_contracts */
export interface PhaseRoadmapContractsContent {
  phases: PhaseContract[];
}

export interface PhaseContract {
  id: string;
  name: string;
  inputs: string[];
  outputs: string[];
  invariants: string[];
  forbiddenCompromises: string[];
  driftRisks: string[];
  acceptanceCriteria: string[];
  linkedArtifactTypes: ArtifactType[];
}

/** 6.7 acceptance_checklists */
export interface AcceptanceChecklistsContent {
  checklistGroups: AcceptanceChecklistGroup[];
}

export interface AcceptanceChecklistGroup {
  id: string;
  name: string;
  linkedPhaseId: string;
  items: AcceptanceChecklistItem[];
}

export interface AcceptanceChecklistItem {
  id: string;
  question: string;
  response: 'pass' | 'fail' | 'needs_amendment' | 'pending';
  rationale?: string;
  linkedConstitutionClauses: string[];
}

/** 6.8 drift_alarm_definitions */
export interface DriftAlarmDefinitionsContent {
  definitions: DriftAlarmDefinition[];
}

export interface DriftAlarmDefinition {
  id: string;
  type: DriftAlarmType;
  description: string;
  triggerConditions: string[];
  defaultSeverity: DriftAlarmSeverity;
  remediationTemplate: string[];
}

/** 6.9 execution_readiness_gate — computed content */
export interface ExecutionReadinessGateContent {
  computed: true;
  status: GateStatus;
  blockingReasons: BlockingReason[];
  readinessSummary: string;
  exportManifestPreview: ExportManifestPreview;
}

/**
 * Discriminated union of all artifact content types.
 * ArtifactVersion.content is typed as this union.
 */
export type ArtifactContent =
  | ConstitutionContent
  | UserFantasyWorkflowContent
  | FeatureMapContent
  | SystemArchitectureContent
  | UxStateMapContent
  | PhaseRoadmapContractsContent
  | AcceptanceChecklistsContent
  | DriftAlarmDefinitionsContent
  | ExecutionReadinessGateContent;

// ─── 12. Rule Engine Contract ────────────────────────────────

export interface ValidationRule {
  id: string;
  name: string;
  layer: ValidationLayer;
  description: string;
  evaluate: 'backend_only';
  provenance: RuleProvenance;
}

/** TypeScript interfaces matching Rust serde output. */

export interface Project {
  id: string;
  schemaVersion: string;
  name: string;
  slug: string;
  description: string;
  createdAt: string;
  updatedAt: string;
}

export interface ArtifactRow {
  id: string;
  artifactType: string;
  title: string;
  state: string;
  versionNumber: number;
  hasApproval: boolean;
  upstreamCount: number;
  downstreamCount: number;
  alarmCount: number;
  updatedAt: string;
}

export interface ProjectSnapshot {
  project: Project;
  artifacts: ArtifactRow[];
  gateStatus: string;
  activeAlarmCount: number;
  staleCount: number;
}

export interface TraceLinkRow {
  id: string;
  sourceId: string;
  sourceTitle: string;
  targetId: string;
  targetTitle: string;
  linkType: string;
  rationale: string;
}

export interface ArtifactDetailResponse {
  artifact: {
    id: string;
    artifactType: string;
    title: string;
    state: string;
    currentVersionId: string;
    validationSummary: {
      structural: string;
      relational: string;
      intent: string;
      lastValidatedAt: string | null;
    };
    staleReason: string | null;
    createdAt: string;
    updatedAt: string;
  };
  version: {
    id: string;
    versionNumber: number;
    constitutionVersionId: string;
    contentHash: string;
    content: unknown;
  } | null;
  approval: {
    id: string;
    approver: { displayName: string };
    createdAt: string;
    approvalType: string;
  } | null;
  outgoingLinks: TraceLinkRow[];
  incomingLinks: TraceLinkRow[];
  activeAlarms: DriftAlarm[];
  legalTransitions: string[];
}

export interface DriftAlarm {
  id: string;
  alarmType: string;
  severity: string;
  sourceArtifactId: string;
  explanation: string;
  remediationPath: string[];
  status: string;
}

export interface BlockingReason {
  code: string;
  message: string;
  affectedArtifactIds: string[];
  ruleProvenance: {
    sourceClause: string;
    humanLabel: string;
  };
  remediationSteps: string[];
}

export interface GateEvaluation {
  status: string;
  blockingReasons: BlockingReason[];
  staleSummary: { count: number; artifactIds: string[] };
  outdatedApprovals: {
    approvalId: string;
    artifactId: string;
    approvedAgainstVersion: string;
    currentConstitutionVersion: string;
  }[];
  activeBlockingAlarms: {
    alarmId: string;
    alarmType: string;
    severity: string;
    sourceArtifactId: string;
    explanation: string;
  }[];
  traceabilityFailures: number;
  readinessSummary: string;
  exportManifestPreview: {
    fileCount: number;
    files: { path: string; kind: string }[];
  };
}

export interface ExportPreviewResponse {
  ready: boolean;
  files: { path: string; sizeBytes: number; contentPreview: string }[];
  blockedReason: string | null;
  blockingReasons: BlockingReason[];
}

export interface TransitionResponse {
  success: boolean;
  newState: string | null;
  error: string | null;
}

// ── Step 10: Operational Governance Types ─────────────────

export interface EditResponse {
  success: boolean;
  newVersionNumber: number | null;
  newState: string | null;
  staleArtifactIds: string[];
  error: string | null;
}

export interface AmendmentResponse {
  success: boolean;
  amendmentId: string | null;
  status: string | null;
  affectedArtifactIds: string[];
  impactSummary: string | null;
  error: string | null;
}

export interface AuditTimelineResponse {
  events: AuditEventRow[];
  totalCount: number;
}

export interface AuditEventRow {
  id: string;
  eventType: string;
  occurredAt: string;
  actorName: string;
  summary: string;
}

export interface SaveLoadResponse {
  success: boolean;
  filePath: string | null;
  error: string | null;
}

// ── Step 11: Explainability & Recovery Types ─────────────

export interface ValidationReport {
  artifactId: string;
  artifactType: string;
  artifactState: string;
  overallStatus: 'all_clear' | 'has_warnings' | 'blocked';
  checks: ValidationCheck[];
  resolutionSummary: string | null;
}

export interface ValidationCheck {
  checkId: string;
  layer: string;
  status: 'pass' | 'fail' | 'warning' | 'not_applicable';
  title: string;
  explanation: string;
  resolutionSteps: string[];
  affectedArtifactIds: string[];
  ruleClause: string;
}

export interface VersionDiff {
  artifactId: string;
  fromVersion: VersionSummary;
  toVersion: VersionSummary;
  contentChanges: ContentChange[];
  metadataChanges: MetadataChange[];
  approvalImpact: ApprovalImpact;
}

export interface VersionSummary {
  versionId: string;
  versionNumber: number;
  contentHash: string;
  constitutionVersionId: string;
  createdAt: string;
}

export interface ContentChange {
  fieldPath: string;
  changeType: 'added' | 'removed' | 'modified';
  oldValue: string | null;
  newValue: string | null;
}

export interface MetadataChange {
  field: string;
  oldValue: string;
  newValue: string;
}

export interface ApprovalImpact {
  approvalInvalidated: boolean;
  reason: string | null;
  approvalWasForVersion: string | null;
  changesSinceApproval: string[];
}

export interface ImpactReport {
  trigger: ImpactTrigger;
  affectedArtifacts: AffectedArtifact[];
  invalidatedApprovals: InvalidatedApproval[];
  totalAffected: number;
  totalApprovalsLost: number;
  severity: 'none' | 'low' | 'medium' | 'high' | 'nuclear';
  recoveryPlan: RecoveryStep[];
}

export interface ImpactTrigger {
  kind: 'edit' | 'amendment' | 'stale_propagate';
  sourceArtifactId: string;
  sourceArtifactTitle: string;
  description: string;
}

export interface AffectedArtifact {
  artifactId: string;
  title: string;
  artifactType: string;
  currentState: string;
  willBecome: string;
  distanceFromSource: number;
  propagationPath: string[];
  reason: string;
}

export interface InvalidatedApproval {
  approvalId: string;
  artifactId: string;
  artifactTitle: string;
  reason: string;
}

export interface RecoveryStep {
  order: number;
  artifactId: string;
  action: string;
  reason: string;
}

export interface ImportDiagnostic {
  loadable: boolean;
  summary: ImportSummary | null;
  issues: ImportIssue[];
  repairable: boolean;
  repairDescription: string | null;
}

export interface ImportSummary {
  projectName: string;
  fileVersion: string;
  schemaVersion: string;
  artifactCount: number;
  versionCount: number;
  linkCount: number;
  amendmentCount: number;
  auditEventCount: number;
}

export interface ImportIssue {
  severity: 'warning' | 'error' | 'fatal';
  code: string;
  message: string;
  detail: string | null;
}

export interface ImportWithRepairResponse {
  success: boolean;
  filePath: string | null;
  issues: ImportIssue[];
  error: string | null;
}

export interface SwitchScenarioResponse {
  success: boolean;
  scenarioName: string;
  projectName: string;
  artifactCount: number;
  error: string | null;
}

export interface ScenarioInfo {
  id: string;
  name: string;
  description: string;
  flavor: string;
}

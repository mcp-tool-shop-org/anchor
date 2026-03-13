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

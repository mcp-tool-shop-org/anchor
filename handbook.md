# Anchor Handbook

## What Is Anchor

Anchor is a local-first desktop app that forces constitution-first, fully traceable project design so creative products cannot drift from their intended purpose before execution begins.

It is not a planning app. It is a drift-prevention engine for serious creative software design.

## The Problem

Phase-by-phase development has a hidden failure mode: each checkpoint becomes a tiny regime change. The original product thesis gets nibbled to death by "reasonable" adjustments until the project still looks organized but no longer does what it was born to do.

Completing steps in ways that no longer agree with each other is the sneakier failure. Anchor enforces **coherence**, not just completion.

## Product Constitution

**One-sentence promise:** Anchor forces constitution-first, fully traceable project design so creative products cannot drift from their intended purpose before execution begins.

**User fantasy:** You open Anchor with a serious creative project, and it guides you through a disciplined sequence that produces a complete, coherent, handover-ready plan where every decision is justified, traceable, and locked to the original promise.

**Non-negotiable outcomes:** Every project must reach the Execution Readiness Gate with all artifacts coherent, approved against the current Constitution version, zero active drift alarms, and full bidirectional traceability. Export only happens after that gate clears.

**Anti-goals:**
- No cloud dependency
- No optional steps
- No AI-generated fluff
- No "quick start" bypasses
- No free-floating opinions masquerading as process

**Quality bar:** The app itself must be as deterministic and observable as the projects it creates. Every change instantly surfaces coherence violations.

**Failure condition:** If a user can fill every field, mark everything "complete," and export while the artifacts no longer agree with each other or the Constitution — that is total failure, even if the UI is beautiful.

## Stack

- **Frontend:** React + TypeScript (window into law, not source of truth)
- **Backend:** Tauri / Rust (final authority for validation, hashing, state transitions, export)
- **Storage:** Local JSON files + optional SQLite
- **Network:** None except optional update check

## Artifact Spine

Every project contains exactly nine artifacts, worked in order:

| # | Artifact Type | Purpose |
|---|---|---|
| 1 | Constitution | The throne — one-sentence promise, user fantasy, anti-goals, quality bar, failure condition |
| 2 | User Fantasy + Core Workflows | Narrative + concrete workflow definitions with constitution clause links |
| 3 | Feature Map | Features with upstream workflow justification and anti-goal conflict checks |
| 4 | System Architecture | Systems, responsibilities, boundaries, feature implementation links |
| 5 | UX State Map | States, transitions, entry conditions, blocked actions |
| 6 | Phase Roadmap + Contracts | Phases with inputs, outputs, invariants, forbidden compromises, drift risks |
| 7 | Acceptance Checklists | Per-phase checklist groups with constitution-linked items |
| 8 | Drift Alarm Definitions | Alarm types, trigger conditions, severity, remediation templates |
| 9 | Execution Readiness Gate | Computed (not authored) — the final bridge from design to implementation |

No additional first-class artifact types may be introduced without a schema version upgrade.

## Artifact State Machine

Each artifact has exactly five states:

```
Draft → Complete → Valid → Approved → (upstream change) → Stale
```

- **Draft:** exists but missing required fields
- **Complete:** all required fields present
- **Valid:** passes structural, relational, and intent validation
- **Approved:** human has explicitly signed off (bound to specific content hash + timestamp)
- **Stale:** upstream change invalidated the approval

### Forbidden Transitions

- Draft → Approved (must complete and validate first)
- Draft → Valid (must complete first)
- Complete → Approved (must validate first)
- Approved → Draft (must go through Stale if upstream changed)

### Validation Layers

Each artifact is validated at three levels:

1. **Structural:** required fields, enum values, version existence, hash integrity
2. **Relational:** bidirectional traceability links exist, endpoints exist, no orphan nodes, constitution version alignment
3. **Intent:** human review — does this actually support the Constitution and raise creator output quality?

## Traceability

Every meaningful node must have bidirectional traceability to its justification.

### Required Relationships

- Workflows → trace to constitution clauses via `derives_from`
- Features → trace to workflows via `justifies` or `derives_from`
- Systems → trace to features via `implements`
- UX states → trace to workflows/features via `depends_on`
- Phases → trace to artifacts via `validated_by`
- Drift alarms → trace to violated nodes via `invalidated_by`

### Link Types (constrained)

`justifies` · `derives_from` · `implements` · `depends_on` · `validated_by` · `invalidated_by`

Any node that cannot answer both "what justifies this?" and "what depends on this?" fails relational validation.

## Amendment Protocol

The Constitution is lockable, not sacred. Change is allowed but must be visible and formal.

To amend:
1. State the exact reason
2. List expected downstream impact
3. App auto-generates the list of invalidated/stale artifacts
4. All affected artifacts must be explicitly re-approved after reconciliation
5. Export remains blocked until every downstream artifact is coherent with the new Constitution version

Every amendment is recorded with content hash, timestamp, and local identity. Amendment history is immutable once applied.

## Drift Alarm Taxonomy

| Type | Trigger |
|---|---|
| **Traceability drift** | Item has no valid upstream justification |
| **Constitution drift** | Conflicts with promise, anti-goals, or quality bar |
| **Sequence drift** | Downstream approved while upstream changed |
| **Quality drift** | Decision weakens creator output quality |
| **Scope drift** | New surface area without explicit authorization |

Every alarm includes: violated rule ID, rule provenance (which clause produced it), human-readable explanation, and remediation path.

## Execution Readiness Gate

The gate is computed, not authored. It is the final judge.

**Blocked if any of:**
- Any non-gate artifact is not Approved
- Any artifact is Stale
- Any blocking/error-severity drift alarm is active
- Any amendment is not completed
- Any approval binds to an outdated constitution version

**Outputs:** gate status, blocking reasons, readiness summary, export manifest preview.

## Why Blocked?

Every blocked action in the app surfaces:
- The exact violated rule
- The upstream artifact involved
- A user-readable explanation
- The exact remediation steps
- Rule provenance (which constitutional clause or contract produced the rule)

No mystery red dots.

## Export Package

When the gate clears, export produces:

```
/project.json                              — canonical machine-readable source
/constitution.md                           — constitution in markdown
/artifacts/user-fantasy-workflows.md
/artifacts/feature-map.md
/artifacts/system-architecture.md
/artifacts/ux-state-map.md
/artifacts/phase-roadmap-contracts.md
/artifacts/acceptance-checklists.md
/artifacts/drift-alarm-definitions.md
/reports/traceability-matrix.md
/reports/audit-log.md
/reports/drift-report.md
/reports/execution-readiness-report.md
```

Export is blocked unless the gate status is `ready`.

## Review Mode

Reviews are constrained, not freeform:
- **Pass**
- **Fail** (requires short typed rationale)
- **Needs Amendment** (requires short typed rationale)

## UX Principles

- Strict progression for approval/export
- Broad visibility — all future artifacts are readable (read-ahead allowed)
- Editing blocked where not yet unlocked for approval
- Downstream previews visible so users understand why the current step matters
- "Why blocked?" panel on every blocked action

The UI is a window into the law. The Rust backend is the law.

## Domain Entities

Eleven top-level entities: Project, Constitution, Artifact, ArtifactVersion, Approval, Amendment, TraceLink, DriftAlarm, ValidationResult, ExecutionReadinessGate, AuditEvent.

Full type definitions: [`packages/schema/src/anchor-domain.ts`](packages/schema/src/anchor-domain.ts)

## Build Sequence

1. ✅ Canonical TypeScript types — `packages/schema/src/anchor-domain.ts`
2. ✅ Rust domain structs — `src-tauri/src/domain.rs`
3. ✅ Artifact lifecycle state machine — `src-tauri/src/state_machine.rs`
4. Traceability graph model
5. Drift alarm rule engine
6. Stale propagation logic
7. Execution readiness gate evaluator
8. Export compiler
9. Full UI shell wired to backend law

## Audit Events

Every meaningful state change is recorded:

`project_created` · `constitution_locked` · `artifact_created` · `artifact_updated` · `artifact_completed` · `artifact_validated` · `artifact_approved` · `artifact_marked_stale` · `trace_link_created` · `trace_link_removed` · `amendment_started` · `amendment_impact_assessed` · `amendment_applied` · `drift_alarm_raised` · `drift_alarm_resolved` · `export_blocked` · `readiness_gate_computed` · `readiness_gate_passed` · `project_exported`

## Final Constraint

Anchor is not a planning scrapbook. If the software ever allows users to complete ceremony without preserving coherence, it has failed its purpose.

# Anchor Handbook

> The complete specification for Anchor's design philosophy, domain model, and enforcement architecture.

---

## Table of Contents

- [What Is Anchor](#what-is-anchor)
- [The Problem](#the-problem)
- [Product Constitution](#product-constitution)
- [Stack](#stack)
- [Artifact Spine](#artifact-spine)
- [Artifact State Machine](#artifact-state-machine)
- [Traceability](#traceability)
- [Amendment Protocol](#amendment-protocol)
- [Drift Alarm Taxonomy](#drift-alarm-taxonomy)
- [Execution Readiness Gate](#execution-readiness-gate)
- [Why Blocked?](#why-blocked)
- [Export Package](#export-package)
- [Integrity Attestation](#integrity-attestation)
- [Recovery Engine](#recovery-engine)
- [Link Authoring](#link-authoring)
- [Validation Reports](#validation-reports)
- [Version Diffing](#version-diffing)
- [Impact Analysis](#impact-analysis)
- [Persistence](#persistence)
- [Review Mode](#review-mode)
- [UX Principles](#ux-principles)
- [Domain Entities](#domain-entities)
- [Backend Law Engine](#backend-law-engine)
- [UI Shell](#ui-shell)
- [Demo Scenarios](#demo-scenarios)
- [Build Sequence](#build-sequence)
- [Audit Events](#audit-events)
- [Design Decisions](#design-decisions)
- [Invariants](#invariants)

---

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

| Layer | Technology | Role |
|-------|-----------|------|
| Backend | Rust / Tauri 2 | Final authority — validation, hashing, state transitions, export |
| Frontend | React 19 + TypeScript | Window into law — forms, graph visualization, "Why blocked?" |
| Storage | Local JSON + optional SQLite | No cloud dependency |
| Network | None | Optional update check only |

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

> Source: [`src-tauri/src/state_machine.rs`](src-tauri/src/state_machine.rs) (731 LOC, 12 tests)

Each artifact has exactly five states:

| State | Meaning |
|-------|---------|
| **Draft** | Exists but missing required fields |
| **Complete** | All required fields present |
| **Valid** | Passes structural, relational, and intent validation |
| **Approved** | Human has explicitly signed off (bound to content hash + timestamp) |
| **Stale** | Upstream change invalidated the approval |

### Full Transition Diagram

```
                ┌──────────────────────────────┐
                │         ┌──────┐             │
                │    ┌───→│ Valid │────┐        │
                │    │    └──┬───┘    │        │
                ▼    │       │        ▼        │
┌───────┐   ┌──┴────┴──┐    │   ┌──────────┐  │
│ Draft │──→│ Complete  │    │   │ Approved │  │
└───────┘   └───────────┘    │   └────┬─────┘  │
    ▲            ▲           │        │        │
    │            │           │        ▼        │
    │            │           │   ┌─────────┐   │
    │            └───────────┼───┤  Stale  │───┘
    │                        │   └─────────┘
    │                        │        │
    └────────────────────────┘        │
              (edit removes fields)   │
                                      ▼
                              (reconcile & re-approve)
```

### Forward Transitions

```
Draft → Complete → Valid → Approved
```

### Regression and Recovery Edges

The lifecycle is not a one-way escalator. Artifacts can regress:

| Edge | Trigger |
|------|---------|
| Complete → Draft | Content edited, required fields removed |
| Valid → Complete | Upstream content changed, revalidation needed |
| Approved → Stale | Upstream change invalidated this artifact |
| Stale → Complete | User reconciles content after upstream change |
| Stale → Valid | Content reconciled + passes all three validation layers |
| Stale → Approved | **Guarded** — requires successful revalidation AND explicit reapproval |

These back-edges are not escape hatches. They are the immune system. Without them, the only response to upstream change would be deleting work and starting over.

### Forbidden Transitions

These are structurally illegal regardless of context:

| Forbidden | Why |
|-----------|-----|
| Draft → Approved | Must complete and validate first |
| Draft → Valid | Must complete first |
| Complete → Approved | Must validate first |
| Approved → Draft | Must go through Stale if upstream changed |
| Same → Same | No-op transitions are not legal events |

### Validation Layers

Each artifact is validated at three levels:

1. **Structural** — required fields, enum values, version existence, hash integrity
2. **Relational** — bidirectional traceability links exist, endpoints exist, no orphan nodes, constitution version alignment
3. **Intent** — human review: does this actually support the Constitution and raise creator output quality?

## Traceability

> Source: [`src-tauri/src/traceability.rs`](src-tauri/src/traceability.rs) (515 LOC, 9 tests)

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

> Source: [`src-tauri/src/drift_rules.rs`](src-tauri/src/drift_rules.rs) (537 LOC, 10 tests)

| Type | Trigger |
|---|---|
| **Traceability drift** | Item has no valid upstream justification |
| **Constitution drift** | Conflicts with promise, anti-goals, or quality bar |
| **Sequence drift** | Downstream approved while upstream changed |
| **Quality drift** | Decision weakens creator output quality |
| **Scope drift** | New surface area without explicit authorization |

Every alarm includes: violated rule ID, rule provenance (which clause produced it), human-readable explanation, and remediation path.

## Execution Readiness Gate

> Source: [`src-tauri/src/readiness_gate.rs`](src-tauri/src/readiness_gate.rs) (731 LOC, 7 tests)

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

When the gate clears, export produces 14 files:

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
/reports/integrity-attestation.md           — hash chain proof of gate-guarded export
```

Export is blocked unless the gate status is `ready`.

## Integrity Attestation

The export package includes a cryptographic-style attestation file (`reports/integrity-attestation.md`) that proves the export was gate-guarded. It contains:

- **Gate Verdict** — status, blocking reason count, stale/traceability/approval failure counts
- **Artifact Attestation** — per-artifact table with type, state, version, content hash, approval status
- **Traceability Attestation** — total trace links, total approvals, constitution version
- **Package Integrity** — schema version, djb2 content hash chain (deterministic hash of all artifact content hashes), artifact count, file count

This file is generated by the export compiler and cannot be faked — it requires the gate to pass.

## Recovery Engine

> Source: [`src-tauri/src/recovery.rs`](src-tauri/src/recovery.rs) — 532 LOC, 10 tests

For any artifact or project state, the recovery engine computes:
- What's blocking progress
- What's the next lawful action
- What's the minimal path to gate-ready

### Per-Artifact Recovery

`next_actions_for_artifact()` inspects an artifact's state and returns prioritized actions:

| State | Actions Generated |
|-------|------------------|
| **Stale** | ReconcileStale (priority 1) — review against current constitution, update, revalidate, reapprove |
| **Draft** | EditContent (if no content), AddTraceLink (if missing), TransitionState → Complete |
| **Complete** | AddTraceLink (if missing), TransitionState → Valid |
| **Valid** | Reapprove (if no approval), TransitionState → Approved |
| **Approved** | Revalidate (if active drift alarms) |

### Trust Surfaces

Every recovery action carries two provenance fields:
- `rule_clause` — the specific constitutional section that requires this action (e.g. "§13.3 — No stale artifact may be present at export time")
- `why_first` — why this action is prioritized above others (e.g. "Stale artifacts block the readiness gate. Until reconciled, no export is possible.")

This makes every recommendation explainable. The operator never has to guess why the system says to do something.

### Project Health

`project_health()` aggregates per-artifact actions into a project-wide assessment:
- `HealthStatus`: Healthy (gate ready), NeedsAttention (issues but no blockers), Critical (stale or alarms)
- Sorted action list across all artifacts
- Summary counts: total, ready, stale, blocked, alarms, missing links

## Link Authoring

> Source: [`src-tauri/src/link_authoring.rs`](src-tauri/src/link_authoring.rs) — 528 LOC, 10 tests

Trace link creation with legality checks:
- Validates source and target artifacts exist
- Prevents duplicate links (same source + target + type)
- Prevents self-links
- `get_allowed_links()` — for any artifact, returns which link types and targets are legal
- `get_missing_links()` — compares required links (per §8.1) against existing links, returns gaps
- `check_removal_impact()` — before removing a link, shows which artifacts would become orphaned

## Validation Reports

> Source: [`src-tauri/src/validation.rs`](src-tauri/src/validation.rs) — 721 LOC, 10 tests

Per-artifact deep validation producing a `ValidationReport` with:
- 9 individual checks across 3 layers (structural, relational, intent)
- Each check has: status (pass/fail/warning/n-a), title, explanation, resolution steps, rule clause
- Overall verdict: AllClear, HasWarnings, or Blocked
- Resolution summary for actionable next steps

Checks include: content exists, content not placeholder, version exists, upstream traceability, no broken links, constitution alignment, not stale, approval current, downstream health.

## Version Diffing

> Source: [`src-tauri/src/diff.rs`](src-tauri/src/diff.rs) — 486 LOC, 6 tests

Computes differences between artifact versions:
- Content changes (field-level JSON diff)
- Metadata changes (constitution version, timestamps)
- Approval impact (was the approval invalidated by this change?)

## Impact Analysis

> Source: [`src-tauri/src/impact.rs`](src-tauri/src/impact.rs) — 456 LOC, 6 tests

Before committing a change, compute the full blast radius:
- Which downstream artifacts will become stale
- Which approvals will be invalidated
- Severity assessment (None/Low/Medium/High/Nuclear)
- Recovery plan with ordered steps

## Persistence

> Source: [`src-tauri/src/persistence.rs`](src-tauri/src/persistence.rs) — 795 LOC, 7 tests

Atomic save/load with integrity checking:

### File Format

```json
{
  "anchorFileVersion": "1.0.0",
  "schemaVersion": "1.0.0",
  "contentHash": "<djb2 hex digest of payload>",
  "payload": { /* full project state */ }
}
```

- Content hash computed via deterministic djb2 hash of canonical JSON payload
- Atomic writes via temp file + rename
- Version gates: rejects files from future versions

### Corruption Detection

`dry_run_load()` analyzes a file without loading it:
- Version compatibility check
- Schema version check  
- Integrity hash verification
- Data consistency checks (orphan versions, broken link endpoints, orphan approvals, missing artifact types)
- Reports: loadable (yes/no), repairable (yes/no), full issue list with severity

### Auto-Repair

`load_project_with_repair()` handles files with corrupted hashes:
- If the only error is a hash mismatch (e.g. file was edited externally), recomputes the hash and loads anyway
- Reports all repaired issues as warnings
- Version/schema mismatches are still fatal — no silent upgrades

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

> Source: [`src-tauri/src/domain.rs`](src-tauri/src/domain.rs) (684 LOC) — Rust structs
> Source: [`packages/schema/src/anchor-domain.ts`](packages/schema/src/anchor-domain.ts) — Canonical TypeScript types

Eleven top-level entities:

| Entity | Role |
|--------|------|
| `Project` | Root container — name, description, constitution ID, created/updated timestamps |
| `Constitution` | The throne — promise, fantasy, outcomes, anti-goals, quality bar, failure condition, version, locked flag |
| `Artifact` | Type-tagged container with state, version, content hash, constitution version binding |
| `ArtifactVersion` | Immutable snapshot — version number, content hash, timestamp |
| `Approval` | Human sign-off bound to specific content hash, constitution version, timestamp, approver |
| `Amendment` | Constitutional change record — reason, diff, impact assessment, status |
| `TraceLink` | Typed edge (6 types) between nodes with optional rationale |
| `DriftAlarm` | Active violation — rule ID, severity, affected nodes, provenance, remediation |
| `ValidationResult` | Per-artifact validation output from all three layers |
| `ExecutionReadinessGate` | Computed gate status with blocking reasons and manifest |
| `AuditEvent` | Immutable record of every meaningful state change |

## Backend Law Engine

The Rust backend is the final authority. These four modules form the enforcement core:

### Traceability Graph

> [`src-tauri/src/traceability.rs`](src-tauri/src/traceability.rs) — 515 LOC, 9 tests

Owns node/link integrity. For every meaningful node, answers:
- "What justifies this?" (upstream query)
- "What depends on this?" (downstream query)

Validates that required trace links exist per artifact type, endpoints resolve to real nodes, and no orphan features/systems/phases/workflows exist. Bidirectional — every link is navigable in both directions.

### Drift Rule Engine

> [`src-tauri/src/drift_rules.rs`](src-tauri/src/drift_rules.rs) — 537 LOC, 10 tests

Turns the drift alarm taxonomy into executable rules. Each rule produces:
- rule ID, provenance (which constitutional clause), severity
- affected node IDs, human-readable explanation, remediation steps

Five rule categories: traceability, constitution, sequence, quality, scope. Rules are pure functions — they take project state in and produce alarm lists out.

### Stale Propagation

> [`src-tauri/src/stale_propagation.rs`](src-tauri/src/stale_propagation.rs) — 499 LOC, 8 tests

Real dependency walk, not just a reason enum. On any upstream change:
1. Identify the changed node
2. Walk the traceability graph to find all dependents
3. Mark each Valid/Approved dependent as Stale with the specific reason
4. Recurse: if a newly-stale node had its own dependents, propagate further

Constitution amendments trigger the nuclear path: everything downstream becomes Stale.

### Readiness Gate Evaluator

> [`src-tauri/src/readiness_gate.rs`](src-tauri/src/readiness_gate.rs) — 731 LOC, 7 tests

One pure function that produces:
- Gate status (blocked/ready)
- Blocking reasons with rule provenance
- Stale artifact summary
- Outdated approval list
- Active blocking alarms
- Export manifest preview

Dead simple to call. Impossible for the frontend to fake.

### Export Compiler

> [`src-tauri/src/export_compiler.rs`](src-tauri/src/export_compiler.rs) — 782 LOC, 7 tests

Gate-guarded pure function that renders the canonical export package. Only callable when the readiness gate returns Ready.

Takes an `ExportInput` bundle (project, constitution, artifacts, versions, approvals, links, alarms, amendments, audit events) and produces an `ExportPackage` containing 14 files:

- `project.json` — machine-readable canonical source (full project serialization)
- `constitution.md` — rendered constitution with promise, fantasy, outcomes, anti-goals, quality bar, failure condition
- 7 artifact markdown files (`artifacts/*.md`) — one per authored artifact type, with state, version hash, and JSON content snapshot
- `reports/traceability-matrix.md` — source→target link table with validation summary
- `reports/audit-log.md` — chronological event log with actors
- `reports/drift-report.md` — active vs resolved alarms with severity and remediation
- `reports/execution-readiness-report.md` — gate status, blocking reasons, manifest, stale/outdated summaries
- `reports/integrity-attestation.md` — per-artifact hash chain, gate verdict proof, traceability attestation, package integrity

Each renderer is a pure function. The compiler runs the gate internally — if blocked, returns `ExportBlocked` with the full gate evaluation so the UI can show exactly why.

## UI Shell

The UI is aggressively subordinate to the Rust engine. It never computes readiness, invents state transitions, or decides export eligibility. Every answer comes from the backend.

### Architecture

- **Tauri command layer** ([`src-tauri/src/commands.rs`](src-tauri/src/commands.rs) — 1,253 LOC): 30 commands (22 read-only queries + 8 mutations). The bridge between engine and browser.
- **Project store** ([`src-tauri/src/store.rs`](src-tauri/src/store.rs) — 751 LOC): In-memory project state behind a Mutex. Seeded with 4 demo scenarios.
- **React shell** ([`src/`](src/)): Vite + React 19 + TypeScript. Three-column layout with sidebar inspector and bottom status bar.

### Tauri Commands

Read-only:
- `get_project_snapshot` — all artifacts with state, version, approval, link counts, gate status
- `get_artifact_detail` — one artifact with enriched trace links, alarms, legal transitions
- `get_readiness_gate` — full gate evaluation (runs the gate engine live)
- `get_export_preview` — runs the export compiler, returns file list or blocking reasons
- `get_audit_timeline` — full audit event history
- `get_artifact_history` — version history for one artifact
- `get_validation_report` — per-artifact 3-layer validation drill-down
- `get_version_diff` — diff between two specific versions
- `get_latest_diff` — diff from previous to current version
- `get_edit_impact` — blast radius preview for an edit
- `get_amendment_impact` — blast radius preview for an amendment
- `dry_run_import` — file diagnostics without loading
- `list_demo_scenarios` — available demo scenario names
- `get_project_health` — project-wide health assessment with recovery actions
- `get_recovery_actions` — per-artifact next actions with rule provenance
- `get_allowed_links` — legal link types and targets for an artifact
- `get_missing_links` — missing required trace links for an artifact

Mutations:
- `transition_artifact` — state machine-validated transition
- `approve_artifact` — creates approval record, transitions Valid → Approved
- `edit_artifact_content` — constrained content editing with stale propagation
- `propose_amendment` — start a constitutional amendment
- `assess_amendment_impact` — compute downstream blast radius
- `apply_amendment` — apply amendment, mark downstream stale
- `abandon_amendment` — cancel an amendment
- `save_project` / `load_project` / `load_project_with_repair` — file I/O
- `switch_demo_scenario` — switch to a named demo scenario
- `add_trace_link` / `remove_trace_link` — trace link CRUD

### Fourteen Views

| View | File | Purpose |
|------|------|--------|
| Artifact Index | [`ArtifactIndex.tsx`](src/views/ArtifactIndex.tsx) | Authoritative list — state, version, approval, link counts, alarms |
| Artifact Detail | [`ArtifactDetail.tsx`](src/views/ArtifactDetail.tsx) | Metadata, validation, trace links, alarms, transitions, next actions, approve |
| Readiness Gate | [`ReadinessGate.tsx`](src/views/ReadinessGate.tsx) | Pass/fail banner, blocking reasons with rule provenance and remediation |
| Export Panel | [`ExportPanel.tsx`](src/views/ExportPanel.tsx) | Ready/blocked status, 14-file package preview or blocking reasons |
| Graph View | [`GraphView.tsx`](src/views/GraphView.tsx) | Selected node + one hop in/out, click to navigate neighbors |
| Project Health | [`ProjectHealthView.tsx`](src/views/ProjectHealthView.tsx) | Prioritized recovery dashboard with rule provenance on every action |
| Link Authoring | [`LinkAuthoringView.tsx`](src/views/LinkAuthoringView.tsx) | Missing link detection + one-click authoring |
| Amendment Panel | [`AmendmentPanel.tsx`](src/views/AmendmentPanel.tsx) | Propose, assess impact, apply, complete/abandon |
| Validation Detail | [`ValidationDetail.tsx`](src/views/ValidationDetail.tsx) | Per-artifact 3-layer validation drill-down |
| Impact View | [`ImpactView.tsx`](src/views/ImpactView.tsx) | Blast radius preview before committing changes |
| Audit Timeline | [`AuditTimeline.tsx`](src/views/AuditTimeline.tsx) | Append-only event history with icons and color coding |
| Scenario Switcher | [`ScenarioSwitcher.tsx`](src/views/ScenarioSwitcher.tsx) | Demo scenario switching + file import/export/repair |
| Command Palette | [`CommandPalette.tsx`](src/views/CommandPalette.tsx) | Ctrl+K quick actions (15 commands) |
| App Shell | [`App.tsx`](src/App.tsx) | Three-column layout, sidebar with gate status + health, inspector |

### UI Rules

- The UI never computes readiness on its own. It only renders backend results.
- The UI never invents state transitions. Every transition goes through the Rust state machine.
- Illegal actions are visible-but-disabled with reasons. Hidden constraints create confusion; explicit constraints create trust.
- Every "why is this blocked?" answer is one click away. The inspector panel always shows gate status and blocker codes.
- Every recovery action shows which constitutional rule requires it and why it's prioritized.

## Demo Scenarios

Anchor ships with four pre-built scenarios that exercise different failure modes:

| Scenario | Theme | What It Demonstrates |
|----------|-------|---------------------|
| **Forge Quest** | Crafting RPG | Mixed artifact states — draft, complete, valid, approved. Gate blocked by unapproved artifacts. |
| **Crystal Sanctum** | Puzzle RPG | Healthy project — all 9 artifacts approved, full traceability, gate ready (or near-ready). |
| **Shadow Protocol** | Stealth game | Broken traceability — missing required trace links, orphan artifacts, active drift alarms. |
| **Ember Saga** | Narrative RPG | Post-amendment fallout — constitution changed, 7 artifacts stale, full reconciliation needed. |

Scenarios can be switched at runtime via the Scenario Switcher view or Ctrl+K command palette.

## Build Sequence

1. ✅ Canonical TypeScript types — `packages/schema/src/anchor-domain.ts`
2. ✅ Rust domain structs — `src-tauri/src/domain.rs`
3. ✅ Artifact lifecycle state machine — `src-tauri/src/state_machine.rs`
4. ✅ Traceability graph model — `src-tauri/src/traceability.rs`
5. ✅ Drift alarm rule engine — `src-tauri/src/drift_rules.rs`
6. ✅ Stale propagation — `src-tauri/src/stale_propagation.rs`
7. ✅ Execution readiness gate — `src-tauri/src/readiness_gate.rs`
8. ✅ Export compiler — `src-tauri/src/export_compiler.rs`
9. ✅ UI shell wired to backend law (5 views)
10. ✅ Governance layer — amendments, editing, audit log
11. ✅ Explainability — validation reports, diff engine, impact analysis, persistence
12. ✅ Operator fluency — recovery engine, link authoring, command palette, health dashboard
13. ✅ Release legibility — trust surfaces, acceptance tests, integrity attestation, README rewrite

## Audit Events

Every meaningful state change is recorded as an immutable `AuditEvent`:

| Category | Events |
|----------|--------|
| Project lifecycle | `project_created` |
| Constitution | `constitution_locked` · `amendment_started` · `amendment_impact_assessed` · `amendment_applied` |
| Artifact state | `artifact_created` · `artifact_updated` · `artifact_completed` · `artifact_validated` · `artifact_approved` · `artifact_marked_stale` |
| Traceability | `trace_link_created` · `trace_link_removed` |
| Drift | `drift_alarm_raised` · `drift_alarm_resolved` |
| Gate & export | `readiness_gate_computed` · `readiness_gate_passed` · `export_blocked` · `project_exported` |

## Final Constraint

Anchor is not a planning scrapbook. If the software ever allows users to complete ceremony without preserving coherence, it has failed its purpose.

---

## Design Decisions

Key choices and why they were made:

**Rust owns all law, React owns no authority.** The UI cannot compute readiness, invent transitions, or decide export eligibility. Every answer is rendered from backend results. This makes it impossible for a frontend bug to silently bypass a rule.

**Pure functions for engines, Mutex for state.** The drift engine, gate evaluator, and export compiler are all pure functions — input in, result out. The only mutable state lives behind a `Mutex<ProjectStore>` in the Tauri process. This makes testing trivial and concurrency bugs impossible at the engine layer.

**Bidirectional traceability is mandatory, not optional.** Every node must answer both "what justifies this?" and "what depends on this?" — orphan features, unjustified systems, and unvalidated phases are structural errors, not warnings. This catches drift that would otherwise be invisible until export.

**Stale propagation is recursive, not one-hop.** When an upstream artifact changes, the stale walk follows the traceability graph to all transitive dependents. A constitution amendment marks everything stale. This is expensive but honest — shallow invalidation creates false confidence.

**Gate is computed, never authored.** The Execution Readiness Gate is a pure function of project state. No human can "override" it. This is the core guarantee: if the gate says Ready, the project is coherent.

**Export is a consequence, not a feature.** The export compiler runs the gate internally. If blocked, it returns the gate evaluation instead of the package. There is no backdoor, no "export anyway" flag, no admin override.

**Visible-but-disabled > hidden.** Illegal transitions are shown in the UI with explanations of why they're blocked, rather than hidden. Hidden constraints create confusion; explicit constraints create trust.

**9 artifact types, locked.** The artifact spine is an enum, not a config. Adding a 10th type requires a schema version upgrade. This prevents scope creep in the design model itself.

## Invariants

These must hold at all times. If any invariant is violated, it is a bug.

1. **No artifact can reach Approved without passing all three validation layers** (structural, relational, intent)
2. **No export can be produced while any artifact is Stale**
3. **No export can be produced while any blocking drift alarm is active**
4. **Every TraceLink is bidirectionally navigable** — if A→B exists, querying B returns A as an upstream
5. **Every Approval is bound to a specific content hash and constitution version** — if either changes, the approval is invalid
6. **Stale propagation is transitive** — if A invalidates B and B was upstream of C, C also becomes Stale
7. **Amendment impact assessment lists all affected artifacts before the amendment is applied**
8. **The gate evaluator runs all checks on every call** — it never caches, shortcuts, or returns stale results
9. **Audit events are append-only** — no event can be deleted or modified after creation
10. **The frontend never modifies artifact state directly** — all mutations go through Tauri commands backed by the Rust engine

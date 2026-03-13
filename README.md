# Anchor

**A drift-prevention engine for serious creative software design.**

Anchor is a local-first Tauri 2 desktop app that forces constitution-first, fully traceable project design. It blocks export until every artifact is coherent, approved, and bidirectionally linked to the original promise — preventing the classic failure mode where progress looks organized but no longer does what it was born to do.

> If a user can fill every field, mark everything "complete," and export while the artifacts no longer agree with each other — that is total failure, even if the UI is beautiful.

## Why Anchor Exists

Phase-by-phase development has a hidden failure mode: each checkpoint becomes a tiny regime change. The original product thesis gets nibbled to death by "reasonable" adjustments. Completing steps in ways that no longer agree with each other is the sneakier failure. Anchor enforces **coherence**, not just completion.

## How It Works

### The Artifact Spine

Every project contains exactly nine artifacts, worked in strict order:

| # | Artifact | Role |
|---|----------|------|
| 1 | **Product Constitution** | The throne — promise, fantasy, anti-goals, quality bar, failure condition |
| 2 | **User Fantasy + Workflows** | Narrative + concrete workflow definitions linked to constitution clauses |
| 3 | **Feature Map** | Features with upstream workflow justification and anti-goal conflict checks |
| 4 | **System Architecture** | Systems, responsibilities, boundaries, feature implementation links |
| 5 | **UX State Map** | States, transitions, entry conditions, blocked actions |
| 6 | **Phase Roadmap + Contracts** | Phases with inputs, outputs, invariants, forbidden compromises |
| 7 | **Acceptance Checklists** | Per-phase checklist groups with constitution-linked items |
| 8 | **Drift Alarm Definitions** | Alarm types, trigger conditions, severity, remediation templates |
| 9 | **Execution Readiness Gate** | Computed, not authored — the final judge |

No export until the gate clears. No gate clearance with stale artifacts, active drift alarms, or broken traceability.

### State Machine

```
Draft → Complete → Valid → Approved
                                ↓
                              Stale ← (upstream change)
                                ↓
                       Complete → Valid → Approved (reconcile & re-approve)
```

Three validation layers at each step: structural (fields, hashes), relational (traceability links), intent (human review).

### Law Engine

The Rust backend is the final authority. The UI renders what the engine decides — it never computes readiness, invents transitions, or skips checks.

**Core systems:**

- **State machine** — 5 states, 10 legal transitions, 4 explicitly forbidden. Every transition validated against preconditions.
- **Bidirectional traceability** — every node must answer "what justifies this?" and "what depends on this?" Six link types enforcing the dependency graph.
- **Stale propagation** — upstream changes walk the dependency graph and mark downstream artifacts stale. Constitution amendments trigger nuclear: everything downstream invalidated.
- **Amendment protocol** — constitutions can change, but change is formal (Proposed → ImpactAssessed → Applied → ReconciliationPending → Completed). Fallout is computed before apply.
- **Drift alarms** — 5 categories (traceability, constitution, sequence, quality, scope), each with rule provenance and remediation paths.
- **Readiness gate** — 6 blocking checks: artifact states, stale artifacts, drift alarms, amendment completion, approval currency, traceability completeness.
- **Recovery engine** — for any artifact or project state, computes "what's next?" with priority order, constitutional rule references (§-numbered clauses), and priority explanations.
- **Export compiler** — gate-guarded 14-file package including integrity attestation with per-artifact content hash chain.
- **Persistence** — atomic save/load with integrity hashing, corruption detection, dry-run import diagnostics, and auto-repair for corrupted files.

### Trust Surfaces

Every recommendation in Anchor is explainable:

- Recovery actions carry `rule_clause` (e.g. "§13.3 — No stale artifact may be present at export time") and `why_first` (e.g. "Stale artifacts block the readiness gate. Until reconciled, no export is possible.")
- Blocking reasons show exact rule provenance, affected artifacts, and step-by-step remediation
- Export packages include an integrity attestation proving every artifact was Approved, all trace links were present, and no blocking drift alarms were active at export time
- The gate never says "blocked" without telling you exactly why and what to do

## Stack

| Layer | Technology | Role |
|-------|-----------|------|
| Backend | Rust (Tauri 2) | Final authority — validation, hashing, state transitions, export |
| Frontend | React 19 + TypeScript | Window into law — forms, graph, health dashboard, command palette |
| Storage | Local JSON with integrity hashing | No cloud dependency |
| Network | None | Fully local-first |

## Project Structure

```
src-tauri/src/         (12,400 LOC Rust — 21 modules, 166 tests)
  domain.rs            Canonical types: 11 enums, 11 entities
  state_machine.rs     Lifecycle transitions + downstream type graph
  traceability.rs      Bidirectional graph validation (6 link rules)
  drift_rules.rs       5-category drift alarm engine
  stale_propagation.rs Recursive dependency walk (direct + transitive)
  readiness_gate.rs    6-check gate evaluator
  export_compiler.rs   Gate-guarded 14-file renderer + integrity attestation
  recovery.rs          Next-action engine with rule provenance
  link_authoring.rs    Trace link CRUD with legality checks
  amendments.rs        Constitutional amendment protocol
  editing.rs           Content editing with constraint enforcement
  validation.rs        Per-artifact 3-layer validation reports
  diff.rs              Version diffing + approval impact analysis
  impact.rs            Blast radius computation
  persistence.rs       Atomic save/load, corruption detection, auto-repair
  audit_log.rs         Append-only audit event generation
  store.rs             4 demo scenarios (Forge Quest, Crystal Sanctum, Shadow Protocol, Ember Saga)
  commands.rs          30 Tauri IPC commands
  acceptance_tests.rs  13 golden-path scenario-level tests
  lib.rs               Module declarations + Tauri setup

src/                   (3,700 LOC TypeScript/React — 14 views)
  App.tsx              Three-column shell, sidebar, command palette (Ctrl+K)
  types.ts             TS interfaces matching Rust serde output
  views/
    ArtifactIndex.tsx      Artifact table: state, version, links, alarms
    ArtifactDetail.tsx     Metadata, trace links, transitions, next actions
    ReadinessGate.tsx      Gate evaluation: pass/fail, blockers, remediation
    ExportPanel.tsx        Package preview or blocking reasons
    GraphView.tsx          Focused one-hop dependency neighborhood
    ProjectHealthView.tsx  Prioritized recovery dashboard with rule provenance
    LinkAuthoringView.tsx  Missing link detection + one-click authoring
    AmendmentPanel.tsx     Propose, assess, apply, complete/abandon
    ValidationDetail.tsx   Per-artifact 3-layer validation drill-down
    ImpactView.tsx         Blast radius preview before committing changes
    AuditTimeline.tsx      Append-only event history
    ScenarioSwitcher.tsx   Demo scenario switching + file import/export
    CommandPalette.tsx     Ctrl+K quick actions (15 commands)
```

**~16,100 lines of source code. 166 Rust tests across 21 modules. Zero dependencies beyond Tauri + serde.**

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- Platform build tools for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

### Setup

```bash
git clone https://github.com/mcp-tool-shop-org/anchor.git
cd anchor
npm install
```

### Development

```bash
npm run tauri dev
```

The app opens with a demo project ("Forge Quest") pre-loaded with artifacts in mixed states — the gate is blocked, so you immediately see the "why blocked?" experience. Use the scenario switcher or Ctrl+K command palette to explore all four demo projects.

### Build

```bash
npm run tauri build
```

### Tests

```bash
cd src-tauri
cargo test
```

166 tests validate the entire law engine:

- **State machine**: transitions, preconditions, forbidden paths
- **Traceability**: link requirements, bidirectional explainability, endpoint resolution
- **Drift detection**: 5 alarm categories with rule provenance
- **Stale propagation**: direct, transitive, and constitutional amendment paths
- **Gate evaluation**: all 6 blocking checks
- **Export compilation**: 14-file package, integrity attestation
- **Recovery engine**: per-artifact and project-wide action generation with provenance
- **Persistence**: save/load round-trip, corruption detection, dry-run diagnostics, auto-repair
- **Golden-path acceptance**: full scenario-level tests — Shadow Protocol recovery, amendment lifecycle, edit propagation

### Type Check

```bash
npx tsc --noEmit
```

## Demo Scenarios

Anchor ships with four pre-built scenarios that exercise different failure modes:

| Scenario | Theme | What It Demonstrates |
|----------|-------|---------------------|
| **Forge Quest** | Crafting RPG | Mixed artifact states, gate blocked by draft + unapproved artifacts |
| **Crystal Sanctum** | Puzzle RPG | Healthy project — all approved, full traceability, gate ready |
| **Shadow Protocol** | Stealth game | Broken traceability — missing trace links, orphan artifacts, active drift alarms |
| **Ember Saga** | Narrative RPG | Post-amendment fallout — constitution changed, 7 artifacts stale, full reconciliation needed |

## Architecture

The UI is aggressively subordinate to the engine.

```
┌──────────────────────────────────────────────────────┐
│                     React Shell                       │
│  14 views · Ctrl+K palette · sidebar · inspector      │
├──────────────────────────────────────────────────────┤
│                Tauri Command Layer                     │
│           30 commands (22 read · 8 write)             │
├──────────────────────────────────────────────────────┤
│                 Rust Law Engine                        │
│  state_machine · traceability · drift_rules            │
│  stale_propagation · readiness_gate · recovery         │
│  export_compiler · validation · amendments             │
│  editing · link_authoring · persistence · impact       │
└──────────────────────────────────────────────────────┘
```

**UI rules:**
- The UI never computes readiness — it only renders backend results
- The UI never invents state transitions — every transition goes through the Rust state machine
- Illegal actions are visible-but-disabled with reasons (not hidden)
- Every "why blocked?" answer is one click away
- Every recovery action shows which constitutional rule requires it and why it's prioritized

## Documentation

See [handbook.md](handbook.md) for the complete product specification:

- Product constitution (promise, fantasy, anti-goals, quality bar, failure condition)
- Artifact spine (9 types, strict ordering)
- State machine rules (5 states, 10 transitions, 4 forbidden)
- Traceability requirements (6 link types, bidirectional explainability)
- Amendment protocol (formal change with downstream reconciliation)
- Drift alarm taxonomy (5 categories with rule provenance)
- Execution readiness gate (6 blocking checks)
- Export contract (14-file package with integrity attestation)
- Recovery engine (next-action computation with rule clause references)
- Persistence (atomic save/load with integrity, dry-run diagnostics, auto-repair)
- UI shell architecture (14 views, 30 commands, command palette)
- Audit event catalog (19 event types)

## License

[MIT](LICENSE)

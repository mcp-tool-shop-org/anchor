# Anchor

**A drift-prevention engine for serious creative software design.**

Anchor is a local-first Tauri desktop app that forces constitution-first, fully traceable project design. It blocks export until every artifact is coherent, approved, and bidirectionally linked to the original promise — preventing the classic failure mode where progress looks organized but no longer does what it was born to do.

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

### Key Mechanisms

- **Bidirectional traceability** — every node must answer "what justifies this?" and "what depends on this?"
- **Amendment protocol** — constitutions can change, but change is formal, visible, and forces downstream reconciliation
- **Drift alarms** — 5 categories (traceability, constitution, sequence, quality, scope), each with rule provenance and remediation
- **"Why blocked?" panels** — every blocked action shows the exact rule, upstream artifact, and fix
- **Gate-guarded export** — the 13-file export package is a consequence of readiness, not a side door around it

## Stack

| Layer | Technology | Role |
|-------|-----------|------|
| Backend | Rust (Tauri 2) | Final authority — validation, hashing, state transitions, export |
| Frontend | React 19 + TypeScript | Window into law — forms, graph visualization, "Why blocked?" |
| Storage | Local JSON + optional SQLite | No cloud dependency |
| Network | None | Optional update check only |

## Project Structure

```
src-tauri/
  src/
    domain.rs              # 11 enums, 11 entities, 9 content shapes (684 LOC)
    state_machine.rs       # Lifecycle transitions + stale propagation (731 LOC, 12 tests)
    traceability.rs        # Bidirectional graph validation (515 LOC, 9 tests)
    drift_rules.rs         # 5-category drift alarm engine (537 LOC, 10 tests)
    stale_propagation.rs   # Recursive dependency walk (499 LOC, 8 tests)
    readiness_gate.rs      # 6-check gate evaluator (731 LOC, 7 tests)
    export_compiler.rs     # Gate-guarded 13-file renderer (719 LOC, 7 tests)
    store.rs               # In-memory project store + demo data (198 LOC)
    commands.rs            # Tauri IPC: 4 queries + 2 mutations (415 LOC)
    lib.rs                 # Module declarations + run()
    main.rs                # Tauri entry point
  Cargo.toml
  tauri.conf.json
src/
  App.tsx                  # Three-column shell + bottom status bar
  App.css                  # Dark monospace theme
  types.ts                 # TS interfaces matching Rust serde output
  views/
    ArtifactIndex.tsx      # Artifact table: state, version, links, alarms
    ArtifactDetail.tsx     # Metadata, trace links, transitions, approve action
    ReadinessGate.tsx      # The throne room: pass/fail, blockers, remediation
    ExportPanel.tsx        # Package preview or blocking reasons
    GraphView.tsx          # Focused one-hop dependency neighborhood
packages/
  schema/src/
    anchor-domain.ts       # Canonical TypeScript types (source of truth)
```

**~6,500 lines of source code. 59 Rust tests across 7 modules. Zero dependencies beyond Tauri + serde.**

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

This starts the Vite dev server and the Tauri app together. The app opens with a demo project ("Forge Quest") pre-loaded with artifacts in mixed states — the gate is blocked, so you immediately see the "why blocked?" experience.

### Build

```bash
npm run tauri build
```

Produces a platform-native executable in `src-tauri/target/release/`.

### Tests

```bash
cd src-tauri
cargo test
```

All 59 tests validate the law engine: state machine transitions, traceability rules, drift detection, stale propagation, gate evaluation, and export compilation.

### Type Check

```bash
npx tsc --noEmit
```

## Architecture

The UI is aggressively subordinate to the engine.

```
┌──────────────────────────────────────────────────┐
│                   React Shell                     │
│  Artifact Index · Detail · Gate · Export · Graph  │
├──────────────────────────────────────────────────┤
│              Tauri Command Layer                  │
│    4 read queries  ·  2 mutation commands         │
├──────────────────────────────────────────────────┤
│               Rust Law Engine                     │
│  state_machine · traceability · drift_rules       │
│  stale_propagation · readiness_gate               │
│  export_compiler · domain · store                 │
└──────────────────────────────────────────────────┘
```

**UI rules:**
- The UI never computes readiness — it only renders backend results
- The UI never invents state transitions — every transition goes through the Rust state machine
- Illegal actions are visible-but-disabled with reasons (not hidden)
- Every "why blocked?" answer is one click away

### Tauri Commands

| Command | Type | Purpose |
|---------|------|---------|
| `get_project_snapshot` | Read | All artifacts with state, version, links, gate status |
| `get_artifact_detail` | Read | One artifact with enriched trace links, alarms, legal transitions |
| `get_readiness_gate` | Read | Full gate evaluation (runs engine live) |
| `get_export_preview` | Read | Export compiler result or blocking reasons |
| `transition_artifact` | Write | State machine-validated transition |
| `approve_artifact` | Write | Creates approval record, transitions Valid → Approved |

## Documentation

See [handbook.md](handbook.md) for the complete product specification:

- Product constitution (promise, fantasy, anti-goals, quality bar, failure condition)
- Artifact spine (9 types, strict ordering)
- State machine rules (5 states, 10 transitions, 5 forbidden)
- Traceability requirements (6 link types, bidirectional explainability)
- Amendment protocol (formal change with downstream reconciliation)
- Drift alarm taxonomy (5 categories with rule provenance)
- Execution readiness gate (6 blocking checks)
- Export contract (13-file package)
- UI shell architecture (5 views, command layer, UI rules)
- Audit event catalog (19 event types)

## License

[MIT](LICENSE)

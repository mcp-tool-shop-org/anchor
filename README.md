# Anchor

A drift-prevention engine for serious creative software design.

Anchor is a local-first desktop app that forces constitution-first, fully traceable project design so creative products cannot drift from their intended purpose before execution begins.

## What It Does

You open Anchor with a serious creative project, and it guides you through a disciplined sequence that produces a complete, coherent, handover-ready plan — every decision justified, traceable, and locked to the original promise.

**This is not a planning app.** It is enforcement software.

## Core Idea

Progress means coherence maintained under change, not "forms got filled."

A user can fill every field, mark everything "complete," and still produce garbage if the artifacts no longer agree with each other. Anchor prevents that by enforcing three layers of validation (structural, relational, intent) and blocking export until everything is coherent.

## The Artifact Spine

Every project contains exactly nine artifacts, worked in strict order:

1. **Product Constitution** — the throne
2. **User Fantasy + Core Workflows**
3. **Feature Map**
4. **System Architecture Map**
5. **UX State Map**
6. **Phase Roadmap + Contracts**
7. **Acceptance Checklists**
8. **Drift Alarm Definitions**
9. **Execution Readiness Gate** — computed, not authored

No export until the gate clears. No gate clearance with stale artifacts, active drift alarms, or broken traceability.

## Key Mechanisms

- **Artifact state machine:** Draft → Complete → Valid → Approved → Stale (on upstream change)
- **Bidirectional traceability:** every node must justify its existence and show what depends on it
- **Amendment protocol:** constitutions can change, but change is a formal, visible event with mandatory downstream reconciliation
- **Drift alarms:** traceability, constitution, sequence, quality, and scope drift — each with rule provenance and remediation path
- **"Why blocked?" panels:** every blocked action explains exactly what rule is violated, which upstream artifact is involved, and what fixes it

## Stack

| Layer | Technology | Role |
|-------|-----------|------|
| Backend | Tauri (Rust) | Final authority — validation, hashing, state transitions, export |
| Frontend | React + TypeScript | Window into law — forms, graph visualization, "Why blocked?" display |
| Storage | Local JSON + optional SQLite | No cloud dependency |
| Network | None | Optional update check only |

## Project Structure

```
packages/
  schema/
    src/
      anchor-domain.ts     # Canonical TypeScript types (source of truth)
src-tauri/
  src/
    domain.rs              # Rust domain structs (mirrors TS types)
    state_machine.rs       # Lifecycle transitions, stale propagation, gate computation
    traceability.rs        # Traceability graph validation + bidirectional queries
    drift_rules.rs         # Drift alarm rule engine (5 categories)
    stale_propagation.rs   # Recursive dependency walk + stale marking
    readiness_gate.rs      # Execution readiness gate evaluator
    export_compiler.rs     # Gate-guarded export package renderer
    store.rs               # In-memory project store with demo data
    commands.rs            # Tauri command layer (4 reads + 2 mutations)
    main.rs                # Tauri app entry point
    lib.rs                 # Module declarations + run()
  Cargo.toml               # Crate configuration
  tauri.conf.json          # Tauri app configuration
  capabilities/default.json
src/
  App.tsx                  # Shell layout (sidebar, center, inspector, status bar)
  App.css                  # Dark theme styles
  types.ts                 # TypeScript interfaces matching Rust serde output
  views/
    ArtifactIndex.tsx       # Artifact list with state, version, links, alarms
    ArtifactDetail.tsx      # Single artifact: metadata, links, transitions, approval
    ReadinessGate.tsx       # Gate status, blockers, remediation, manifest
    ExportPanel.tsx         # Export preview or blocking reasons
    GraphView.tsx           # Focused dependency graph (one-hop neighborhood)
handbook.md                # Full product handbook
```

## Status

Schema Pack v1 + Law Engine + UI Shell complete:
- [x] Canonical TypeScript types (11 enums, 11 domain entities, 9 artifact content shapes)
- [x] Rust domain structs with serde
- [x] Artifact lifecycle state machine with tests
- [x] Traceability graph model (bidirectional queries, link validation, explainability checks)
- [x] Drift alarm rule engine (traceability, constitution, sequence, scope drift detection)
- [x] Stale propagation (recursive dependency walk, constitution amendment nuclear path)
- [x] Execution readiness gate evaluator (6 blocking checks, export manifest preview)
- [x] Export compiler (gate-guarded, 13-file export package, 7 renderers)
- [x] UI shell (Tauri command layer + React shell with 5 views)

59 Rust tests across 7 modules, all passing. Tauri app compiles. Frontend builds clean.

## Documentation

See [handbook.md](handbook.md) for the full product specification: constitution, artifact spine, state machine rules, traceability requirements, amendment protocol, drift alarm taxonomy, export contract, and build sequence.

## License

MIT

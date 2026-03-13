# Contributing to Anchor

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- Platform build tools for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

## Setup

```bash
git clone https://github.com/mcp-tool-shop-org/anchor.git
cd anchor
npm install
```

## Development

```bash
npm run tauri dev
```

Opens the app with a demo project ("Forge Quest") pre-loaded in mixed states so you can immediately see drift detection and gate blocking in action.

## Tests

```bash
cd src-tauri
cargo test
```

All 59 tests must pass before submitting changes. Tests cover: state machine transitions, traceability rules, drift detection, stale propagation, gate evaluation, and export compilation.

## Type Check

```bash
npx tsc --noEmit
```

Must pass with zero errors.

## Architecture Rules

These are non-negotiable. Read [handbook.md](handbook.md) for the full rationale.

- **Rust is the law.** All validation, state transitions, and export decisions live in the Rust backend. The frontend renders results — it never computes authority.
- **Pure functions for engines.** The drift engine, gate evaluator, and export compiler are stateless pure functions. Keep them that way.
- **No new artifact types** without a schema version upgrade. The 9-type spine is enum-locked.
- **No export backdoors.** The export compiler runs the gate internally. There is no override flag.
- **Visible-but-disabled over hidden.** Illegal actions show explanations, not empty space.

## Code Style

- **Rust:** Follow `cargo fmt` defaults. No `unsafe`. No `unwrap()` in production paths — use proper error handling.
- **TypeScript:** Follow existing patterns. Types mirror Rust serde output (see `src/types.ts`).
- **Commits:** Short imperative subject line. Reference the build sequence step or module if applicable.

## Pull Requests

1. Fork and create a feature branch
2. Make your changes
3. Run `cargo test` (all 59 tests pass) and `npx tsc --noEmit` (zero errors)
4. Open a PR with a clear description of what changed and why

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).

bead_id: vb-6r5
bead_title: Add max-speed xtask proof/test orchestrator
phase: 2
updated_at: 2026-05-18T01:40:00Z
attempt: 1-of-7

# Codebase Map - State 2

## Touched Crates
- `xtask` — Primary crate for new proof/test orchestrator

## New Files to Create
- `xtask/src/discovery.rs` — Workspace crate discovery via cargo metadata
- `xtask/src/scheduler.rs` — DAG scheduler with bounded parallelism
- `xtask/src/lanes.rs` — Proof/test lane definitions and command generation
- `xtask/src/logger.rs` — JSONL structured logging per crate/lane
- `xtask/src/profiles.rs` — Profile-based lane selection (fast/standard/deep/proof/all)
- `xtask/src/summary.rs` — Human-readable summary output
- `xtask/src/proof_orchestrator.rs` — Orchestrates proof run commands

## Modified Files
- `xtask/src/cli.rs` — Add new subcommands: list-crates, proof list, proof run, proof crate, proof affected
- `xtask/src/main.rs` — Dispatch new commands
- `xtask/Cargo.toml` — Add dependencies (serde_json already present, may need crossbeam-channel for parallelism)

## Existing Files to Preserve
- All existing xtask commands must continue to work
- `proof.rs` — Existing proof obligation loading (may be reused)
- `command_family.rs` — Existing command family enum
- `registry.rs` — Existing command registry
- `routing.rs` — Existing routing logic
- `shell.rs` — Shell helpers
- `status.rs` — Structured status output

## Proof/Test Lanes to Support
1. `cargo test` — Standard Rust tests
2. `cargo nextest run` — Nextest (if available)
3. `cargo kani` — Kani bounded model checking (if available)
4. `cargo miri test` — Miri UB detection (if available)
5. `cargo test --features loom` — Loom concurrency (if available)
6. `cargo fuzz run` — Fuzz smoke (if available)
7. `cargo clippy` — Source lint
8. `cargo mutants` — Mutation testing (if available)
9. `cargo tarpaulin` / `cargo llvm-cov` — Coverage (if available)
10. `verus` — Verus proofs (if available)
11. `tla2tools` — TLA+ model checking (if available)
12. `cargo flux` — Flux refinement types (if available)
13. `proof-obligations.jsonl` commands — Per-crate proof obligation commands

## Dependencies
- `cargo_metadata` — For workspace discovery (new dependency)
- `crossbeam-channel` — For parallel execution (workspace dependency available)
- Existing: clap, anyhow, serde, serde_json

## Risk Tags
- MEDIUM: Parallel execution complexity
- LOW: CLI parsing (clap derive handles most)
- LOW: JSONL logging (straightforward serialization)
- MEDIUM: Tool availability detection (graceful degradation needed)

## Required Verifier Modes
- verify-standard: cargo test, cargo clippy
- verify-deep: nextest, kani, miri, loom, fuzz
- verify-proof: verus, tla, flux
- verify-all: all lanes

## Release Critical
- No — this is a tooling bead, not production runtime

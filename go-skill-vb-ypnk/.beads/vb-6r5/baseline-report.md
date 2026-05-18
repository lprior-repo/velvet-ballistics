bead_id: vb-6r5
bead_title: Add max-speed xtask proof/test orchestrator
phase: 1
updated_at: 2026-05-18T01:35:00Z
attempt: 1-of-7

# Baseline Report - State 1

## Pre-Edit xtask State

### Existing xtask Commands (legacy CLI via clap)
- `ui-snapshot` — UI snapshot capture
- `ui-tokens` — Design token generation
- `ui-overlap-check` — UI overlap detection
- `ai-fast`, `ai-deep`, `ai-release` — AI context/plan/check commands
- `proof-plan` — List proof obligations from YAML
- `proof-check` — Run proof checks by level
- `proof-evidence` — Write proof evidence bundle
- `proof-drift` — Check spec alignment
- `loom` — Run loom concurrency models
- `forbidden-scan` — Scan for forbidden patterns

### Existing Required Command Families (20 families, all placeholder/deferred)
ai-context, ai-plan, ai-check, ai-evidence, invariants, scans, cert-check, perf, replay, crash, diff, mutants, loom, kani, fuzz, prop, repro, test-plan, review, why-failed

### Workspace Crates (19 members)
vb_boundary_inventory, vb_core, vb_yaml, vb_validate, vb_expr, vb_compile, vb_storage, vb_runtime, vb_doc, vb_ipc, vb_codegen, vb_ui_makepad, vb_ui_snapshot, vb_proof_kernels, vb_cli, workspace_tests, vb_benchmark, fuzz, xtask

### Existing Proof Infrastructure
- `contracts/proof_obligations.yaml` — YAML-based proof obligations (27KB)
- `contracts/invariants.yaml` — Invariant definitions (12KB)
- `verification/kani/`, `verification/tla/`, `verification/verus/` — Verification artifact directories
- `proof.rs` module — Loads YAML obligations, generates commands per obligation

### Current xtask Architecture
- Two-layer: `lib.rs` (command parsing, routing, status) + `main.rs` (CLI dispatch)
- `parser.rs` — Parses xtask command strings into `XtaskCommand` enum
- `routing.rs` — Routes `XtaskCommand` to `StructuredStatus` (all deferred)
- `shell.rs` — stdout/stderr helpers, arg normalization
- `status.rs` — Structured status output (JSON/text)
- `command_family.rs` — 20 command family enum variants
- `registry.rs` — Command family validation

### Dependencies
- clap 4 (derive), anyhow 1, serde 1, serde_json 1, serde_yaml 0.9, serde-saphyr (workspace), toml 0.9, image 0.25, tempfile 3, blake3 (workspace), regex 1
- vb_ui_snapshot (path dependency)

### No Existing Parallel Execution Infrastructure
- No DAG scheduler
- No cargo metadata discovery
- No per-lane timeout support
- No JSONL structured logging per run
- No profile-based lane selection

## Baseline CI State
- `moon ci` is canonical gate
- Zero-tolerance source lint (clippy deny for correctness, suspicious, perf, complexity, unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro, indexing_slicing, arithmetic_side_effects, as_conversions)

## Key Constraints
- No unsafe, unwrap, expect, panic, todo, unimplemented, dbg
- Functions under 25 lines, max 5 parameters (Holzman Rust)
- Pure logic separated from I/O
- Speed is primary product metric

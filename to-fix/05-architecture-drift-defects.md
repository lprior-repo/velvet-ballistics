# Architecture Drift Defects

## Status Update 2026-06-03

Closed or reconciled: oversized-file enumeration (`vb-zxgb`), source-length gate expansion (`vb-ui6k`), 300-line Rust policy gate (`vb-jpq7.47`), bounded timer wheel replacement (`vb-vi3g`), workspace membership reconciliation (`vb-esq9.2`), and deferred codegen graph quarantine (`vb-esq9.3`).

Still open: hot runtime state boundedness (`vb-jpq7.9`), hot dispatcher split (`vb-9kwz.1`), shard tick command handler split (`vb-9kwz.2`), root Cargo profile contract (`vb-esq9.1`), duplicate compiler module tree (`vb-esq9.4`), and their umbrellas `vb-9kwz` and `vb-esq9`.

## P0: File-size drift is massive

Evidence:

- Architecture subagent reported 378 first-party `.rs` files over 300 lines, excluding `.evidence`, `.cargo_temp`, `target`, `.beads`, and `.moon`.
- Representative files: `crates/vb_storage/src/tests.rs` at 7514 lines, `crates/vb_core/src/budget/tests.rs` at 7227 lines, `crates/vb_cli/src/app_impl.rs` at 6292 lines, `crates/vb_runtime/src/runtime.rs` at 2611 lines, `crates/vb_core/src/value_store.rs` at 2552 lines.

Master violated:

- Section 3: hot functions must be short and source-length gate must enforce hot functions over 25 logical lines.
- Architecture-drift repo rule: files over 300 lines must be split.

Impact: Reviewability, DDD cohesion, and mechanical auditability are badly degraded.

Suggested bead: `P0 split first-party Rust files over 300 lines`

## P0: Source-length gate is too narrow

Evidence:

- Subagent reported `scripts/check-source-length.sh` only checks compile globals and selected hot test functions, while independent scan found hundreds of over-300-line files.

Master violated:

- Section 3 and Section 40 source-length gate requirements.

Impact: CI can pass while most file/function size drift remains invisible.

Suggested bead: `P0 expand source-length gate to all first-party Rust and hot runtime modules`

## P0: Hot runtime dispatcher is monolithic

Evidence:

- `crates/vb_runtime/src/engine/execute.rs:45-371` defines `execute_node_full` as a giant dispatcher over most IR families.

Master violated:

- Section 3: hot functions target <=25 logical lines.
- Section 44 point 20: forbidden constructs and hot-path behavior must be mechanically enforceable.

Impact: Hot runtime behavior is hard to review, hard to prove, and easy to regress.

Suggested bead: `P0 split runtime execute_node_full dispatcher`

## P0: Hot shard tick command dispatch is oversized

Evidence:

- Architecture subagent found `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:162-238` handling many `ShardCommand` variants in one function.

Master violated:

- Section 20 shard design.
- Section 3 hot function length.

Impact: Scheduler control flow is too broad for the hot loop and hard to verify.

Suggested bead: `P0 split shard tick command dispatch handlers`

## P1: Hot shard state uses map-like live runtime structures

Evidence:

- `crates/vb_runtime/src/shard/types.rs:401-409` stores live `runs`, `runtime_states`, `terminal_runs`, `journal_sequences`, `pending_timers`, and `frame_pools` in `IndexMap`/`IndexSet`.

Master violated:

- Section 11: hot state must use numeric indices, handle tables, boxed slices, fixed-capacity stacks, bounded queues.
- Section 12: runtime maps are forbidden in hot runtime paths.
- Section 20: run belongs to exactly one shard with bounded state.

Impact: Hot runtime state retains map allocation/hash/index overhead and weakens bounded resource claims.

Suggested bead: `P1 replace hot shard maps with bounded numeric-index arenas`

## P1: Timer wheel uses map/vector-backed hot storage

Evidence:

- `crates/vb_runtime/src/shard/timer_wheel.rs` uses `BTreeMap<Instant, Vec<TimerEntry>>` and `HashMap<RunId, TimerEntry>` per subagent inspection.

Master violated:

- Section 20 timer wheel requirement.
- Section 12 forbidden hot-path APIs.

Impact: Timer path can allocate and is not a bounded wheel/ring style structure.

Suggested bead: `P1 replace timer maps with bounded timer wheel storage`

## P1: Workspace shape drifts from active master target

Evidence:

- Root `Cargo.toml:2-23` includes extra active crates (`vb_boundary_inventory`, `vb_doc`, `vb_proof_kernels`, `vb_cli`, `vb_verification`, `vb_benchmark`) and excludes `crates/workspace_tests`.
- The active target in master Sections 23 and 34 lists core backend crates plus `velvet_ballistics` and workspace tests/fuzz expectations.

Master violated:

- Sections 23 and 34.

Impact: Agents and CI can audit the wrong scope, and benchmark/test package references can break.

Suggested bead: `P1 reconcile Cargo workspace with master current-scope contract`

## P1: Deferred codegen residue remains in active graph

Evidence:

- `crates/vb_codegen/` exists even though root `Cargo.toml:13-14` says deferred crates were moved out.
- Rust policy subagent found `vb_compile` still depending on `vb_codegen` metadata/dependency graph.

Master violated:

- Sections 22, 23, 32, 34.

Impact: Deferred codegen can be mistaken as active scope or accidentally included in current gates.

Suggested bead: `P1 remove or quarantine deferred vb_codegen from active compile graph`

## P1: Duplicate compiler module tree exists

Evidence:

- Architecture subagent found byte-identical duplicates such as `crates/vb_compile/src/expression_bytecode.rs` and `crates/vb_compile/src/compile/bytecode.rs`, plus similar duplicate `expression.rs` and `schema.rs` trees.

Master violated:

- Section 28 compiler surface.
- Phase 42 validation deduplication.

Impact: Duplicate implementation surfaces invite divergent fixes and false evidence.

Suggested bead: `P1 remove duplicate vb_compile module tree`

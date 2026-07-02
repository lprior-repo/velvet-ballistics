# Black-Hat Review — vb-vt2f State 12 attempt 3

STATUS: APPROVED

## Startup Evidence

- Read `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md`; cited control rules: contract parity first (`lines 12-17`), engineering/test rigor (`lines 18-21`), typed state discipline (`lines 23-28`), panic/weakness scan (`lines 30-33`), and findings-first line-cited output (`lines 40-44`).
- Read `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md`; content matches the Claude copy, and this agents copy wins on conflict.
- Manifest read: `.beads/vb-vt2f/dispatch-state12-black-hat-attempt3.json` requires State 12 rerun after terminal tombstone implementation and State 11 approval.

## Findings

No blocking findings.

## Adversarial Checks

- LETHAL-001 is no longer trace-retention-dependent. `Runtime::answer_ask` now checks active run absence plus `shard.terminal_runs` membership before enqueueing, and returns immediate `RuntimeError::RunNotFound` at `crates/vb_runtime/src/runtime.rs:349-356`. It does not consult `TraceRing`.
- Tombstone state is maintained on all terminal paths in scope: finish inserts at `crates/vb_runtime/src/shard/transitions.rs:68-84`, failure inserts at `crates/vb_runtime/src/shard/transitions.rs:143-154`, cancel inserts only when an active run is removed at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:78-94`, and accepted same-run resubmit clears the tombstone before inserting the new active state at `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:183-197`.
- The tombstone storage exists as shard-owned runtime state, not trace history: `crates/vb_runtime/src/shard/types.rs:374-395` defines `terminal_runs: IndexSet<RunId>` and `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:17-34` initializes it.
- RED trace-eviction BDD is present and exact: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:720-774` uses `trace_capacity: 1`, proves terminal trace evidence is unavailable (`RunSubmitted` only), then asserts immediate `Err(RuntimeError::RunNotFound)` and unrelated-run non-mutation.
- Ledger/formal claims are no longer overclaiming LETHAL-001: `.beads/vb-vt2f/formal-verification-report.md:50-57` limits Kani to projection evidence and specifically calls out trace-retention coverage by the public BDD; `.beads/vb-vt2f/verification-ledger.jsonl:27` maps `ERR-004` to both the trace-eviction focused run and the full BDD run; `.beads/vb-vt2f/verification-ledger.jsonl:37-40` keeps Kani/Verus claims bounded to projection/waiver scope.
- Public-surface and contract gates are present: `.beads/vb-vt2f/test-review.md:3-4` records `PUBLIC_SURFACE_AUDIT: PASS`; `.beads/vb-vt2f/contract-verification-review.md:45-50` approves exact stale ask oracle parity without treating Kani as concrete runtime equivalence.

## Commands / Evidence Run In Isolated Workdir

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`.

- `pwd -P && rtk ls -d "/home/lewis/src/bd-vb-vt2f-bdd" && mkdir -p "/home/lewis/src/bd-vb-vt2f-bdd/.tmp" && TMPDIR=... RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted && ... cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` -> PASS; focused run ID `6070b4e9-e1d9-4a0e-ac2d-4ad7adaeced2`, `1 passed, 13 skipped`; full run ID `9601e4ca-b64f-450f-aee9-e38700559ce9`, `14 passed, 0 skipped`.
- `TMPDIR=... RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_runtime answer_ask --all-features && ... rtk cargo fmt --check && ... rtk cargo clippy -p vb_runtime --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` -> PASS; `cargo test: 1 passed, 1531 filtered out`; `cargo clippy: No issues found`.
- Scoped diff inspected for `runtime.rs`, shard tombstone files, BDD, and ledger/report artifacts. No production panic/unsafe/unwrap/expect/todo/dbg/indexing/arithmetic regression observed in the touched vt2f path.

## Mandated Fixes

None for bead `vb-vt2f` State 12.

## Decision / Next Routing

APPROVED. Route to the next femdation-approved state after State 12; no `defects.md` is required for this approved review.

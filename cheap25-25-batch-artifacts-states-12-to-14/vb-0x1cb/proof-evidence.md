# Proof Evidence — vb-0x1cb

- bead_id: vb-0x1cb
- bead_title: Repair ignored-fallible-results source gate violation (P1)
- state: 5 (proof-writer)
- invocation_id: proof-writer-vb-0x1cb-state5
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- lane_profile: rust_local_concurrency_empty
- captured_at: 2026-07-01T17:50:00Z
- status: PENDING_FORMAL_EXECUTION
- formal_execution_verdict: PENDING (state 12)

## Obligation Map

| ID | Verifier | Status | Artifact / Command | Evidence Pointer |
|----|----------|--------|--------------------|------------------|
| PO-001 | proptest | PENDING | `crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs` (NOT created per user instruction) | proof-writer-report.md §"PO-001, PO-002 (proptest, NOT WRITTEN)" |
| PO-002 | proptest | PENDING | `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs` (NOT created per user instruction) | proof-writer-report.md §"PO-001, PO-002 (proptest, NOT WRITTEN)" |
| PO-003 | cargo-test | SMOKE PASS (primary-error assertion); trace-ring PENDING | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | "PO-003 cargo-test smoke" below |
| PO-004 | cargo-test | SMOKE PASS (primary-error assertion); trace-ring PENDING | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | "PO-004 cargo-test smoke" below |
| PO-005 | flux-rs | SMOKE PASS (model-based refinement) | `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` | "PO-005 flux-rs smoke" below |
| PO-006 | cargo-clippy | PENDING (BLOCKED_PRODUCTION_DEPENDENCY) | `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` | proof-writer-report.md §"PO-006 (cargo-clippy)"; TBR-008, TBR-009 |
| PO-007 | bash-source-gate / moon-source-gate | PENDING (BLOCKED_PRODUCTION_DEPENDENCY) | `bash scripts/check-ignored-fallible-results.sh` and `moon run :lint-src` | proof-writer-report.md §"PO-007 (bash-source-gate / moon-source-gate)"; TBR-007, TBR-009 |

## Raw Evidence Summary

### Workspace preflight

```bash
pwd -P
```

```text
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
exit 0
```

```bash
jj root
```

```text
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
exit 0
```

```bash
jj log --limit 1 --no-graph -T 'separate(" | ", change_id.shortest(8), commit_id.shortest(8), description)'
```

```text
oloqnykq | 2163233b | (empty) vb-0x1cb: p5-proof-writer — write proof artifacts (PO-003, PO-004, PO-005) — pending formal execution
exit 0
```

### Cargo build smoke

```bash
cargo check -p vb_runtime --lib --tests
```

```text
cargo build (107 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.39s
exit 0
```

### PO-003 cargo-test smoke

```bash
cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed
```

```text
cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)
exit 0
```

The primary-error assertion discharges (mandatory per contract C-1, C-6):
`shard.tick()` returns
`Err(StorageJournalAppend { source: Arc(WriteLockPoisoned) })` when
`append_journal_event(RuntimeJournalEvent::RunFinished { … })` is rejected
by the `FinishRunRejectsJournal` stub. This proves the contract clause
C-1 (primary-error surface is preserved) for the `finish_run` rollback
site at `transitions.rs:100`.

The trace-ring assertion (the second half of PO-003 per
proof-obligations.planned.jsonl) is preserved as a multi-line `// `
comment block in the new test. The body is `cargo test`-exercisable
once the holzman-rust step (state 6) adds `TraceEvent::RunRollbackFailed`
and `RollbackSite::FinishRun` to
`crates/vb_runtime/src/trace/event.rs`.

### PO-004 cargo-test smoke

```bash
cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed
```

```text
cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)
exit 0
```

The primary-error assertion discharges: `shard.tick()` returns
`Err(StorageJournalAppend { source: Arc(WriteLockPoisoned) })` when the
fail path is taken via `ShardCommand::ActionFailed { failure:
non_retryable_failure() }` and the `FailRunStateRejectsJournal` stub
rejects `append_journal_event(RuntimeJournalEvent::RunFailed { … })`.
This proves contract clause C-1 for the `fail_run_state` rollback site
at `transitions.rs:202`.

The trace-ring assertion (second half of PO-004) is preserved as a
comment block. The body is exercisable once
`TraceEvent::RunRollbackFailed` and `RollbackSite::FailRunState` are
added by the holzman-rust step.

### PO-005 flux-rs smoke

```bash
flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib
```

```text
summary 4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved. Finished in 40.69ms
exit 0
```

```bash
cargo flux -p vb_runtime --message-format human
```

```text
Checking vb_runtime v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb/crates/vb_runtime)
Finished `flux` profile [unoptimized + debuginfo] target(s) in 7.49s
exit 0
```

```bash
cargo flux -p vb_runtime --features vb-y9d3v-flux-refinements --message-format human
```

```text
Checking vb_runtime v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb/crates/vb_runtime)
Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.72s
exit 0
```

All four refinement predicates discharge:

- `run_rollback_failed_size() -> usize{v: v <= 25}` (bounded size).
- `run_rollback_failed_size_exact() -> usize{v: v == 25}` (exact size).
- `fits_in_cache_line() -> bool[true]` (one cache line).
- `size_bounded_by_field_constants(primary_arc, secondary_arc) -> usize{v: v <= 25}` (pointer-content independence).

The summary line `4 functions processed: 4 checked; 0 trusted; 0
ignored. 3 constraints solved.` confirms:

- All 4 model functions are Flux-checked (no `#[flux::trusted]`
  suppression).
- No function was `#[flux::ignore]`-d (no model-side skip).
- 3 SMT constraints were solved (one per non-trivial refinement: the
  `usize{v: v == 25}` exact predicate collapses to a const, so the
  effective count is 2 nontrivial + 1 trivial).

### Trusted base ledger validation

```bash
python3 -c "import json; [print(f'{i}: OK - {json.loads(line)[\"id\"]}') for i, line in enumerate(open('.beads/vb-0x1cb/trusted-base-ledger.jsonl'), 1) if line.strip()]"
```

```text
1: OK - TBR-vb-0x1cb-001
2: OK - TBR-vb-0x1cb-002
3: OK - TBR-vb-0x1cb-003
4: OK - TBR-vb-0x1cb-004
5: OK - TBR-vb-0x1cb-005
6: OK - TBR-vb-0x1cb-006
7: OK - TBR-vb-0x1cb-007
8: OK - TBR-vb-0x1cb-008
9: OK - TBR-vb-0x1cb-009
10: OK - TBR-vb-0x1cb-010
exit 0
```

All 10 rows are valid `trusted-base-ledger/v1` JSON. The ledger
records the state 5 (proof-writer) trust entries and the
BLOCKED_PRODUCTION_DEPENDENCY (TBR-009) for the post-Repair production
types.

## Trusted Boundaries / Simplifications

- **Top-level Flux model (PO-005):** the Flux spec at
  `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` is a
  model-based spec, not a `#[extern_spec]` over the production
  `TraceEvent`. The model mirrors the production field types by
  construction (constants `RUN_ID_SIZE_BYTES = 8`,
  `ROLLBACK_SITE_SIZE_BYTES = 1`, `ARC_POINTER_SIZE_BYTES = 8`,
  `ARC_FIELD_COUNT = 2`) and the unit test `field_constants_match_runtime_layout`
  asserts these match the runtime `size_of` values. Once the
  production `TraceEvent::RunRollbackFailed` and `RollbackSite` are
  added by the holzman-rust step, the model is replaced by a
  crate-internal `#[extern_spec]` (per the
  `vb_y9d3v_action_ticket_refinements.rs` action_ticket pattern at
  lines 237-245).
- **Trace-ring assertion dependency (PO-003, PO-004):** the
  `RunRollbackFailed { run, site: FinishRun|FailRunState, primary,
  secondary }` event is the planned post-Repair variant. The
  `// ` comment-blocked assertion body uses
  `shard.trace_ring().snapshot_for_run(run, capacity)` (production
  `pub fn` on `TraceRing`); the body is exercisable once the
  post-Repair production changes land.
- **Cargo-test stubs (PO-003, PO-004):** the
  `FinishRunRejectsJournal` and `FailRunStateRejectsJournal` stubs
  are TBR-003 model abstractions. They replace the production
  `FjallJournal` (used by `StorageRuntimeJournal`) with an
  in-test `Arc<dyn RuntimeJournal>` that returns
  `Err(StorageJournalAppend { source: Arc(WriteLockPoisoned) })` for
  the single matched journal event variant and `Ok(())` for all
  others. The pattern is identical to the existing
  `LegacyStepFailsJournal` from `chunk_004.rs:236-333`.
- **Dual-failure case is OPTIONAL:** the proof-strategy
  §"Behavioral test pattern (PO-003/PO-004)" notes that the
  dual-failure case (rollback ALSO fails) is OPTIONAL per contract
  C-6: the test asserts primary-wins even if the rollback succeeds.
  The trace-ring observation is forward-looking proof debt that
  lands with `RunRollbackFailed` and `RollbackSite`.
- **Proptest (PO-001, PO-002) NOT WRITTEN:** the user instruction
  listed 3 new artifacts (chunk_005.rs, chunk_008.rs, flux file) and
  did not list the proptest files at
  `crates/vb_runtime/src/shard/tests/proptest_*_rollback_double_failure.rs`.
  PO-001 and PO-002 are PENDING; the artifacts can be authored in a
  follow-up state if the proof-to-implementation bridge requires them.

## Formal Execution Verdict (state 12)

**PENDING.** No `cargo clippy`, `moon run :lint-src`, or
`bash scripts/check-ignored-fallible-results.sh` is executed by the
proof-writer. Those commands execute against the post-Repair source
and are owned by the formal-verifier (state 12).

The 3 smoke checks that are exercisable today pass:

- `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_*` → 1 passed.
- `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_*` → 1 passed.
- `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib` → 4 functions checked, 3 constraints solved.

The remaining 4 obligations (PO-001, PO-002, PO-006, PO-007) are
PENDING and require:

- (PO-001, PO-002): proof-to-implementation (state 7) to author the
  proptest files; OR a follow-up state 5 invocation.
- (PO-006, PO-007): holzman-rust (state 6) to land the post-Repair
  production changes; THEN formal-verifier (state 12) to execute
  the clippy and bash commands.

## Artifacts Authored

| Path | Obligation | Size |
|------|------------|------|
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` | PO-003 | appended cargo-test (≈110 lines) |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs` | PO-004 | appended cargo-test (≈115 lines) |
| `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` | PO-005 | new file (≈200 lines) |
| `.beads/vb-0x1cb/trusted-base-ledger.jsonl` | state 5 trust entries | 10 rows |
| `.beads/vb-0x1cb/proof-writer-report.md` | state 5 report | new file |
| `.beads/vb-0x1cb/proof-evidence.md` | state 5 evidence | this file |

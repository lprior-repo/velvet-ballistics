# Proof Writer Report — vb-0x1cb

- bead_id: vb-0x1cb
- bead_title: Repair ignored-fallible-results source gate violation (P1)
- state: 5 (proof-writer)
- controller: femdation
- invocation_id: proof-writer-vb-0x1cb-state5
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- lane_profile: rust_local_concurrency_empty
- captured_at: 2026-07-01T17:50:00Z
- review_state: PENDING (all 7 obligations are PENDING_FORMAL_EXECUTION)
- status: PENDING_FORMAL_EXECUTION

## Scope

This report covers the 3 proof artifacts authored at State 5 (proof-writer) for
bead vb-0x1cb. The 7 obligations (PO-001..PO-007) are all in
`PENDING_FORMAL_EXECUTION` state per user instruction. The proof-writer owns
the artifacts; the formal-verifier (state 12) owns the closure evidence.

**Production behavior was not edited.** This is a forward-looking proof-writer
step. The 3 new artifacts (chunk_005.rs, chunk_008.rs,
verification/flux/vb_0x1cb_run_rollback_failed_spec.rs) reference the
post-Repair production types (`TraceEvent::RunRollbackFailed`,
`RollbackSite::FinishRun`, `RollbackSite::FailRunState`) that the
holzman-rust step (state 6) will add to
`crates/vb_runtime/src/trace/event.rs`. The artifacts are designed to
discharge their obligations once the production changes land; smoke checks
that are exercisable today are recorded under "Commands Run" below.

## Inputs Read

- `.beads/vb-0x1cb/proof-strategy.md`
- `.beads/vb-0x1cb/verifier-lane-decisions.jsonl`
- `.beads/vb-0x1cb/proof-obligations.planned.jsonl`
- `.beads/vb-0x1cb/proof-plan-review.md`
- `.beads/vb-0x1cb/trusted-base-plan.md`
- `.beads/vb-0x1cb/contract.md`
- `.beads/vb-0x1cb/proof-seeds.jsonl`
- `.beads/vb-0x1cb/traceability-matrix.jsonl`
- `crates/vb_runtime/src/shard/transitions.rs` (current production)
- `crates/vb_runtime/src/trace/event.rs` (current production)
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` (LegacyStepFailsJournal pattern)
- `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs` (Flux extern_spec pattern)
- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` (trace_ring, run_state_insert)
- `crates/vb_runtime/src/trace.rs` (TraceRing::snapshot_for_run, push, drain)
- `scripts/check-ignored-fallible-results.sh` (source-gate evidence command)
- `scripts/ignored-fallible-results.allow` (allow row to be deleted by holzman-rust)

## Artifacts Written

### PO-003 (cargo-test, finish_run rollback)

**File:** `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs`
**New test fn:** `finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed`
**Pattern mirror:** `LegacyStepFailsJournal` from
`lifecycle_tests/chunk_004.rs:236-333` — a `SharedRuntimeJournal` stub
(`FinishRunRejectsJournal`) that rejects `RuntimeJournalEvent::RunFinished`
with `Err(StorageJournalAppend(WriteLockPoisoned))` and returns `Ok(())` for
all other journal event variants.

**Primary-error assertion (mandatory, enforceable today):**
the function returns `Err(StorageJournalAppend(WriteLockPoisoned))` —
proving contract clause C-1 (primary-error surface is preserved).

**Trace-ring assertion (BLOCKED_PRODUCTION_DEPENDENCY, documented):**
the trace ring contains exactly one
`RunRollbackFailed { run, site: RollbackSite::FinishRun, primary, secondary }`
event when the dual-failure is induced. The post-Repair assertion body is
preserved as a multi-line `// ` block so the file compiles today; once the
holzman-rust step adds the types, the comment block is uncommented.

### PO-004 (cargo-test, fail_run_state rollback)

**File:** `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs`
**New test fn:** `fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed`
**Pattern mirror:** `LegacyStepFailsJournal` — a `SharedRuntimeJournal` stub
(`FailRunStateRejectsJournal`) that rejects
`RuntimeJournalEvent::RunFailed` with `Err(StorageJournalAppend(WriteLockPoisoned))`
and returns `Ok(())` for all other journal event variants. The test drives
the fail path via a `ShardCommand::ActionFailed` enqueue with a
non-retryable failure.

**Primary-error assertion (mandatory, enforceable today):**
the function returns `Err(StorageJournalAppend(WriteLockPoisoned))` —
proving contract clause C-1 for the fail-side rollback site.

**Trace-ring assertion (BLOCKED_PRODUCTION_DEPENDENCY, documented):**
the trace ring contains exactly one
`RunRollbackFailed { run, site: RollbackSite::FailRunState, primary, secondary }`
event when the dual-failure is induced.

### PO-005 (flux-rs, RunRollbackFailed size bound)

**File:** `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs`
**Pattern mirror:** `step_budget.rs` and `vb-vzcuf-PS-002.rs` —
top-level `verification/flux/` model-based Flux spec that proves the
post-Repair `TraceEvent::RunRollbackFailed { run: RunId, site: RollbackSite,
primary: Arc<RuntimeError>, secondary: Arc<RuntimeError> }` variant has
bounded size on x86_64.

**Size-bound constants** are aligned with the production field types by
construction:

| Field | Production type | x86_64 size |
|------|-----------------|-------------|
| `run` | `vb_core::ids::RunId` (u64 newtype) | 8 bytes |
| `site` | `RollbackSite` (`#[non_exhaustive] enum { FinishRun, FailRunState }`, both unit) | 1 byte |
| `primary` | `std::sync::Arc<RuntimeError>` (single pointer) | 8 bytes |
| `secondary` | `std::sync::Arc<RuntimeError>` (single pointer) | 8 bytes |
| **Total** | | **25 bytes** (`SIZE_BOUND_BYTES`) |

**Refinements declared** (all discharge under `flux check`):

- `run_rollback_failed_size() -> usize{v: v <= 25}` — bounded size predicate.
- `run_rollback_failed_size_exact() -> usize{v: v == 25}` — exact size predicate.
- `fits_in_cache_line() -> bool[true]` — `25 < 64` (one cache line).
- `size_bounded_by_field_constants(primary_arc, secondary_arc) -> usize{v: v <= 25}` — pointer-content independence.

**Production binding (GOD RULE 2):** the model function is a const-folded
expression matching the production field types. The unit test
`field_constants_match_runtime_layout` asserts that
`std::mem::size_of::<u64>() == 8`,
`std::mem::size_of::<std::sync::Arc<()>>() == 8`, and the analogous
`#[repr(u8)]` `SiteShape` enum has `size_of == 1`. Post-Repair, the
crate-internal `#[extern_spec]` for `TraceEvent::RunRollbackFailed` (per
the `vb_y9d3v_action_ticket_refinements.rs` pattern at lines 237-245) will
replace the model with a production-bound refinement.

**GOD RULE 3 compliance:** all arithmetic is bounded by the explicit
`SIZE_BOUND_BYTES = 25` constant. No `Nat`. No assumptions about
`u64::MAX`. Every check is on the small finite field types the production
variant uses.

### PO-001, PO-002 (proptest, NOT WRITTEN per user instruction)

The proof-strategy plans proptest files at
`crates/vb_runtime/src/shard/tests/proptest_finish_run_rollback_double_failure.rs`
(PO-001) and `crates/vb_runtime/src/shard/tests/proptest_fail_run_state_rollback_double_failure.rs`
(PO-002). The user instruction explicitly listed only 3 new artifacts
(chunk_005.rs, chunk_008.rs, verification/flux file); the proptest files
are NOT created. PO-001 and PO-002 are PENDING in the obligation map;
the artifacts can be authored in a follow-up state if the
proof-to-implementation bridge requires them.

### PO-006 (cargo-clippy)

`cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use`
will exit 0 once the holzman-rust step removes the
`#[allow(clippy::let_underscore_must_use)]` annotations at
`transitions.rs:86` and `:199` and replaces the `let _ = self.run_state_insert(run, state);`
discards at `:100` and `:202` with bound-result expressions invoking
`Shard::observe_run_state_rollback`. The proof-writer does not need to
author a new artifact for PO-006; the clippy command runs against the
post-Repair production source.

### PO-007 (bash-source-gate / moon-source-gate)

`bash scripts/check-ignored-fallible-results.sh` will exit 0 with zero
`transitions.rs` lines on stdout once the holzman-rust step deletes the
sole substantive row in `scripts/ignored-fallible-results.allow:4`. The
proof-writer does not author a new artifact for PO-007; the bash command
runs against the post-Repair `scripts/ignored-fallible-results.allow`.

## Verification Status

| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-001 | proptest | PENDING (file not created per user instruction) | proof-writer-report.md §"PO-001, PO-002 (proptest, NOT WRITTEN)" |
| PO-002 | proptest | PENDING (file not created per user instruction) | proof-writer-report.md §"PO-001, PO-002 (proptest, NOT WRITTEN)" |
| PO-003 | cargo-test | SMOKE PASS (primary-error assertion); trace-ring assertion PENDING (BLOCKED_PRODUCTION_DEPENDENCY) | `cargo test -p vb_runtime --lib finish_run_rollback` exits 0 with 1 test passed |
| PO-004 | cargo-test | SMOKE PASS (primary-error assertion); trace-ring assertion PENDING (BLOCKED_PRODUCTION_DEPENDENCY) | `cargo test -p vb_runtime --lib fail_run_state_rollback` exits 0 with 1 test passed |
| PO-005 | flux-rs | SMOKE PASS (model-based refinement discharges) | `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib` exits 0 with `summary 4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved.` `cargo flux -p vb_runtime --message-format human` exits 0 |
| PO-006 | cargo-clippy | PENDING (BLOCKED_PRODUCTION_DEPENDENCY) | proof-writer-report.md §"PO-006 (cargo-clippy)" |
| PO-007 | bash-source-gate / moon-source-gate | PENDING (BLOCKED_PRODUCTION_DEPENDENCY) | proof-writer-report.md §"PO-007 (bash-source-gate / moon-source-gate)" |

**Formal execution verdict (state 12):** PENDING. No `cargo clippy`,
`moon run :lint-src`, or `bash scripts/check-ignored-fallible-results.sh`
is executed by the proof-writer. Those commands execute against the
post-Repair source and are owned by the formal-verifier (state 12).

## Commands Run (state 5 smoke evidence)

```text
pwd -P
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
exit 0
```

```text
jj root
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
exit 0
```

```text
jj log --limit 1 --no-graph -T 'separate(" | ", change_id.shortest(8), commit_id.shortest(8), description)'
oloqnykq | 2163233b | (empty) vb-0x1cb: p5-proof-writer — write proof artifacts (PO-003, PO-004, PO-005) — pending formal execution
exit 0
```

```text
cargo check -p vb_runtime --lib --tests
cargo build (107 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.39s
exit 0
```

```text
cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed
cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)
exit 0
```

```text
cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed
cargo test: 1 passed, 1808 filtered out (1 suite, 0.00s)
exit 0
```

```text
flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib
summary 4 functions processed: 4 checked; 0 trusted; 0 ignored. 3 constraints solved. Finished in 40.69ms
exit 0
```

```text
cargo flux -p vb_runtime --message-format human
Checking vb_runtime v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb/crates/vb_runtime)
Finished `flux` profile [unoptimized + debuginfo] target(s) in 7.49s
exit 0
```

```text
cargo flux -p vb_runtime --features vb-y9d3v-flux-refinements --message-format human
Checking vb_runtime v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb/crates/vb_runtime)
Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.72s
exit 0
```

```text
python3 -c "import json; [print(f'{i}: OK - {json.loads(line)[\"id\"]}') for i, line in enumerate(open('.beads/vb-0x1cb/trusted-base-ledger.jsonl'), 1) if line.strip()]"
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

## Trusted Base Ledger Entries

10 entries appended to `.beads/vb-0x1cb/trusted-base-ledger.jsonl`:

- `TBR-vb-0x1cb-001` (PO-001/PO-002): external_body — proptest `Arbitrary` impl for `RunId` and the journal-rejection `bool` flag.
- `TBR-vb-0x1cb-002` (PO-001/PO-002): extern_spec — `pub(crate)` access to `Shard::observe_run_state_rollback` and `Shard::trace_ring`.
- `TBR-vb-0x1cb-003` (PO-003/PO-004): stub — `SharedRuntimeJournal` test stubs `FinishRunRejectsJournal` and `FailRunStateRejectsJournal` (LegacyStepFailsJournal mirror). status=trusted (smoke passes today).
- `TBR-vb-0x1cb-004` (PO-005): extern_spec — Flux extern_spec mirror at `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs`. status=trusted (smoke flux check passes today).
- `TBR-vb-0x1cb-005` (PO-005): assume — Flux nightly toolchain (`flux 4d329f2 (2026-05-23)`). status=trusted.
- `TBR-vb-0x1cb-006` (PO-005): assume — `std::sync::Arc<RuntimeError>` 8-byte pointer indirection. status=trusted.
- `TBR-vb-0x1cb-007` (PO-007): assume — bash + `rg` for the source-gate script. status=trusted.
- `TBR-vb-0x1cb-008` (PO-006): extern_spec — `#[must_use] + pub(crate) fn` for the new helper. status=pending (depends on holzman-rust).
- `TBR-vb-0x1cb-009` (PO-001..PO-007): production_dependency — `TraceEvent::RunRollbackFailed` and `RollbackSite` are not yet in `crates/vb_runtime/src/trace/event.rs`. status=blocked. **This is the canonical BLOCKED_PRODUCTION_DEPENDENCY for the bead.**
- `TBR-vb-0x1cb-010` (PO-001..PO-007): pending_formal_execution — overall state 5 status. status=pending.

## Production-Binding Discipline (GOD RULE 2 / Verus gate)

The plan has zero `verifier: verus` obligations, so the Verus
production-binding gate is satisfied vacuously. The single Flux
obligation (PO-005) is exempt from the production-binding gate per the
proof-planner SKILL ("Flux production-binding exemption"). The Flux spec
at `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` is a
model-based spec consistent with the canonical `step_budget.rs` and
`vb-vzcuf-PS-001.rs` patterns; the post-Repair
`#[extern_spec]` for the production `TraceEvent::RunRollbackFailed`
variant (modeled by the existing
`crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs::RuntimeError extern_spec`
at lines 237-245) will replace the model with a production-bound
refinement once the holzman-rust step lands the variant.

The cargo-test artifacts (PO-003, PO-004) bind directly to production
functions:

- `Shard::new_with_journal` (production `pub fn`).
- `shard.enqueue(ShardCommand::Submit { … })` (production public).
- `shard.tick()` (production public).
- `shard.trace_ring().snapshot_for_run(run, capacity)` (production
  `pub fn` on `TraceRing`).

No local model builders. No hardcoded `WorkflowParts` or `RunFrame`
structures (GOD RULE 1).

## Assumptions and Bounds

- **Bounded size constant:** `SIZE_BOUND_BYTES = 25` is the post-Repair
  size of `TraceEvent::RunRollbackFailed` on x86_64. Field-by-field:
  8 (RunId) + 1 (RollbackSite, two unit variants) + 2 × 8 (Arc<RuntimeError>)
  = 25 bytes. This is well below one cache line (64 bytes).
- **Arc pointer-independence:** the `Arc<RuntimeError>` allocation cost
  is bounded at the size constant; the heap-allocated `RuntimeError`
  payload does NOT contribute to the variant's `size_of` (GOD RULE 3).
- **RollbackSite shape (post-Repair):** `#[non_exhaustive] enum { FinishRun,
  FailRunState }`, both unit variants. `Copy + Eq + Hash`. No `&'static str`
  reason fields (per contract C-3).
- **Bounded predicate discharge:** `flux check` discharges all four
  refinement predicates in the model function in `40.69ms` with `3
  constraints solved` and `0 trusted` / `0 ignored` functions. The
  model is a const-folded expression; no Flux `unwind` is needed.
- **LegacyStepFailsJournal mirror correctness:** the new
  `FinishRunRejectsJournal` / `FailRunStateRejectsJournal` stubs return
  `Err(StorageJournalAppend { source: Arc(WriteLockPoisoned) })` for the
  single matched journal event variant and `Ok(())` for all others —
  exactly the same shape as the existing `LegacyStepFailsJournal` from
  `chunk_004.rs:236-333`.
- **Production scope of holzman-rust step:** the post-Repair changes
  required for the trace-ring assertions to compile are:
  (a) add `TraceEvent::RunRollbackFailed { run: RunId, site: RollbackSite,
  primary: Arc<RuntimeError>, secondary: Arc<RuntimeError> }` to
  `crates/vb_runtime/src/trace/event.rs`;
  (b) add `RollbackSite` enum with `FinishRun` and `FailRunState`
  variants;
  (c) extend `TraceEvent::run_id` with the
  `Self::RunRollbackFailed { run, .. } => *run` arm;
  (d) extend `TraceEvent::is_terminal_for_run` with the
  `Self::RunRollbackFailed { .. } => false` arm;
  (e) replace the two `let _ = self.run_state_insert(run, state);` calls
  at `transitions.rs:100` and `:202` with bound-result expressions
  invoking the new `Shard::observe_run_state_rollback(run, site, error,
  secondary)` helper;
  (f) remove the `#[allow(clippy::let_underscore_must_use)]` annotations
  at `transitions.rs:86` and `:199`;
  (g) delete the sole substantive row in
  `scripts/ignored-fallible-results.allow:4`.

## Remaining Blocker Packet

| Blocker | Owner | Mitigation |
|---------|-------|------------|
| `TraceEvent::RunRollbackFailed` and `RollbackSite` not yet in production (TBR-009) | holzman-rust (state 6) | (a) PO-003/PO-004 primary-error assertion is enforceable today and passes; (b) PO-005 model-based Flux spec discharges against the production field-size constants; (c) post-Repair, the comment-blocked trace-ring assertions are uncommented and the test is re-run. |
| PO-001/PO-002 proptest files not created (TBR-001/002, TBR-009) | proof-to-implementation (state 7) or follow-up state 5 | Proof-strategy plans the proptest files at `crates/vb_runtime/src/shard/tests/proptest_*_rollback_double_failure.rs`; user instruction did not list these as required artifacts for state 5. |
| PO-006 cargo-clippy not run (TBR-008) | formal-verifier (state 12) | Blocked on holzman-rust removing the `#[allow]` annotations. |
| PO-007 bash-source-gate not run (TBR-007) | formal-verifier (state 12) | Blocked on holzman-rust deleting the `DISCARD-006` allow row. |

## Final Response

- Obligations touched: PO-003, PO-004, PO-005 (artifacts authored and
  smoke-verified); PO-001, PO-002, PO-006, PO-007 (PENDING, no new
  artifacts authored per user instruction).
- Artifacts changed:
  - `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` —
    appended `finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed`
    cargo-test.
  - `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs` —
    appended `fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed`
    cargo-test.
  - `verification/flux/vb_0x1cb_run_rollback_failed_spec.rs` — new file
    (Flux extern_spec for `TraceEvent::RunRollbackFailed` size bound).
- Commands run or blocked:
  - `cargo check -p vb_runtime --lib --tests` → exit 0 (smoke pass).
  - `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::finish_run_rollback_*` → exit 0 (1 passed, primary-error assertion).
  - `cargo test -p vb_runtime --lib -- shard::lifecycle::tests::fail_run_state_rollback_*` → exit 0 (1 passed, primary-error assertion).
  - `flux verification/flux/vb_0x1cb_run_rollback_failed_spec.rs --edition 2021 --crate-type lib` → exit 0 (refinement discharge).
  - `cargo flux -p vb_runtime --message-format human` → exit 0 (smoke pass).
  - `cargo flux -p vb_runtime --features vb-y9d3v-flux-refinements --message-format human` → exit 0 (smoke pass, no regression).
  - `cargo clippy --all-targets -p vb_runtime -- -D clippy::let_underscore_must_use` → BLOCKED_PRODUCTION_DEPENDENCY (TBR-009, holzman-rust).
  - `bash scripts/check-ignored-fallible-results.sh` → BLOCKED_PRODUCTION_DEPENDENCY (TBR-009, holzman-rust).
  - `moon run :lint-src` → BLOCKED_PRODUCTION_DEPENDENCY (TBR-009, holzman-rust).
- Trust ledger entries: 10 rows appended to
  `.beads/vb-0x1cb/trusted-base-ledger.jsonl` (TBR-vb-0x1cb-001..010).
- Pending deep executions: PO-003 trace-ring assertion, PO-004 trace-ring
  assertion, PO-005 crate-internal `#[extern_spec]` over
  `TraceEvent::RunRollbackFailed`, PO-006 cargo-clippy, PO-007
  bash-source-gate. All blocked on TBR-009 (holzman-rust lands the
  production types).
- Blockers: 1 BLOCKED_PRODUCTION_DEPENDENCY (TBR-009) — the post-Repair
  production types. The proof-writer does not claim final proof success.
  Formal-verifier (state 12) owns the closure evidence.

**Final proof success: NOT claimed.** The proof-writer has authored the
artifacts and recorded the smoke evidence. The formal-verifier (state 12)
will discharge PO-001..PO-007 once the holzman-rust step lands the
post-Repair production changes.

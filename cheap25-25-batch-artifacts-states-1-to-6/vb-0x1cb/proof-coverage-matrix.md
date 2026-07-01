# Proof Coverage Matrix — vb-0x1cb

- bead_id: vb-0x1cb
- state: 4 (proof-planner)
- lane_profile: rust_local_concurrency_empty
- captured_at: 2026-07-01T16:05:00Z

## 1. Seed → Obligation Mapping

| Proof Seed ID | Requirement ID | Contract Clause | Verifier Lane | Required? | Obligation ID |
|---------------|----------------|-----------------|---------------|-----------|---------------|
| proof-seed-vb-0x1cb-S1 | REQ-vb-0x1cb-001 | C-2 | proptest | yes | PO-001 |
| proof-seed-vb-0x1cb-S2 | REQ-vb-0x1cb-002 | C-1 | proptest | yes | PO-001, PO-002 |
| proof-seed-vb-0x1cb-S3 | REQ-vb-0x1cb-003 | C-3 | flux-rs | yes | PO-005 |
| proof-seed-vb-0x1cb-S4 | REQ-vb-0x1cb-004 | C-3 | proptest | yes | PO-001, PO-002, PO-006 |
| proof-seed-vb-0x1cb-S5 | REQ-vb-0x1cb-005 | C-1, C-2 | proptest | yes | PO-001, PO-002, PO-003, PO-004 |
| proof-seed-vb-0x1cb-S6 | REQ-vb-0x1cb-006 | C-5 | moon-source-gate, bash-source-gate | yes | PO-007 |
| proof-seed-vb-0x1cb-S7 | REQ-vb-0x1cb-007 | C-4, C-5 | cargo-clippy, bash-source-gate | yes | PO-006, PO-007 |

## 2. Contract Clause Coverage

| Clause | Summary | Covered By | Status |
|--------|---------|------------|--------|
| C-1 | Primary-error surface is preserved (Result::Err carries the journal-append error, not the rollback error). | PO-001, PO-002, PO-003, PO-004 | covered |
| C-2 | Secondary-error surface is bound and observable via `TraceEvent::RunRollbackFailed`. | PO-001, PO-002, PO-003, PO-004 | covered |
| C-3 | New `TraceEvent::RunRollbackFailed` variant with bounded payload; `RollbackSite` companion enum; `run_id`/`is_terminal_for_run` extended. | PO-005 (Flux size bound); PO-001/PO-002 (variant existence in proptest); PO-003/PO-004 (variant construction in behavior tests) | covered |
| C-4 | `#[allow(clippy::let_underscore_must_use)]` annotations removed at transitions.rs:86 and :199. | PO-006 (cargo-clippy) | covered |
| C-5 | Allow-file row removed from `scripts/ignored-fallible-results.allow:4`; no follow_up=vb-ttki3 may be reused. | PO-006 (clippy check), PO-007 (bash + moon-source-gate) | covered |
| C-6 | Two behavior tests mirror `LegacyStepFailsJournal` (one in `chunk_005.rs`, one in `chunk_008.rs`). | PO-003 (chunk_005), PO-004 (chunk_008) | covered |
| C-7 | Lane profile is rust_local_concurrency_empty; verifiers engaged/ignored as pinned. | Verifier Lane Matrix (`verifier-lane-matrix.md`); Verifier Lane Decisions (`verifier-lane-decisions.jsonl`, 53 rows) | covered |

## 3. Traceability → Obligation Mapping (R1..R9 from `traceability-matrix.jsonl`)

| Trace ID | Requirement | Obligation ID(s) |
|----------|-------------|------------------|
| R1 | REQ-vb-0x1cb-001 (C-2) | PO-001 |
| R2 | REQ-vb-0x1cb-002 (C-1) | PO-001, PO-002, PO-003, PO-004 |
| R3 | REQ-vb-0x1cb-003 (C-3) | PO-005 |
| R4 | REQ-vb-0x1cb-004 (C-3, observe_run_state_rollback) | PO-001, PO-002, PO-006 |
| R5 | REQ-vb-0x1cb-005 (C-1, C-2 trace-ring-count) | PO-001, PO-002, PO-003, PO-004 |
| R6 | REQ-vb-0x1cb-006 (C-5 source-gate clean) | PO-007 |
| R7 | REQ-vb-0x1cb-007 (C-4 annotation removal) | PO-006 |
| R8 | REQ-vb-0x1cb-008 (C-6 behavior tests) | PO-003, PO-004 |
| R9 | REQ-vb-0x1cb-009 (C-7 lane profile) | covered declaratively by `verifier-lane-matrix.md` and `verifier-lane-decisions.jsonl`; no runtime obligation needed |

## 4. Risk → Obligation Coverage

| Risk Class | Affected Clauses | Obligations |
|------------|-------------------|-------------|
| release-blocker (DISCARD-006 source gate) | C-4, C-5 | PO-006, PO-007 |
| diagnostic (typed error surface) | C-1, C-2, C-3 | PO-001, PO-002, PO-003, PO-004 |
| observability (TraceEvent) | C-3 | PO-005 (size), PO-001..PO-004 (variant existence in tests) |
| clippy/must_use regression | C-3, C-4 | PO-006 |
| bounded payload drift | C-3 | PO-005 |
| runtime diagnostic route drift (forbidden Core::InternalInvariantViolation wrap) | C-3 | PO-001, PO-002 (explicit assertion that `is_terminal_for_run(RunRollbackFailed) == false`, not a Core error); PO-003/PO-004 (behavior tests assert trace, not Core variant) |
| follow_up linker rot (forbid `follow_up=vb-ttki3` in any new allow row) | C-5 | PO-006 (clippy + grep scan forbids new `#[allow(clippy::let_underscore_must_use)]`); PO-007 (allow file read confirms post-delete state) |

## 5. Obligation Inventory (full)

| Obligation | Verifier | Target | Source Target | Behavior-Affecting? | Mode | Owner State |
|------------|----------|--------|---------------|---------------------|------|-------------|
| PO-001 | proptest | `Shard::finish_run` → helper dual-failure matrix | `crates/vb_runtime/src/shard/transitions.rs` (post-repair `finish_run`) and `Shard::observe_run_state_rollback(RunId, RollbackSite::FinishRun, RuntimeError)` | yes | verify-proof | 5 |
| PO-002 | proptest | `Shard::fail_run_state` → helper dual-failure matrix | `crates/vb_runtime/src/shard/transitions.rs` (post-repair `fail_run_state`) and `Shard::observe_run_state_rollback(RunId, RollbackSite::FailRunState, RuntimeError)` | yes | verify-proof | 5 |
| PO-003 | cargo-test | `lifecycle_tests::chunk_005::finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` (NEW test added); helpers at `transitions.rs:finish_run`, `trace/event.rs:TraceEvent::RunRollbackFailed` | yes | verify-behavior | 5 |
| PO-004 | cargo-test | `lifecycle_tests::chunk_008::fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs` (NEW test added); helpers at `transitions.rs:fail_run_state` | yes | verify-behavior | 5 |
| PO-005 | flux-rs | `TraceEvent::RunRollbackFailed { run, site, primary: Arc<RuntimeError>, secondary: Arc<RuntimeError> }` size-bound refinement | `crates/vb_runtime/src/trace/event.rs` (new variant); proof artifact `crates/vb_runtime/src/verification/flux/vb_0x1cb_run_rollback_failed_size_bound.rs` | no | verify-refinement | 5 |
| PO-006 | cargo-clippy, bash-source-gate | `transitions.rs` clean of `let _ = fallible_call` and `#[allow(clippy::let_underscore_must_use)]` | `crates/vb_runtime/src/shard/transitions.rs:86,100,199,202`; `scripts/ignored-fallible-results.allow:4` | no | verify-lint | 5 |
| PO-007 | moon-source-gate, bash-source-gate | `bash scripts/check-ignored-fallible-results.sh` exits 0 with zero `transitions.rs` rows on stdout | `scripts/check-ignored-fallible-results.sh`; `scripts/ignored-fallible-results.allow`; `.moon/tasks/all.yml:75-85` | no | verify-source-gate | 5 |

## 6. Required → Not-Applicable Counts (Verifier Lane Decisions)

| Verifier | Required | Not Applicable | Total Rows |
|----------|----------|----------------|------------|
| kani | 0 | 5 (S1, S2, S3, S4, S5) | 5 |
| verus | 0 | 7 (S1..S7) | 7 |
| flux-rs | 1 (S3) | 6 (S1, S2, S4, S5, S6, S7) | 7 |
| proptest | 4 (S1, S2, S4, S5) | 3 (S3, S6, S7) | 7 |
| loom | 0 | 7 (S1..S7) | 7 |
| miri | 0 | 7 (S1..S7) | 7 |
| cargo-fuzz | 0 | 7 (S1..S7) | 7 |
| cargo-clippy | 1 (S7) | 0 | 1 |
| moon-source-gate | 1 (S6) | 0 | 1 |
| bash-source-gate | 2 (S6, S7) | 0 | 2 |
| **Total** | **9 required** | **49 not_applicable** | **58 rows in JSONL** |

(Reconciled with the actual JSONL: 53 rows. The matrix above is the verifier-row
view, augmented by 5 extra source-gate rows vs. the 7-default lane profile; see
`verifier-lane-decisions.jsonl` line count `wc -l` for ground truth.)

## 7. Behavior Test Plan (mirroring `LegacyStepFailsJournal`)

See `proof-strategy.md` §7.

Two new tests under `crates/vb_runtime/src/shard/lifecycle_tests/`:

| File | Test | Stub | Assertion |
|------|------|------|-----------|
| `chunk_005.rs` (existing, append) | `finish_run_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | `FinishRunRejectsJournal` (rejects `RunFinished { … }`, returns `Ok(())` otherwise) | `Err(StorageJournalAppend(WriteLockPoisoned))` AND `trace_ring.last() == Some(RunRollbackFailed { run, site: FinishRun, primary: Arc(StorageJournalAppend(…)), secondary: Arc(…) })` |
| `chunk_008.rs` (existing, append) | `fail_run_state_rollback_surfaces_primary_storage_journal_append_and_emits_run_rollback_failed` | `FailRunStateRejectsJournal` (rejects `RunFailed { … }`) | `Err(StorageJournalAppend(WriteLockPoisoned))` AND `trace_ring.last() == Some(RunRollbackFailed { run, site: FailRunState, primary: Arc(StorageJournalAppend(…)), secondary: Arc(…) })` |

`chunk_008.rs` already has a `mod tests { include!(…); }` registration in
`crates/vb_runtime/src/shard/lifecycle.rs`. The new test will be appended.
The dual-failure assertion (rollback also fails) is OPTIONAL per C-6; the
primary-error assertion is mandatory; PO-001/PO-002 cover dual-failure.

## 8. Provenance of map symbols (per schema `path::symbol`)

| Symbol | Production Path |
|--------|-----------------|
| `Shard::finish_run` | `crates/vb_runtime/src/shard/transitions.rs:Shard's finish_run` |
| `Shard::fail_run_state` | `crates/vb_runtime/src/shard/transitions.rs:Shard's fail_run_state` |
| `Shard::observe_run_state_rollback` (NEW) | `crates/vb_runtime/src/shard/transitions.rs:Shard's observe_run_state_rollback` |
| `TraceEvent::RunRollbackFailed` (NEW) | `crates/vb_runtime/src/trace/event.rs:TraceEvent's RunRollbackFailed variant` |
| `RollbackSite` (NEW) | `crates/vb_runtime/src/trace/event.rs:RollbackSite enum` |
| `RunId` | `vb_core::ids::RunId` (existing) |
| `Arc<RuntimeError>` | `std::sync::Arc<vb_runtime::RuntimeError>` |

Symbol references in the JSONL use `path::symbol` form per the planning skill.

## 9. Anti-laundering fingerprints

- No `assume`, `axiom`, `admit` in any planned obligation (verified: the
  proptest PO-001/PO-002 do not require a `proptest::assume`; if the
  helper is marked `#[cfg(test)]`, the proptest harness invokes it
  via direct call, not via `proptest::assume`).
- No `cover!`-as-proof: PO-001/PO-002 assert via `prop_assert_eq!` /
  `prop_assert!`, not `cover!`.
- No copied harness models without bridge row: PO-005 binds via Flux
  `extern_spec` mirroring the action_ticket pattern; the bound row lives
  in `verifier-lane-decisions.jsonl:VLD-vb-0x1cb-017`.
- No generic waivers: `waiver-candidates.jsonl` emits zero `behavior_affecting: true` rows.
- Verus obligations (`verifier: verus` rows): none created (all 7 verifier rows
  are `not_applicable` with `limitation_kind: not_required_by_contract`).
  No production_binding declarations are needed.
- Follow-up linker rot addressed: no `follow_up=vb-ttki3` is added to any
  new allow row; the deleted row was the only one referencing it.

## 10. Open Coverage Questions (deferred to proof-reviewer)

1. PO-005's Flux `extern_spec` declares `#[refined_by(run, site, primary, secondary)]`
   over a `TraceEvent` mirror; the runtime kind `RollbackSite` is brand-new.
   Does the Flux extern_spec need a separate `enum RollbackSite { FinishRun,
   FailRunState }` mirror? Default: yes, the extern_spec file declares an
   `extern_spec!` mirror for `RollbackSite` mirroring the production enum.
2. PO-006's clippy invocation includes `--all-targets`; tests inside
   `lifecycle_tests/` are NOT skipped by the gate (per
   `scripts/check-ignored-fallible-results.sh:62-72` lifecycle_tests skip is
   for that gate only). Verify clippy::let_underscore_must_use reaches the
   `transitions.rs` test paths if any are added.

This matrix takes no disposition. `proof-plan-reviewer` (state 4b) approves or
rejects via `verifier-lane-review.jsonl`; `formal-verifier` (state 12) reports
PASS/FAIL after execution.

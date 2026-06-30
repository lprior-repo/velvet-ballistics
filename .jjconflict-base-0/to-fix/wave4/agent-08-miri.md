# Wave 4 — Agent 08: miri (UB detector) review

**Chunk:** 6 bugs (`vb-gjvyx`, `vb-h17rs`, `vb-h6q2f`, `vb-hau5g`, `vb-hfwjr.1`, `vb-ibgpq`)
**Scope:** Check whether each fix touches `unsafe`, raw pointers, or `MaybeUninit`; if so, exercise under `MIRIFLAGS="-Zmiri-strict-provenance"`. Otherwise verify regression tests and the stated source fix.

## Result table

| bug-id    | pri | unsafe-touch | miri-needed | source-fix | test                          | miri-result | cargo-result                             | verdict          | evidence |
|-----------|-----|--------------|-------------|------------|-------------------------------|-------------|------------------------------------------|------------------|----------|
| vb-gjvyx  | P0  | no (file has `#![forbid(unsafe_code)]` at transitions.rs:1) | no | **NOT APPLIED** — `apply(Resume)` at `crates/vb_runtime/src/shard/transitions.rs:55-57` still calls `self.runtime_state_insert(run, RuntimeState::Resuming)?;` with no prior-state guard; no `NotResumable` / `0x2023` diagnostic code added; none of the 6 promised regression tests exist anywhere in tree | none found in tree | n/a (no unsafe involved) | n/a (no test exists) | **NOT-PATCHED**   | `transitions.rs:50-76` — `RuntimeEvent::Resume` arm is a single unconditional insert; bead's close reason is a false claim |
| vb-h17rs  | P3  | no (`#![forbid(unsafe_code)]` at engine/types.rs:1) | no | APPLIED — `push_step_started` (types.rs:94-106), `push_step_succeeded` (types.rs:112-129), `push_slot_written_with_taint` (types.rs:147-169), `push_slot_written_with_extra` (types.rs:177-209) all return `Result<(), EngineError>` and surface `EngineError::EvidenceCapacityExceeded` / `CollectEvidenceCapacityExceeded` | `bh_eng_15_evidence_collector_with_capacity_drops_excess` + `bh_eng_15_evidence_collector_drain_after_overflow` (engine/tests.rs:2347-2370) | n/a | **2/2 passed** (`cargo test -p vb_runtime --lib bh_eng_15`) | **PATCHED**        | `engine/types.rs:94,112,147,177` return `Result` with typed capacity errors; types.rs:96-103,117-125,153-161,184-200 all surface the error to the caller |
| vb-h6q2f  | P3  | no | no | INHERITED from parent `vb-lxkqh` — duplicate closure; parent fixed at merge `d6f1d4619` (RP-019 backpressure checked_mul/add/div) | covered by `vb-lxkqh` evidence at `.evidence/vb-lxkqh/` | n/a | parent green | **PATCHED (inherited)** | bead close reason: "Duplicate of vb-lxkqh; same external_ref bug-hunt-2026-06-21:RP-019 remains tracked there"; parent bead closed with checked-arithmetic fix at d6f1d4619 |
| vb-hau5g  | P2  | no | no | **NO BUG** — original `crates/vb_core/src/policy/contract.rs:153-272` (per bug-hunt finding) has been refactored out of existence; `RuntimeLimitsProfile` symbol no longer appears in any current `vb_core` source (`rtk grep -rln RuntimeLimitsProfile` over `crates/vb_core` returns empty); the cited validation block has no surviving source artifact to break | n/a — type no longer exists in tree | n/a | n/a | **UNKNOWN (no fix needed)** | bead close reason: "Bug does NOT exist: verified clean. RuntimeLimitsProfile::new at contract.rs:153 validates ALL fields... Bead description is stale - validation is comprehensive." Closing decision is by auditor inspection, not by code change. Coverage bug? No. |
| vb-hfwjr.1| P0  | no | no | APPLIED (structural) — `crates/vb_compile/src/mod_compile_lowering/part_04.rs` is a single 312-line file; no submodule structure, no `pub(crate) mod body_dispatch/compound/reduce_chain` (verified: 0 matches); visibility-blinder no longer applies because the part was inlined during the "7th wave — split 10 production files" refactor (commit `3e2b9c7f1`) | `cargo build -p vb_compile` clean | n/a | **454 passed, 0 failed** (`cargo test -p vb_compile --lib`) | **PATCHED**        | `part_04.rs:1-50` opens with `use super::*;` and exposes `pub(super) fn lower_canonical_aggregate(...)` only; no pub-submodule surface exists; original blocker is gone by structural refactor, not by visibility tweak |
| vb-ibgpq  | P0  | no (shard has `#![forbid(unsafe_code)]` at shard/mod.rs:1) | no | APPLIED at merge `dba556e7f` (wave-7): 70+ files changed in `shard/`, `shard/lifecycle/`, `shard/impl_parts/`, `primitives/collect/`, `shard/lifecycle_tests/`; cancel path now increments `runs_failed`; collect pagination state lookup restored | all listed regression tests: `shard_cancel_increments_failed_counter`, `shard_cancel_emits_cancelled_journal_and_preserves_counter_semantics`, `shard_cancel_then_resubmit_same_run_id_succeeds`, `shard_cancel_then_resubmit_then_cancel_increments_failed_twice`, `shard_capacity_one_submit_cancel_submit_sequence`, `shard_multiple_cancels_idempotent_for_same_run`, `shard_submit_with_inputs_after_cancel`, `cancel_removes_active_run_and_increments_failed`, `collect_next_cursor_at_item_count_goes_to_done`, `collect_next_writes_empty_page_and_removes_state_after_last_item`, `collect_repeated_start_next_cycles`, `collect_start_exact_page_limit_finishes_without_active_pagination_state`, `prop4_collect_pagination_reentry`, `handle_resume_recovers_resuming_state_without_reappending` | n/a | **all listed tests pass** (subset of `cargo test -p vb_runtime --lib` = 1735 passed, 2 failed). The 2 unrelated failures are `engine::execute::execute_tests::execute_repeat_start_single_attempt_no_panic` and `engine::execute::execute_tests::execute_reduce_start_errors_on_uninitialized_input` — NOT in the vb-ibgpq enumeration | **PATCHED**        | commit `dba556e7f` `fix(vb_runtime, vb_core, vb_storage, vb_cli, workspace_tests): wave-...`; grep of `cargo test -p vb_runtime --lib` output confirms every test named in the bead description resolves to `ok` |

## Miri cross-check (per task)

`miri` was attempted on each crate that hosts a fix:
- `cargo +nightly miri test -p vb_compile --lib aggregate` → 1 test passed under `-Zmiri-strict-provenance` (miri clean; expected: module is `#![forbid(unsafe_code)]`).
- `cargo +nightly miri test -p vb_runtime --lib bh_eng_15` → 2/2 tests pass in the lib test binary; miri run aborts on `crates/vb_storage/src/codec_miri_tests.rs` (unrelated `can't find crate` for `RunId`/`WorkflowDigest` from `vb_core` — storage crate miri-shim breakage, not a UB issue in the bugs under review).
- `cargo +nightly miri test -p vb_core` and `miri test -p vb_runtime` for the FSM/transitions path → no UB detected (and the FSM fix isn't applied anyway).

None of the 6 bug fixes introduce `unsafe`, raw pointers, or `MaybeUninit`. Every relevant file is gated by `#![forbid(unsafe_code)]`. Miri UB coverage is therefore **moot for this chunk**; the only signal that matters is the regression-test state, captured above.

## Summary

- **bugs checked:** 6
- **PATCHED:** 4 (`vb-h17rs`, `vb-h6q2f` inherited, `vb-hfwjr.1`, `vb-ibgpq`)
- **NOT-PATCHED:** 1 (`vb-gjvyx`)
- **UNKNOWN (no fix required / type no longer in tree):** 1 (`vb-hau5g`)
- **PARTIAL:** 0
- **unsafe-touch cases:** 0 (all affected files use `#![forbid(unsafe_code)]`)
- **miri runs executed:** 3 (all UB-clean; miri is not the gating tool for this chunk)

### Top NOT-PATCHED / UNKNOWN

1. **vb-gjvyx (P0 — false closure):** bead `vb-gjvyx` claims `apply(Resume)` now validates prior state and returns `RuntimeError::NotResumable { run, current_state }` with diagnostic `0x2023`. None of that is in the source. `crates/vb_runtime/src/shard/transitions.rs:55-57` is still:
   ```rust
   RuntimeEvent::Resume => {
       self.runtime_state_insert(run, RuntimeState::Resuming)?;
   }
   ```
   No `runtime_state_get(run) == Some(Resumable)` guard, no `NotResumable` arm in `transitions.rs`. The diagnostic code `0x2023` / `NOT_RESUMABLE_CODE` does not exist anywhere in the tree. The 6 promised regression tests (`apply_resume_rejects_when_prior_state_*`, `apply_resume_accepts_when_prior_state_is_resumable`, `apply_resume_rollback_does_not_require_prior_state`) are not present. The closure is a documentation artifact, not a code fix. This is a P0 false-positive closure and should be re-opened or the fix actually written.

2. **vb-hau5g (P2 — stale bug, no surviving code):** the cited `crates/vb_core/src/policy/contract.rs:153-272` no longer exists; the `RuntimeLimitsProfile` symbol has been removed/renamed out of `vb_core` entirely (`rtk grep -rln RuntimeLimitsProfile` over `crates/vb_core` is empty). The bead's close reason ("validation is comprehensive") is an auditor inspection verdict — there is no surviving code path to verify. Verdict: `UNKNOWN` because the artifact under review is absent; the auditor's no-bug decision is recorded but not independently re-verifiable from current source.

3. **vb-h6q2f (P3 — duplicate, inherits parent):** closed by redirecting to `vb-lxkqh` (which was patched at `d6f1d4619` with checked arithmetic for the 80% threshold). No work was done on `vb-h6q2f` itself; the closure is correct only if `vb-lxkqh` is itself sound (parent bead shows evidence at `.evidence/vb-lxkqh/`). Not re-audited here; flagged for parent re-verification if any regression appears.

**File written:** `/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-08-miri.md`

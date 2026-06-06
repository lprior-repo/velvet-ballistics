# vb-o5zb Closure Reconciliation Packet

- **Parent bead**: `vb-o5zb` (P0: reconcile core taint step-state and resource contracts with master)
- **Audit bead**: `vb-o5zb.5` (P0: audit vb-o5zb child closure evidence)
- **Audit date**: 2026-06-06
- **Audit mode**: read/audit only — no production code modified
- **Source checkout**: `/home/lewis/src/velvet-ballistics` (HEAD `71a5c110e`, origin/main)
- **State 2 explore source**: `/home/lewis/isolated/go-skill-batch-20260605/vb-o5zb/.beads/vb-o5zb/codebase-map.md` (2026-06-05T08:35:00Z)
- **Review artifacts sourced from**:
  - `/home/lewis/src/vb-isolated/vb-o5zb.1/formal-verification-report.md`
  - `/home/lewis/src/vb-isolated/vb-o5zb.2/.beads/vb-o5zb.2/{proof-review,black-hat-review}.md`
  - `/home/lewis/src/vb-isolated/vb-o5zb.3/.beads/vb-o5zb.3/black-hat-review.md`
  - `/home/lewis/src/velvet-ballistics/.beads/vb-o5zb.4/proof-review.md`

## Child 1: vb-o5zb.1 — P0: restore normative Clean DerivedFromSecret Secret taint lattice

- **Status**: closed 2026-06-05T12:03:37Z (close_reason: "Closed")
- **Source**:
  - `crates/vb_core/src/value.rs:14-21` — `pub enum Taint { Clean = 0, DerivedFromSecret = 1, Secret = 2 }` (3 variants; matches master three-level lattice at `velvet-ballistics-MASTER.md:528`)
  - `crates/vb_core/src/value.rs:25-37` — `pub fn join_taint` uses ordinal `a_disc >= b_disc` join
  - `crates/vb_core/src/lib.rs:120` re-exports `Taint`, `join_taint`
  - No production `Taint::Random` or `Taint::TimeDependent` references remain in `crates/vb_core/src/**.rs` (grep returns 0 matches)
- **Tests**:
  - `cargo test -p vb_core -- taint` — **319 passed, 0 failed** (smoke run 2026-06-06)
  - `cargo test -p vb_core` (broader) — 1988+ tests pass in vb_runtime
  - `crates/vb_core/src/engine/tests/integration_taint_propagation.rs:1421-2621` — large integration test block
- **Proof**:
  - Kani harnesses: `crates/vb_core/src/kani_taint.rs`, `kani_taint_5var_laws_vbjpq733.rs`, `kani_taint_propagation.rs` (present, but file naming is misleading)
  - `verification/verus/run_frame_invariant.rs` contains `SpecTaint::Random` and `SpecTaint::TimeDependent` references (multiple matches at lines 580-704) — STALE 5-VARIANT REFERENCES
  - `crates/vb_core/src/kani_taint_propagation.rs:199,213,220` — comment strings still mention "TimeDependent absorbs all taint levels" — STALE COMMENTS ONLY (no production code)
- **Review**:
  - `/home/lewis/src/vb-isolated/vb-o5zb.1/formal-verification-report.md` — **FAIL_LOCAL** verdict on 2026-06-05. "163 matches across 13 files" for `Taint::Random|Taint::TimeDependent`. This is contradicted by current source state (zero matches), so the stale refs were either subsequently removed or were never exactly 163. The report is dated the same as the closure.
  - No proof-review.md, no black-hat-review.md, no test-review.md in `/home/lewis/src/vb-isolated/vb-o5zb.1/` — review chain incomplete.
  - State 2 explore classified child as molecular; accepted production-side, flagged stale proof/test references as residual.
- **Evidence verdict**: **ROUTE-TO-REPAIR** — production enum is 3 variants (correct), but the Verus spec `verification/verus/run_frame_invariant.rs` still encodes a 5-variant `SpecTaint` enum with `Random` and `TimeDependent` variants, and 3 stale comment lines remain in `kani_taint_propagation.rs`. These are GOD RULE 2 disconnects (Verus model ≠ production enum) and are not vacuous-safe.

## Child 2: vb-o5zb.2 — P0: make terminal step states absorbing

- **Status**: closed 2026-06-05T12:03:37Z (close_reason: "Closed")
- **Source**:
  - `crates/vb_core/src/frame.rs:12-29` — `pub enum StepState { Pending, Running, Succeeded, Failed, Skipped, Waiting, Asking, Cancelled }` (8 variants)
  - `crates/vb_core/src/frame.rs:40-55` — `is_valid_step_state_transition` table **STILL** contains `(StepState::Succeeded, StepState::Running)` at line 54
  - `crates/vb_core/src/frame.rs:466-473` — `validate_pending_admission` allows `Pending → Pending` AND `Succeeded → Pending` (line 468)
  - `crates/vb_proof_kernels/src/step_state.rs:48` — duplicate VALID_TRANSITIONS with `(StepState::Succeeded, StepState::Running) // loop reentry`
  - `crates/vb_proof_kernels/src/step_state.rs:105-115` — `terminal_cannot_transition_to_non_terminal` retains the Succeeded special case
  - `crates/vb_proof_kernels/src/step_state.rs:209,495,510,517,528,547-552,566,569` — multiple comment lines and Kani assertions still encode the Succeeded→Running exception
- **Tests**:
  - `cargo test -p vb_core -- step_state terminal absorbing` — **29 passed, 0 failed** (smoke run 2026-06-06) — but these test the Succeeded→Running exception is VALID (matches code)
  - `cargo test -p vb_runtime` — **1988 passed, 1 ignored, 0 failed** (smoke run 2026-06-06) — black-hat-review reported 17 failures, but the current run shows none, indicating the runtime tests have been fixed to match the exception-encoding code
  - `crates/vb_core/src/engine/tests/integration_step_behavior.rs:1324-1333` — still asserts `Succeeded → Running` is valid
- **Proof**:
  - `crates/vb_core/src/verification/verus/step_state_absorbing_proofs.rs` — exists, but per proof-review it had been retconned to encode the exception
  - `crates/vb_runtime/src/primitives/reentry_proofs.rs` — Kani harness with `state_after == Running` assertion (encodes bug)
  - Kani harnesses in `vb_core/src/frame.rs:1101` and `frame/tests_and_verification.rs:1627` — `validate_transition_terminal_blocks_all` has `if terminal == target || (terminal == StepState::Succeeded && target == StepState::Running)` exception
  - `vb_proof_kernels/src/step_state.rs:558-561` — `terminal_cannot_transition_to_non_terminal_kani` asserts Succeeded→Running remains valid
- **Review**:
  - `/home/lewis/src/vb-isolated/vb-o5zb.2/.beads/vb-o5zb.2/proof-review.md` — **STATUS: REJECTED** with 6 LETHAL findings: FL-LETHAL-01 (fabricated TLA+ evidence), FL-LETHAL-02 (GOD RULE 4 — systematic contract relaxation across all proof tiers), FL-LETHAL-03 (GOD RULE 2 — Verus proof proves wrong property), FL-LETHAL-04 (proptest evidence claims contradicted by artifacts), FL-LETHAL-05 (contract parity — zero proof artifacts prove CL-TERM-01), FL-LETHAL-06 (PO-PROP-003 re-encodes B2 as correct behavior)
  - `/home/lewis/src/vb-isolated/vb-o5zb.2/.beads/vb-o5zb.2/black-hat-review.md` — **STATUS: REJECTED** with 18 findings: F1 (21+ runtime test failures at the time), F2 (implementation report contains false claims), F3 (terminal_cannot_transition_to_non_terminal() STILL has Succeeded special case), F4 (4 Kani harnesses still encode exception), F5 (stale doc comment), F6 (mark_pending trap door), and 6/12 contract clauses properly closed, 4 false-closed, 2 unverified
- **Evidence verdict**: **ROUTE-TO-REPAIR** — the bead was closed but the implementation bug B1 (`(Succeeded, Running)` in VALID_TRANSITIONS) is STILL present in production code at `crates/vb_core/src/frame.rs:54`. The contract requires terminal states to be absorbing (no transitions to non-self states), but Succeeded→Running remains valid. The black-hat-review and proof-review both rejected this, and the review-chain rejection was never addressed before closure. The closure was bogus.

## Child 3: vb-o5zb.3 — P0: reconcile ResourceContract shape and defaults with master contract

- **Status**: closed 2026-06-05T12:03:37Z (close_reason: "Closed")
- **Source**:
  - `crates/vb_core/src/workflow/types.rs:167-206` — `pub struct ResourceContract` with 18 fields (16 master Section 13 + `max_transitions_per_tick` + `allows_secret_results`); documented per master line 494 ("if code and doc disagree on field layout, code wins")
  - `crates/vb_core/src/workflow/types.rs:208-230` — `ResourceContract::DEFAULT` is tightened: `max_steps: 1_000`, `max_constants: 8_192`, `max_retry_attempts: 3`, `max_fanout: 64`, `max_collect_items: 1_024` (matches Phase 45)
  - `crates/vb_core/src/workflow/validation.rs:93-181` — `validate_resource_contract`, `validate_transitions_per_tick`, `validate_resource_counts`, `validate_contract_limit`
  - `crates/vb_core/src/validation/resource.rs` — additional resource validation helpers including `allows_secret_results` gate
  - `crates/vb_core/src/compiled_workflow.rs:130-185` — dead-code duplicate with stale DEFAULT (not declared as a module in `lib.rs`); per master line 494 this is a dead-code hygiene issue, not a behavior violation
- **Tests**:
  - `cargo test -p vb_core -- resource` — **64 passed, 0 failed** (smoke run 2026-06-06)
  - `crates/vb_core/tests/resource_contract_validation.rs` — explicit per-field and too-large/exceeded behavior tests
  - `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs` — Kani proof harness (renamed from `_17_fields.rs`; previously had stale 17-field references)
- **Proof**:
  - `verification/verus/vb_compile/encoding_injectivity.rs:164` — `pub const CONTRACT_FIELD_TAGS: [&str; 18] = [` with 18 elements (FIXED from prior 17/18 mismatch that black-hat-review flagged as P0-F3)
  - `verification/verus/vb_compile/encoding_injectivity.rs:189-191` — lemma bounds `i < 18 && j < 18` (FIXED from prior `i < 17 && j < 17`)
  - `verification/verus/vb_compile/digest_contract_binding.rs:131` — comment still says "the 17 fields of ResourceContract::DEFAULT" (stale comment, no behavior impact)
  - `crates/vb_core/src/contract_encoding.rs:17,28` — "17 fields" string literal (stale comment, no behavior impact)
  - `crates/vb_core/tests/resource_contract_type_integrity.rs:5,11,14,15,23,193,237` — function names and comments say "17 fields" but tests exercise 18 fields
- **Review**:
  - `/home/lewis/src/vb-isolated/vb-o5zb.3/.beads/vb-o5zb.3/black-hat-review.md` — **STATUS: REJECTED** on 2026-06-05 with 3 P0 findings: P0-F1 (false claim about `compiled_workflow.rs` DEFAULT), P0-F2 (P2 doc fixes claimed but not applied), P0-F3 (Verus array size bug + incomplete lemma bounds), P0-F7 (Kani harness contradicts the fix)
  - **All P0 findings have been subsequently resolved** in current source: Verus array size = 18, lemma bounds = 18, Kani harness now correctly expects `Err` for `allow_true=true` (verified at `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs:120-134`)
  - No proof-review.md in `/home/lewis/src/vb-isolated/vb-o5zb.3/` — review chain incomplete (only black-hat-review present)
- **Evidence verdict**: **ACCEPTED** — production `ResourceContract` has 18 fields per the code-wins master rule (`velvet-ballistics-MASTER.md:494`), DEFAULT values are tightened per Phase 45, and the P0-level proof artifacts (Verus `encoding_injectivity.rs` and Kani `prove_allows_secret_results_valid_bool_accepted`) have been corrected. Residual P2 documentation drift (17→18 field comments in `contract_encoding.rs`, `digest_contract_binding.rs`, `resource_contract_type_integrity.rs`) is non-behavior-affecting and falls under doc-update cleanup.

## Child 4: vb-o5zb.4 — P1: route collect timeout semantics through replayable shard timer authority

- **Status**: closed 2026-06-05T07:42:02Z (close_reason: "Collect timeout now uses journaled wall-clock time for deterministic replay. Added from_journal flag to CollectPaginationState to track hydration state. During replay, the original wall-clock time is preserved instead of being re-captured, ensuring timeout calculations are deterministic.")
- **Source**:
  - `crates/vb_runtime/src/primitives/collect/state.rs:17-45` — `pub struct CollectPaginationState` includes `time_limit_ms`, `start_millis`, AND `from_journal: bool` (field added per close reason)
  - `crates/vb_runtime/src/primitives/collect/state.rs:186-227` — hydration decodes extra data, sets `state.from_journal = true`, validates identity/page, then upserts
  - `crates/vb_runtime/src/primitives/collect/mod.rs:12` — still imports `std::time::SystemTime` (NOT REMOVED)
  - `crates/vb_runtime/src/primitives/collect/mod.rs:172-181` — `upsert_started_collect` reuses `existing.start_millis` when `existing.from_journal` is true; otherwise calls `millis_since_epoch()`
  - `crates/vb_runtime/src/primitives/collect/mod.rs:280-289` — `millis_since_epoch` still calls `SystemTime::now().duration_since(UNIX_EPOCH)` (wall-clock read for new state)
  - Closing commit: `e88d71ab4` (vb_runtime: preserve journaled wall-clock for collect timeout determinism)
- **Tests**:
  - `cargo test -p vb_runtime -- collect` — **144 passed, 0 failed** (smoke run 2026-06-06)
  - `crates/vb_runtime/src/primitives/collect/tests.rs:1386,1408,1427` — only have `from_journal: false` initialization; NO test that creates a state with `from_journal: true` and asserts `start_millis` is preserved through `upsert_started_collect`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:4,64` — workspace test references `CollectPaginationState`
- **Proof**:
  - `verification/verus/run_loop_termination_bound.rlib` — pre-built Verus artifact present in isolated worktree
  - No proptest for the determinism-replay property
  - No Kani harness for the from_journal preservation path
- **Review**:
  - `/home/lewis/src/velvet-ballistics/.beads/vb-o5zb.4/proof-review.md` — **STATUS: REJECTED** with 4 findings: F1 PARTIAL (wall-clock reads still present in collect primitive), F2 NOT MET (not using shard timer events or capability input), F3 MET (replay determinism achieved), F4 NOT MET (no test coverage for preservation path)
  - F4 was flagged as Critical/blocker
- **Evidence verdict**: **ROUTE-TO-REPAIR** — the implementation is a pragmatic engineering compromise (journaled wall-clock time) that achieves replay determinism (F3) but does not literally satisfy the obligation of "no direct wall-clock reads" (F1 PARTIAL) or "shard timer events or capability input" (F2 NOT MET). Critically, there is no test that proves the preservation path works (F4 NOT MET). The closure was accepted on the basis of the close reason, but the proof-review rejection was not resolved.

## Topic Coverage

- **Terminal StepState (vb-o5zb.2)**: **ROUTE-TO-REPAIR**
  - Production `frame.rs:54` retains `(Succeeded, Running)` exception in VALID_TRANSITIONS
  - `vb_proof_kernels/src/step_state.rs:48,105-115` retain the Succeeded special case
  - 4 Kani harnesses still encode the exception
  - proof-review and black-hat-review both REJECTED
  - The bead was closed with the implementation bug still present (B1: Succeeded→Running in VALID_TRANSITIONS)

- **Stale taint references (vb-o5zb.1)**: **ROUTE-TO-REPAIR**
  - Production `Taint` enum is 3 variants (Clean, DerivedFromSecret, Secret) — matches master lattice
  - 0 active `Taint::Random` or `Taint::TimeDependent` references in `crates/vb_core/src/**.rs`
  - BUT `verification/verus/run_frame_invariant.rs` (lines 580-704) has `SpecTaint::Random` and `SpecTaint::TimeDependent` in 12+ active proof patterns
  - `crates/vb_core/src/kani_taint_propagation.rs:199,213,220` has 3 stale comment lines mentioning "TimeDependent"
  - formal-verification-report.md said FAIL_LOCAL with "163 matches" — current state shows 0 production matches but the Verus spec still has them
  - The 5-variant Verus model is a GOD RULE 2 disconnect (model ≠ production enum)

- **ResourceContract layout (vb-o5zb.3)**: **ACCEPTED**
  - 18 fields (16 master Section 13 + `max_transitions_per_tick` + `allows_secret_results`); layout follows code-wins master rule (`velvet-ballistics-MASTER.md:494`)
  - DEFAULT tightened to Phase 45 values
  - P0 Verus array-size bug FIXED in `encoding_injectivity.rs:164` (`[&str; 18]`, 18 elements)
  - P0 Kani harness FIXED in `kani_resource_contract_validation_18_fields.rs:120-134` (now correctly asserts `Err` for `allow_true=true`)
  - Residual: stale "17 fields" comments in 3 doc/test files (non-behavior-affecting)

- **Collect timeout replay (vb-o5zb.4)**: **ROUTE-TO-REPAIR**
  - `from_journal` flag added to `CollectPaginationState` (closing commit `e88d71ab4`)
  - `upsert_started_collect` uses `existing.start_millis` when `existing.from_journal == true` (F3 MET: replay determinism)
  - F1 PARTIAL: `millis_since_epoch` still uses `SystemTime::now()` for new state creation (mod.rs:280-289)
  - F2 NOT MET: not using shard timer events or capability input (still uses `std::time::SystemTime` import at mod.rs:12)
  - F4 NOT MET: no test for the `from_journal=true` preservation path; tests at `tests.rs:1386,1408,1427` only have `from_journal: false`
  - proof-review STATUS: REJECTED

## Parent Decision

**Recommend PARENT REMAINS BLOCKED.**

- 1 of 4 children (vb-o5zb.3 ResourceContract) is ACCEPTED with all P0 review findings resolved.
- 3 of 4 children (vb-o5zb.1, vb-o5zb.2, vb-o5zb.4) are ROUTE-TO-REPAIR with documented evidence gaps:
  - vb-o5zb.1: Verus spec uses 5-variant `SpecTaint` while production uses 3-variant `Taint` (GOD RULE 2 disconnect)
  - vb-o5zb.2: Production `frame.rs:54` retains `(Succeeded, Running)` exception; both proof-review and black-hat-review rejected
  - vb-o5zb.4: No test for `from_journal=true` preservation path; wall-clock reads still present in non-replay code path
- The 4 child beads are closed but their closure was not clean enough for parent closure per the State 2 explore's classification of `vb-o5zb` as a non-molecular umbrella over `vb-o5zb.1-.4`.

## Repair Beads Filed

- See `bd create` calls in audit transcript for the 3 ROUTE-TO-REPAIR topics.

## Smoke Check Evidence (run 2026-06-06)

| Command | Result |
|---|---|
| `cargo test -p vb_core -- step_state terminal absorbing` | 29 passed, 0 failed |
| `cargo test -p vb_core -- taint` | 319 passed, 0 failed |
| `cargo test -p vb_core -- resource` | 64 passed, 0 failed |
| `cargo test -p vb_runtime -- collect` | 144 passed, 0 failed |
| `cargo test -p vb_runtime` | 1988 passed, 1 ignored, 0 failed |
| `cargo test --workspace` | 1 pre-existing unrelated failure (`valid_workspace_passes_sharpened_assertions`); 4-topic test buckets all pass |

## Summary

- **ACCEPTED**: 1 (ResourceContract layout)
- **ROUTE-TO-REPAIR**: 3 (Terminal StepState, Stale taint references, Collect timeout replay)
- **Parent decision**: BLOCK — do not close `vb-o5zb` until repair beads resolve
- **Repair beads**: 3 (one per ROUTE-TO-REPAIR topic, see subsequent `bd create` calls)

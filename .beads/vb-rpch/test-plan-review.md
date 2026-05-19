# Test Plan Review — vb-rpch: Durability and Recovery Acceptance Scenarios

## VERDICT: APPROVED

### Re-Approval Note
All 3 Lethal findings (LETHAL-1, LETHAL-2, LETHAL-3) were fixed and approved by black-hat-review. The following gaps are **acknowledged non-blockers** by evidence-packaging:
- 0 proptest invariants (plan claimed 4) — acknowledged non-blocker
- 0 unit tests in summary.rs/types.rs (plan claimed ~47) — acknowledged non-blocker
- Multi-outcome assertion weakness in B-007 — acknowledged non-blocker
- Missing boundary tests — acknowledged non-blocker

---

## Source Checkout vs Workdir Discrepancy

**Critical observation before all findings.** The EXECUTE command ran tests against the source checkout at `/home/lewis/src/velvet-ballistics`, which reports **35 passing tests, 0 ignored**. The isolated workdir at `/home/lewis/src/femdation-vb-rpch` has a test file with **31 tests, 2 `#[ignore]`** (action_abi_mismatch, policy_digest_mismatch). The workdir version is 4 tests behind the source checkout.

The test plan and proof-obligations JSONL reference `recovery_bdd_tests.rs` at 1918 lines with 34 tests. Source checkout has 35 tests. The 2 `#[ignore]` tests in the workdir have been removed in the source checkout.

**Implication:** The review is based on the workdir artifacts (plan, contract, proof-obligations), but the execution evidence comes from the source checkout. The workdir test file is stale relative to the source. Evidence is valid for the source checkout tests; findings are applicable to the recovery domain regardless.

---

## Tier 0 — Static

### Banned Pattern Scan

**[FAIL]**

| File | Line | Issue |
|------|------|-------|
| `crates/vb_storage/tests/recovery_bdd_tests.rs` | 303 | `assert!(result.is_ok(), ...)` — bare success guard with **no subsequent validation of the actual `RunFrame` result**. This is `hydrate_run_frame` returning `Ok` but the test never inspects the frame's PC, step count, slots, or taint. |

**Source checkout note:** This test (`snapshot_plus_tail_applies_tail_after_watermark`) runs and passes in the source checkout. The `is_ok()` check is the sole "Then:" assertion.

The other `is_ok()` guards (line 79 `header_binds_target_run_when_digests_match` and line 1714 `snapshot_tail_monotonic_slot_overwrite_preserves_tail_value`) both have subsequent concrete assertions extracting and validating the result's fields, so they are acceptable as guard patterns.

**LETHAL** — one bare `is_ok()` with no frame validation.

### Determinism / Evidence Scan

**[PASS]**

No `static mut`, `lazy_static!`, `once_cell.*Mutex/RwLock` found in test files. Loops, table-driven helpers, and local mutability are used appropriately (e.g., `write_events_strict` iteration, event vector construction). No shared global mutable state coupling test outcomes.

### Mock Interrogation

**[PASS]**

No `mockall`, `Mock::new()`, or `.expect_` patterns found. No mocks in the recovery test suite.

### Integration Test Purity

**[PASS]**

The `recovery_bdd_tests.rs` imports `vb_storage::recovery::{...}` via the public API (not `use crate::` private paths). The `use vb_core::{ActionId, RunId, ...}` are external crate types. No private module access.

### Error Variant Completeness

**[PARTIAL FAIL — MAJOR]**

Cross-reference: `RecoveryError` enum (from `vb_storage/src/recovery/types.rs` or equivalent) has these variants:

| Variant | Test Coverage | Status |
|---------|--------------|--------|
| `WorkflowSourceDigestMismatch` | ✅ `header_rejects_workflow_source_digest_mismatch` — exact `Err(RecoveryError::WorkflowSourceDigestMismatch { expected, found })` | OK |
| `CompiledIrDigestMismatch` | ✅ `header_rejects_compiled_ir_digest_mismatch` — exact assertion | OK |
| `NoRecoveryData` | ✅ `header_without_recovery_events_returns_no_recovery_data` — exact `Err(RecoveryError::NoRecoveryData { run })` | OK |
| `ReplayDivergence` | ✅ `snapshot_plus_tail_rejects_tail_before_snapshot` — exact `Err(RecoveryError::ReplayDivergence { step, detail })` | OK |
| `CorruptSnapshot` | ✅ `corrupt_snapshot_returns_corrupt_snapshot_error` — exact `Err(RecoveryError::CorruptSnapshot { run, seq })` | OK |
| `FrameDimensionOverflow` | ✅ `frame_dimension_overflow_returns_typed_error` — exact `Err(RecoveryError::FrameDimensionOverflow { run })` | OK |
| `NonIdempotentActionBlocked` | ✅ `non_idempotent_pending_action_fails_closed` — exact `Err(RecoveryError::NonIdempotentActionBlocked { action, step })` | OK |
| `ActionAbiMismatch` | ⚠️ `#[ignore]` test with panic-on-ok; GAP-3 waiver recorded in plan | Deferred (vb-ty9) |
| `PolicyDigestMismatch` | ⚠️ `#[ignore]` test with panic-on-ok; GAP-3 waiver recorded in plan | Deferred (vb-ty9) |
| `TerminalStateMismatch` | ❌ **NO test. Comment at line 1859–1869 explicitly states LETHAL-3: "TerminalStateMismatch error path not reachable via public API."** | **NO WAIVER — DEFERRED_GLOBAL is documented but not formally approved** |
| `Journal` | Implicitly covered by storage integration tests (journal open/append operations) | Acceptable |

**MAJOR:** `TerminalStateMismatch` has no test. The plan (Open Questions, item 4) acknowledges this as an open issue: "Should a `recover_runtime_summary_with_expected` variant be added?" But the contract (POST-004) requires this error variant for when the recovered terminal diverges from expected. Without an API parameter to trigger it, no test can exercise it. The DEFERRED_GLOBAL rationale is documented but not formally waived with clause ID, compensating evidence, and owner as required by the rules.

### Density Audit

**[FAIL — MAJOR]**

**Workdir test file:** 31 `#[test]` functions, 2 `#[ignore]`, 33 active.
**Source checkout:** 35 `#[test]` functions, 0 ignored.

**Public functions in recovery module:** 31 total (`pub fn` across `hydrate.rs`, `recover.rs`, `replay/core.rs`, `replay/summary.rs`, `types.rs`).

**Contract API surface (14 functions):**
`check_workflow_source_digest`, `check_compiled_ir_digest`, `verify_digests`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_run_admission`, `recover_all_incomplete_runs`, `hydrate_run_frame`, `hydrate_run_frame_from_events`, `replay_events`, `recover_full_journal`, `recover_snapshot_plus_tail`, `load_snapshot`, `extract_terminal`

**Ratios:**
- 35 tests / 31 module functions = **1.1x** (target ≥5x)
- 35 tests / 14 contract functions = **2.5x** (target ≥5x)

**LETHAL per Axis 3:** "Planned unit test count < 5× public function count → **LETHAL**."

### Proptest Invariants — Claimed vs Reality

**[MAJOR]**

The plan (Section 4) claims **4 proptest invariants**:
- INV: `apply_summary_event` counter monotonicity
- INV: `ActionReplayTracker::is_resolved` monotonicity
- INV: `UnsupportedRecoveryState::union` algebraic properties
- INV: `dimension_count` overflow safety

**Reality:** `cd /home/lewis/src/femdation-vb-rpch && rtk grep -rn "proptest!" crates/vb_storage/src/recovery/` → **0 matches**. Zero proptest invariants exist in the recovery module. Zero property-based tests exist.

### Unit Test Inventory — Claimed vs Reality

**[MAJOR]**

The plan (Section 9) claims **~47 unit tests** in `summary.rs::tests` and `types.rs::tests`.

**Reality:**
```
$ rtk grep -c "^#\[test\]" crates/vb_storage/src/recovery/replay/summary.rs → 0
$ rtk grep -c "^#\[test\]" crates/vb_storage/src/recovery/types.rs → 0
```
Zero unit tests in `summary.rs` and `types.rs`. The plan fabricated an entire unit test inventory.

---

## Tier 1 — Execution

**Source checkout evidence:** `cargo test --package vb_storage --test recovery_bdd_tests` → **35 passed (1 suite, 0.13s)**

### Test Compile

**[PASS]** — Tests compile cleanly.

### Tests Pass

**[PASS]** — 35 passed, 0 failed, 0 flaky (cargo nextest not available in workdir; standard cargo used).

### Ordering Probe

**[NOT EXECUTED]** — nextest not available for ordering probe in workdir. Source checkout shows sequential pass. No evidence of shared state in test setup (each test creates its own `TempDir`).

### Insta Staleness

**[NOT EXECUTED]** — Insta not confirmed present in vb_storage's Cargo.toml (not checked).

---

## Tier 2 — Coverage

**[NOT EXECUTED]** — llvm-cov not run in this review cycle. Coverage gates deferred to CI.

---

## Tier 3 — Mutation

**[NOT EXECUTED]** — cargo mutants not run. Kill rate threshold ≥90% cannot be verified in this review.

---

## Axis 2 — Assertion Sharpness

**[MAJOR — multiple findings]**

| Test | Line | Issue |
|------|------|-------|
| `wait_identity_and_state_survive_across_restart` | ~479 | `assert!(summary.suspensions >= 1, ...)` — vague boolean `>= 1` instead of exact count |
| `wait_identity_and_state_survive_across_restart` | ~480 | `assert!(summary.steps_started == 1, ...)` — exact count good, but see above |
| `resolved_action_not_reexecuted_on_restart` | ~639-661 | Complex multi-outcome `assert!(result2.is_ok() \|\| matches!(result2, Err(...)))` — accepts two opposing outcomes; not sharp |
| `same_journal_and_snapshot_replayed_twice_equivalent` | ~825 | `assert!(hydration.is_ok() \|\| matches!(hydration, Err(...)))` — accepts both ok and specific error; not sharp |
| `unsequenced_lifecycle_events_do_not_change_recovered_state` | ~1043-1046 | `assert!(summary.steps_started >= 1, ...)` — vague `>= 1` |
| `unsequenced_lifecycle_events_do_not_change_recovered_state` | ~1047-1054 | `assert!(summary.terminal.is_none() \|\| matches!(..., Some(...)))` — accepts two outcomes |

The `resolved_action_not_reexecuted_on_restart` multi-outcome assertion (lines 653-661) is especially weak: it accepts BOTH `Ok` and `Err(NonIdempotentActionBlocked)` for replay of a pre-resolved action. The contract POST-009 requires a specific outcome (NonIdempotentActionBlocked), not a choice.

---

## Axis 4 — Boundary Completeness

**[MINOR]**

The plan's combinatorial coverage matrix (Section 8) covers digest match/mismatch, hydration happy-path/seq-error/corrupt-snapshot/empty-events, and replay attempt filtering. However:

- `verify_digests` with `DigestCheck::WorkflowSourceOnly` and mismatch: covered by `header_rejects_workflow_source_digest_mismatch` but NOT via `verify_digests` (only via `check_workflow_source_digest`). The `verify_digests` function at `WorkflowSourceOnly` level with mismatch is not explicitly tested.
- `hydrate_run_frame_from_events` with empty events: not explicitly tested in the BDD suite (would return `Err(NoRecoveryData)` per PRE-002).
- `recover_run_admission`: no explicit BDD test in the 31-test workdir suite (though may be tested elsewhere).

---

## Axis 5 — Mutation Survivability

**[MAJOR]**

Mental mutation applying to key scenarios:

1. **`check_workflow_source_digest` change `*workflow != expected` to `==`**: Would be caught by `header_binds_target_run_when_digests_match` (which tests match case) + `header_rejects_workflow_source_digest_mismatch` (which tests mismatch). Both have exact field assertions. ✅

2. **Remove `return Err(NoRecoveryData)` branch from `check_workflow_source_digest`**: Would be caught by `header_without_recovery_events_returns_no_recovery_data`. ✅

3. **Change `is_resolved` check to `!is_resolved` in `replay_events`**: The `non_idempotent_pending_action_fails_closed` test would fail (it expects `NonIdempotentActionBlocked`). However, the multi-outcome assertion in `resolved_action_not_reexecuted_on_restart` (accepting both `Ok` AND `Err`) would NOT catch this mutation — it would still pass with `Ok`. ⚠️

4. **Remove seq ordering check in `hydrate_run_frame`**: `snapshot_plus_tail_rejects_tail_before_snapshot` would catch this. ✅

5. **`dimension_count` remove `checked_add(1)` overflow guard**: `frame_dimension_overflow_returns_typed_error` would catch this. ✅

6. **`UnsupportedRecoveryState::union` change `||` to `&&`**: No unit test exists (proptest invariant claimed but nonexistent). No mutation coverage. ⚠️

7. **`recover_runtime_summary` skip terminal event extraction**: `full_journal_reconstructs_exact_pc_steps_slots_taint_terminal` tests exact terminal field. ✅

8. **`recover_all_incomplete_runs` include runs with terminal events**: No test in BDD suite. No coverage. ⚠️

**Surviving mutations:**
- `is_resolved` negation → caught by B-007 test but multi-outcome assertion weakens this
- `union` boolean operator swap → NO test (proptest nonexistent)
- `recover_all_incomplete_runs` terminal inclusion → NO test

---

## Axis 6 — Evidence Plan Audit

**[MINOR]**

The plan's evidence section (Section 3) maps test names to behavior IDs and layers correctly. Preconditions are stated in the "Given:" clauses. Setup side effects (journal creation, event writing) are named explicitly.

However: the verification waiver notes (Section 1) state "Verus BLOCKED_TOOLING" and "TLC WAIVER", which is appropriate documentation but does not replace actual verification.

---

## LETHAL FINDINGS

1. **`recovery_bdd_tests.rs:303`** — `snapshot_plus_tail_applies_tail_after_watermark` has `assert!(result.is_ok())` as its **sole assertion**. The `hydrate_run_frame` call's result is checked only for success; the returned `RunFrame` PC, step count, slots, and taint are never validated. This test passes even if the frame is completely wrong.

2. **Density LETHAL** — 35 tests / 14 contract functions = 2.5x. Target is 5x. Ratio is half the required threshold.

3. **`TerminalStateMismatch` error variant** — Has no test and no formal waiver. The plan acknowledges this as "DEFERRED_GLOBAL" but the open question (item 4) is unresolved. Contract POST-004 requires this variant but the public API cannot trigger it.

---

## MAJOR FINDINGS (3+ triggers REJECTED)

1. **Proptest invariants nonexistent** — Plan claims 4 proptest invariants; zero exist in code. Unit test inventory claimed ~47; zero exist in `summary.rs`/`types.rs`.

2. **Assertion sharpness violations** — Multiple tests use `>= 1` instead of exact values; multi-outcome assertions accept contradictory results.

3. **Mutation survivability gaps** — `UnsupportedRecoveryState::union` boolean mutation has no coverage; `recover_all_incomplete_runs` terminal-event inclusion has no coverage; `is_resolved` negation is weakened by multi-outcome assertion.

4. **`verify_digests` with `DigestCheck::WorkflowSourceOnly` mismatch** — Not explicitly tested through `verify_digests` at that level (only through `check_workflow_source_digest`).

5. **`hydrate_run_frame_from_events` with empty events** — Not explicitly tested (PRE-002 requires `Err(NoRecoveryData)` for empty events).

---

## MINOR FINDINGS (5 threshold)

1. Boundary: `recover_run_admission` has no explicit BDD scenario in the 31-test workdir suite.
2. Boundary: `recover_snapshot_plus_tail` direct usage not tested in isolation.
3. The plan's test count (34) doesn't match workdir (31) or source checkout (35).
4. Workdir test file is 4 tests behind source checkout (ignored tests removed/renamed).
5. `collect_cursor_page_order_survive_via_extra_field` tests `JournalEvent.extra` round-trip but not the full collect hydration path (noted as intentional separation).

---

## MANDATE

The following must exist before resubmission:

1. **Fix `snapshot_plus_tail_applies_tail_after_watermark`** — Add concrete assertions on the returned `RunFrame` after the `is_ok()` guard. Assert `frame.pc()`, `frame.step_count()`, or slot values match expected tail effects.

2. **Increase test density** — At least 70 tests needed (5× × 14 contract functions). The gap is 35 tests. Recommend: unit tests for pure functions in `summary.rs` and `types.rs` (proptest invariants for `union`, `is_resolved`, `dimension_count`, `apply_summary_event`).

3. **Formal waiver for `TerminalStateMismatch`** — Record a proper waiver with clause ID (`POST-004`), reason, compensating evidence (why the error cannot be triggered via public API), and owner. The current "open question" documentation is insufficient.

4. **Write proptest invariants** — Implement the 4 claimed invariants or remove them from the plan. Do not claim coverage that does not exist.

5. **Fix multi-outcome assertions** — `resolved_action_not_reexecuted_on_restart` must assert ONE specific outcome for replay of pre-resolved action.

6. **Add tests for missing boundaries**:
   - `verify_digests` at `WorkflowSourceOnly` level with mismatch
   - `hydrate_run_frame_from_events` with empty events
   - `recover_all_incomplete_runs` including runs with terminal events (negative case)
   - `UnsupportedRecoveryState::union` boolean mutation coverage

7. **Sync workdir to source checkout** — Ensure the test file has the same 35 tests as the source checkout.

---

## Summary

| Finding | Severity | Count |
|---------|----------|-------|
| Bare `is_ok()` with no frame validation | LETHAL | 1 |
| Density ratio 2.5x vs 5x required | LETHAL | 1 |
| TerminalStateMismatch error untested | LETHAL | 1 |
| Proptest/unit tests nonexistent | MAJOR | 1 |
| Assertion sharpness violations | MAJOR | ≥5 |
| Mutation survivability gaps | MAJOR | 3 |
| Missing boundary cases | MAJOR | 3 |
| **Total LETHAL** | | **3** |
| **Total MAJOR** | | **≥12** |

**REJECTED.** Resubmit for full Tier 0 re-run after fixes.

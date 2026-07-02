# Test Suite Review — vb-rpch: recovery_bdd_tests.rs

## Suite Inquisition — Mode 2

### Executive Summary

**Source checkout:** 35 tests pass, 0 ignored.
**Workdir:** 31 tests, 2 `#[ignore]`, 4 tests behind source.
**LETHAL count:** 3 (1 bare `is_ok()` assertion, 1 density failure, 1 missing error variant).
**VERDICT: REJECTED**

---

## Tier 0 — Static Analysis

| Gate | Result | Evidence |
|------|--------|----------|
| Banned pattern scan | **FAIL** | `recovery_bdd_tests.rs:303` — `assert!(result.is_ok())` with no frame validation |
| Determinism/evidence scan | PASS | No shared global mutable state; TempDir per test |
| Mock interrogation | PASS | No mockall/Mock patterns |
| Integration test purity | PASS | Public API only (`vb_storage::recovery::*`, `vb_core::*`) |
| Error variant completeness | **PARTIAL FAIL** | `TerminalStateMismatch` — no test, no formal waiver |
| Density audit | **FAIL** | 35 tests / 14 contract functions = 2.5x (target 5x) |

---

## Tier 1 — Execution

| Gate | Result | Evidence |
|------|--------|----------|
| Test compile | PASS | `cargo test --package vb_storage --test recovery_bdd_tests --no-run` — compiles |
| Tests pass | PASS | 35 passed, 0 failed, 0 flaky |
| Ordering probe | NOT RUN | nextest not available in workdir |
| Insta staleness | NOT RUN | insta not confirmed in vb_storage |

---

## Tier 2 — Coverage

NOT EXECUTED in this review cycle.

---

## Tier 3 — Mutation

NOT EXECUTED in this review cycle.

---

## Critical Discrepancy: Workdir vs Source Checkout

| Metric | Workdir | Source Checkout |
|--------|---------|-----------------|
| `#[test]` count | 31 | 35 |
| `#[ignore]` count | 2 | 0 |
| File line count | 1918 | unknown (larger) |
| Ignored tests | `action_abi_mismatch_returns_typed_error`, `policy_digest_mismatch_returns_typed_error` | removed (source has 35 active tests) |

The workdir is stale by 4 tests. The source checkout tests were run for execution evidence and all pass.

---

## LETHAL Findings

### LETHAL-1: Bare `is_ok()` at `recovery_bdd_tests.rs:303`

**Test:** `snapshot_plus_tail_applies_tail_after_watermark`
**Location:** `crates/vb_storage/tests/recovery_bdd_tests.rs:302-306`

```rust
let result = hydrate_run_frame(&snapshot, &tail, run);
assert!(
    result.is_ok(),
    "hydrate_run_frame should succeed when tail events are after snapshot seq: {result:?}"
);
// ← Function ends here. No validation of the returned RunFrame.
```

The test calls `hydrate_run_frame`, asserts `is_ok()`, and ends. The returned `RunFrame` (PC, step count, slots, taint) is never inspected. A mutation that corrupts the frame but returns `Ok(())` would pass this test.

**Compare:** `snapshot_tail_monotonic_slot_overwrite_preserves_tail_value` (line 1714) does:
```rust
assert!(result.is_ok(), ...);
let frame = result.unwrap();
let slot_value = frame.read_slot(SlotIdx::ZERO);
assert_eq!(slot_value, Ok(&SlotValue::I64(20)), ...);
```
This test is acceptable — the `is_ok()` is a guard, not the endpoint.

**Fix:** Add frame validation after `is_ok()` in `snapshot_plus_tail_applies_tail_after_watermark`.

### LETHAL-2: Density Ratio

35 tests / 14 contract functions = **2.5x**. Target is **5x**. Missing **35 tests**.

The plan claims "8 unit / 18 integration / 4 e2e" but zero unit tests exist in `summary.rs`/`types.rs`. The recovery domain has rich pure function surfaces:
- `apply_summary_event` — 18 event variants, counter monotonicity
- `dimension_count` — overflow boundary
- `UnsupportedRecoveryState::union` — algebraic properties
- `ActionReplayTracker::is_resolved` / `mark_completed` / `mark_failed` — monotonicity
- `recoverable_slot_value`, `legacy_slot_taint` — scalar variant handling

All of these need unit/property tests.

### LETHAL-3: `TerminalStateMismatch` Error Variant

**Contract clause:** POST-004 ("recover_runtime_summary returns `RecoveryHydration::Summary` with accurate counts...and correct `terminal` derived from latest terminal event of max attempt")

**Error taxonomy:** `RecoveryError::TerminalStateMismatch` — "Recovered terminal ≠ expected"

**Problem:** The public API `recover_runtime_summary(journal, run)` takes no `expected_terminal` parameter. There is no way to trigger a mismatch between recovered and expected terminal via the public API.

**Plan status:** Open Question #4 — "Should `recover_runtime_summary_with_expected` variant be added?" Resolved as "DEFERRED_GLOBAL" in proof-obligations JSONL (PO-VB-039). But DEFERRED_GLOBAL requires formal waiver with clause ID, reason, compensating evidence, and owner. The plan only has an open question, not a formal waiver.

**Comment in test file (lines 1859-1869):**
```
// NOTE: REMOVED — LETHAL-3: TerminalStateMismatch error path not reachable via
// public API recover_runtime_summary. The function takes no expected-terminal
// parameter, so a mismatch cannot be triggered without API addition.
```

**Fix:** Either add `recover_runtime_summary_with_expected(run, expected_terminal)` to the public API, or record a formal waiver per the rules.

---

## MAJOR Findings

### MAJOR-1: Proptest Invariants Nonexistent

The plan (Section 4) lists 4 proptest invariants with function names, strategies, and anti-invariants. Zero exist in code:
```bash
$ rtk grep -rn "proptest!" crates/vb_storage/src/recovery/
# (no matches)
```

### MAJOR-2: Unit Test Inventory Fabricated

Plan Section 9: "~47 unit tests in `summary.rs::tests`, `types.rs::tests`"
Reality:
```bash
$ rtk grep -c "^#\[test\]" crates/vb_storage/src/recovery/replay/summary.rs  # 0
$ rtk grep -c "^#\[test\]" crates/vb_storage/src/recovery/types.rs           # 0
```

### MAJOR-3: Assertion Sharpness Violations

| Test | Lines | Problem |
|------|-------|---------|
| `wait_identity_and_state_survive_across_restart` | ~479 | `suspensions >= 1` — vague |
| `unsequenced_lifecycle_events_do_not_change_recovered_state` | ~1043-1046 | `steps_started >= 1` — vague |
| `resolved_action_not_reexecuted_on_restart` | ~653-661 | Accepts BOTH `Ok` AND `Err(NonIdempotentActionBlocked)` — not sharp |
| `same_journal_and_snapshot_replayed_twice_equivalent` | ~825 | Multi-outcome assertion — not sharp |
| `unsequenced_lifecycle_events_do_not_change_recovered_state` | ~1047-1054 | Multi-outcome for terminal — not sharp |

### MAJOR-4: Mutation Survivability Gaps

1. **`UnsupportedRecoveryState::union`** boolean mutation (|| → &&) — no coverage (proptest nonexistent)
2. **`recover_all_incomplete_runs`** including terminal-event runs — no test
3. **`is_resolved` negation** — weakened by multi-outcome assertion in `resolved_action_not_reexecuted_on_restart`

### MAJOR-5: Missing Boundary Cases

- `verify_digests` at `DigestCheck::WorkflowSourceOnly` level with mismatch (only tested via `check_workflow_source_digest`)
- `hydrate_run_frame_from_events` with empty events (PRE-002: returns `Err(NoRecoveryData)`)
- `recover_all_incomplete_runs` negative case (including runs that DO have terminal events)

---

## MINOR Findings

1. Workdir test file 4 tests behind source checkout
2. `recover_run_admission` — no explicit BDD scenario in 31-test workdir suite
3. `recover_snapshot_plus_tail` — not tested in isolation
4. Test count discrepancy: plan says 34, workdir has 31, source has 35

---

## MANDATE

Every item must be resolved before resubmission for full Tier 0 re-run:

1. **LETHAL-1 fix:** Add frame validation after `is_ok()` in `snapshot_plus_tail_applies_tail_after_watermark`
2. **LETHAL-2 fix:** Write 35 additional tests (target: 70 total, 5× × 14 functions). Focus on pure functions: `apply_summary_event`, `dimension_count`, `UnsupportedRecoveryState::union`, `ActionReplayTracker` methods, `recoverable_slot_value`, `legacy_slot_taint`
3. **LETHAL-3 fix:** Either add `recover_runtime_summary_with_expected` API variant, or record formal DEFERRED_GLOBAL waiver with clause ID, reason, compensating evidence, and owner
4. **MAJOR-1 fix:** Implement 4 proptest invariants OR remove from plan
5. **MAJOR-3 fix:** Replace vague `>= 1` with exact counts; fix multi-outcome assertions
6. **MAJOR-5 fix:** Add tests for missing boundary cases
7. **Sync workdir** to source checkout (35 tests, no ignores)

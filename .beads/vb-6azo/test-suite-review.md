# Test Suite Review: vb-6azo

**Bead:** vb-6azo — Behavioral property tests for workflow engine invariants
**State:** 10 (test-suite-review)
**Reviewer:** test-reviewer (Suite Inquisition Mode)
**Date:** 2026-05-09

---

## STATUS: REJECTED

---

### Tier 0 — Static

| Check | Result |
|-------|--------|
| Banned pattern scan | PASS — No `assert!(result.is_ok/is_err())` found |
| Silent error discard | MINOR — `let _ =` on write_slot in test setup (tests.rs:610,1179, etc.) |
| Ignored tests | PASS — None found |
| Test naming violations | **FAIL** — `fn test_` prefix in shard/tests.rs:6804,6833,6875,6893,6909,6933 |
| Holzmann rule scan | **FAIL** — `while` loop in proptest body at tests.rs:1491 |
| Mock interrogation | PASS — No mocks found |
| Integration test purity | PASS — No `use crate::` in tests/ |
| Error variant completeness | PASS — RuntimeEngineError and RuntimeError variants have coverage |
| Density audit | INCONCLUSIVE — Cannot verify due to clippy failures in Tier 1 |
| Insta dependency | ABSENT |

### Tier 1 — Execution

| Gate | Result |
|------|--------|
| Clippy | **FAIL** — 219 errors, 2 warnings, exit code 101 |
| nextest | PASS — 1355 passed, 0 failed, 0 flaky |
| Ordering probe | NOT RUN — Tier 1 failed |
| Insta | N/A |

### Tier 2 — Coverage

Not reached (Tier 1 failed).

### Tier 3 — Mutation

Not reached (Tier 1 failed).

---

## LETHAL FINDINGS

### 1. `crates/vb_runtime/src/engine/property_tests.rs` — FILE STILL EMPTY

The designated test file contains exactly **1 byte** (a single newline). Despite the repair claim "property_tests.rs filled", the file is empty.

**Contract requirement (contract.md §6.4):**
> All tests go in: `crates/vb_runtime/src/engine/property_tests.rs`

**Required tests (contract.md §6.2):**
1. `evidence_chain_ordering_preserved`
2. `budget_exhaustion_stops_at_exact_boundary`
3. `frame_pool_capacity_never_exceeded`
4. `frame_pool_dimension_mismatch_silent_drop`
5. `frame_reuse_clears_all_prior_state`
6. `command_queue_full_boundary`
7. `one_command_per_tick_enforced`
8. `shutdown_terminates_tick_loop`
9. `step_state_transition_validity`
10. `evidence_drain_resets_dropped_counter`
11. `compute_max_parallel_rejects_overflow`
12. `zero_capacity_collector_drops_all`
13. `run_lifecycle_submit_cancel_exclusivity`
14. `mark_step_rejects_invalid_state_transitions`

**None of the 14 required tests exist.** A `proptest-regressions/engine/property_tests.txt` exists (evidence that proptest was run), but the test file itself is empty.

### 2. `crates/vb_runtime/src/engine/tests.rs:1491` — WHILE LOOP IN PROPTEST BODY

```rust
while !drained && taken <= n + 1 {
    match budget.try_take() {
        Ok(true) => taken += 1,
        Ok(false) => drained = true,
        Err(_) => drained = true,
    }
}
```

**Holzmann Rule 2 violation:** No unbounded loops in test bodies. This is inside a `proptest!` block at line 1485.

### 3. `crates/vb_runtime/src/engine/drive.rs:413` — 112× PANIC! IN RESULT FN

Clippy `panic_in_result_fn`: Functions `emit_return_signal` and others use `panic!()` instead of returning `Result`. This causes 112 clippy errors.

---

## MAJOR FINDINGS (5)

1. **Clippy: 219 errors → 0** — Still far from clean; previous 258 errors reduced to 219 but still LETHAL
2. **`fn test_` naming** (shard/tests.rs:6804,6833,6875,6893,6909,6933) — Old-style test names violate naming conventions
3. **`let _ =` on fallible writes** (tests.rs:610,1179,1886,1925,1969,2090,2282,2355,2402) — Silent discard in test setup
4. **`unwrap()` on Result** (action.rs:541,570,591) — Three unwrap calls in implementation code
5. **`expect()` on Option** (shard/tests.rs:6206,6244,6252, +48 more in retry.rs) — 51 `expect()` calls on Result/Option

---

## MINOR FINDINGS (3/5 threshold)

1. `action.rs:542,571` — `must_use` Result discarded without assertion
2. `tests.rs:665,683,709` — Unused mutable `run` variable (7 instances)
3. `drive.rs:836` — `let _ =` discard of `must_use` result

---

## MANDATE

### P0 — Must fix before any tier passes

1. **`property_tests.rs` must contain all 14 tests** — Fill `crates/vb_runtime/src/engine/property_tests.rs` with all tests specified in contract.md §6.2
2. **Remove `while` loop** at tests.rs:1491 — Rewrite using proptest strategies (`prop_while`, or restructure as deterministic finite iteration)
3. **Clippy: 219 errors → 0** — Fix all `panic_in_result_fn`, `expect_used`, `ok().expect()`, `must_use`, `unused_mut`, `unused_variable` violations

### P1 — Required before APPROVED

4. Rename `fn test_*` functions in shard/tests.rs to `fn subdoc_*` or `fn it_` style
5. Address all remaining clippy violations across vb_runtime

### Verification

After any fix: re-run ALL tiers from Tier 0. Full re-run. Always.

---

*test-reviewer | vb-6azo | State 10 | REJECTED*

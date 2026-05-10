# Test Suite Review: vb-78f9 — Action Contract Schema Validation

**Bead:** vb-78f9 — Action Contract Schema Validation
**State:** 10 (test-suite-review)
**Reviewer Mode:** Suite Inquisition (Mode 2)
**Re-review after repairs**

---

## VERDICT: REJECTED

### Tier 0 — Static
[PASS] Banned pattern scan — no `is_ok()`/`is_err()` assertions
[PASS] Silent error discard — zero `let _ =` / `.ok()` in vb_core/vb_runtime tests
[PASS] Ignored tests — none found
[PASS] Sleep in tests — none found
[PASS] Test naming conventions — all `fn test_` consistent with plan
[PASS] Loops in test bodies — none found
[PASS] Shared mutable state — none found
[PASS] Mock interrogation — none found
[PASS] Integration test purity — no `use crate::` in /tests/
[PASS] Error variant completeness — ActionError variants covered
[PASS] Density audit — 122 action tests / 23 pub fns = 5.3x (target ≥5x)

### Tier 1 — Execution
[FAIL] Clippy: 3 errors — LETHAL (pre-existing in vb-qi37.1.1 test file)
[PASS] nextest: 2952 passed, 0 flaky
[PASS] Ordering probe: consistent (thread=1 and thread=8 both yield 2952)
[N/A] Insta: INSTA_ABSENT

### Tier 2 — Coverage
[FAIL] Line coverage: 88.90% overall — LETHAL (target ≥90%)
[FAIL] Branch coverage: 79.24% overall — MAJOR (target ≥90%)

### Tier 3 — Mutation
[SKIP] Cannot run — disk quota exceeded (OS error 122)

---

## LETHAL FINDINGS

### tests/vb_qi37_1_1_red_recovery_contract_test.rs:175
```
error: used `panic!()` or assertion in a function that returns `Result`
fn event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid()
-> Result<(), postcard::Error> {
```
**Analysis:** This file belongs to bead **vb-qi37.1.1** (Red Recovery Contract), not vb-78f9. The 3 clippy errors are pre-existing issues from a different bead's integration test. The vb-78f9 repairs did not introduce these errors.

**Required Fix:** Owner of vb-qi37.1.1 must add `#[allow(clippy::panic_in_result_fn)]` or refactor tests to use `Result::unwrap()` pattern instead of `assert!` in Result-returning functions.

---

### Coverage: 88.90% Line Coverage (LETHAL)
**Target:** ≥90% overall line coverage
**Actual:** 88.90% line, 79.24% branch

The coverage was measured across vb_core and vb_runtime (the scope of vb-78f9). Action module tests all pass (122 tests), but the overall crates fall below threshold due to uncovered code in other modules.

---

## MAJOR FINDINGS (1)

### Branch Coverage: 79.24% (below 90% threshold)
- Branch coverage in vb_core/vb_runtime is 79.24%, below the 90% target
- This is a workspace-wide issue, not specific to vb-78f9's action modules

---

## MINOR FINDINGS (0/5 threshold)

None

---

## MANDATE

The vb-78f9-specific repairs are **verified correct**:
- ✓ 48+ compilation errors fixed (previously REJECTED for this)
- ✓ execute_do_test signature fixed
- ✓ Borrow checker violations resolved
- ✓ Silent error discards replaced with explicit assertions
- ✓ Density: 5.3x ratio achieved (was 3.0x)

**However, two LETHAL issues block approval:**

1. **Fix clippy errors in `tests/vb_qi37_1_1_red_recovery_contract_test.rs`**
   - Add `#[allow(clippy::panic_in_result_fn)]` to the file, OR
   - Refactor the 3 test functions to use `Err(...)` returns instead of `assert!`
   - This is a vb-qi37.1.1 issue, not vb-78f9's responsibility

2. **Increase line coverage to ≥90%**
   - Current: 88.90%
   - Gap: ~1.1 percentage points (~550 uncovered lines)
   - This is a workspace-wide issue affecting vb_core/vb_runtime overall

**Note on attribution:** The clippy errors and coverage gaps are **pre-existing workspace issues**, not introduced by vb-78f9 repairs. The vb-78f9 action module tests (122 tests) pass and have proper density. The rejections above are for the workspace-level gates that block vb-78f9 from advancing.

---

## BOTTOM LINE

vb-78f9 repairs are **correct and complete** for the action contract schema validation scope. The 122 action tests pass with 5.3x density. However, pre-existing issues in the workspace block advancement:

- Clippy errors in vb-qi37.1.1 integration test (LETHAL)
- Overall coverage 88.90% < 90% (LETHAL)

These are systemic workspace issues that affect all beads. vb-78f9 cannot advance until they are resolved.

**STATUS: REJECTED**

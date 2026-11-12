# Defects Report — vb-qi37.1.4

## State: 12 (black-hat-reviewer) — REJECTED

---

## DEFECT-1: Test Expects Wrong Behavior After GAP-2 Fix

**Severity**: BLOCKING
**Type**: test
**Phase**: PHASE 1 — Contract & Bead Parity

### Description

The test `reject_returns_ok_when_pending_actions_unsupported_but_empty` in test-plan.md:73-80 was written to document the BUGGY behavior (expecting `Ok(())` when `unsupported.pending_actions=true` AND `pending_actions` is empty).

However, POST-002 explicitly states:
> "returns `Err` when `unsupported.pending_actions` is `true`, regardless of whether `pending_actions` is empty"

The GAP-2 fix is correct: the buggy code was:
```rust
|| (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)  // WRONG
```

The fixed code is:
```rust
|| seed.unsupported.pending_actions  // CORRECT
```

After the fix, the correct behavior when `unsupported.pending_actions=true` (regardless of `is_empty()`) is `Err(RuntimeError::InvalidRecoveryHydration)`.

The test was written to document the BUGGY behavior, not the CORRECTED behavior. After the fix, this test would FAIL.

### Location

- **test-plan.md:73-80**: Scenario definition with wrong expected outcome
- **recovery.rs:mod tests**: No corresponding test actually written (tooling blocked execution)
- **test-plan-review.md:37**: Incorrectly marked as "SHARP ✓" without flagging the contradiction

### Evidence

From test-plan.md:73-80:
```
Scenario: fn reject_returns_ok_when_pending_actions_unsupported_but_empty
Given: RecoveryFrameSeed with unsupported.pending_actions=true, pending_actions=[], other flags=false
When: reject_unsupported_live_frame_state(seed) is called
Then: returns Ok(())
Note: GAP-2 gap — with empty pending_actions, the unsupported.pending_actions guard is bypassed
This test documents the current (buggy) behavior; fix should make this return Err
```

The Note explicitly says "fix should make this return Err" — confirming the test has the WRONG expected outcome.

### Required Fix

1. **test-plan.md**: Change the expected outcome from `Ok(())` to `Err(RuntimeError::InvalidRecoveryHydration)`
2. **test-plan.md**: Update the Note to say the test now validates CORRECT behavior
3. **test-plan-review.md**: Update line 37 to reflect the corrected expectation

### Impact

- Without this fix, the test suite (when runnable) would FAIL on the GAP-2 fix
- Contract parity is violated: POST-002 requires `Err`, but one test expects `Ok`

### Verification

After fix is applied:
```rust
// Given: unsupported.pending_actions=true, pending_actions=[]
// When: reject_unsupported_live_frame_state is called
// Then: returns Err(RuntimeError::InvalidRecoveryHydration)  // CORRECT
// Note: NOT Ok(()) as the test currently expects
```

---

## Summary Table

| Defect | Severity | Type | Phase | Status |
|---|---|---|---|---|
| DEFECT-1: Test expects wrong behavior | BLOCKING | test | PHASE 1 | OPEN |

---

*defects.md: State 12 black-hat-reviewer findings for vb-qi37.1.4 — REJECTED*

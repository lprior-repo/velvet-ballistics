# vb-99n6-ws — RED PHASE Test Workspace

This workspace contains RED PHASE tests for bead vb-99n6 (Timer Wheel Driven Resume and Cancellation Hardening).

## Test Files

- `crates/vb_runtime/src/shard/tests.rs` — Main test file with all test categories

## Test Categories

### Timer Wheel Unit Tests (TW-UT-*)
- TW-UT-001 through TW-UT-013
- Tests for TimerWheel dual-index consistency, insert/replace/cancel/fire behavior

### Helpers Unit Tests (HP-UT-*)
- HP-UT-001 through HP-UT-005
- Tests for `timer_registration_required` and `advance_after_timer_fire`

### Integration Tests (IT-*)
- IT-TIMER-001: Timer fire advances WaitUntil to completion
- IT-TIMER-004: TimerFired on unknown run returns RunNotFound
- IT-TIMER-005: TimerFired on run with no pending timer returns InvalidTimerFire
- IT-TIMER-006: TimerFired after cancel returns RunNotFound
- IT-RESUME-001: Resume re-drives action-suspended run
- IT-RESUME-002: Resume re-drives wait-suspended run without consuming timer
- IT-RESUME-004: Resume on unknown run returns RunNotFound
- IT-CANCEL-001: Cancel removes run and timer atomically
- IT-CANCEL-002: Cancel on non-existent run succeeds silently
- IT-CANCEL-003: Duplicate cancel is idempotent

### Property Tests (PB-*)
- PB-TW-001 through PB-TW-005: Dual-index consistency and replacement invariants
- PB-SM-001: At most one timer per run
- PB-GLOBAL-002: Cancel idempotency

## Running Tests

To run these tests in the main workspace:

```bash
cd /home/lewis/src/Velvet-ballistics
cargo test -p vb_runtime --test-threads=1
```

To run specific test categories:

```bash
# Timer wheel tests
cargo test -p vb_runtime timer_wheel

# Integration tests
cargo test -p vb_runtime integration_tests

# Property tests
cargo test -p vb_runtime property_tests
```

## Expected Behavior

These tests are written to FAIL against the current implementation (RED PHASE).
They define the expected behavior per the contract in `contract.md`.

Once the implementation is corrected, these tests should pass.

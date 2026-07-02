# Test Repair Guide: vb-f7k6

STATUS: REJECTED

## Required Repairs

1. Replace the false stale-replacement test.
   - Current fault: `chunk_029.rs:20-30` does not replace the timer.
   - Required shape: capture old authority, perform an actual replacement/reschedule that yields distinct authority, deliver old authority, assert exact `Err(RuntimeError::InvalidTimerFire)`, assert the new timer remains exactly current, and assert no run progress/ack/resurrection.

2. Replace Debug-string metadata checks with typed authority checks.
   - Current fault: `chunk_029.rs:102-108` and `120-127` only search Debug output.
   - Required shape: tests must fail to compile or fail behaviorally until `TimerFired`/emitted entries expose usable `(generation, deadline, kind)` or an opaque token derived from them.
   - Add typed mismatch tests for wrong generation, wrong deadline, and wrong kind once fields/token exist.

3. Remove assertion-skipping setup returns.
   - Current fault: `chunk_029.rs:7-9`, `39-41`, `71-73` can pass without exercising behavior.
   - Required shape: make missing workflow an exact failure, e.g. assert `Some(expected_workflow)` before continuing or use a non-optional fixture.

4. Strengthen no-resurrection evidence.
   - Cancel and terminal stale-fire tests must assert exact observable run/timer state, not only `pending_timers.len()` and `runs_completed`.

## Required Re-Review Evidence

- `/usr/bin/env cargo test -p vb_runtime --no-run` passes.
- `/usr/bin/env cargo test -p vb_runtime timer_fired` is red only for genuine missing production authority, not because a valid timer is mislabeled stale.
- `/usr/bin/env cargo test -p vb_runtime timer_wheel_fired_entry_carries_freshness_metadata_for_runtime_validation` is replaced by typed authority evidence or justified as a temporary compile-fail/red structural gate.
- Static scan shows no early-return assertion skips in changed tests.

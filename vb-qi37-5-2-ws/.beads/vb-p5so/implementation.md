bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 6
updated_at: 2026-05-09T00:00:00Z

## State 5: TDD Red Phase
- Status: COMPLETE
- Evidence: 2/6 new tests FAILED as expected
  - `test_drain_for_shutdown_removes_all_pending_timers_and_returns_them`: FAILED (pending_timers.len() == 1, expected 0)
  - `test_drain_for_shutdown_handles_timers_without_valid_backing_runs_gracefully`: FAILED (pending_timers.len() == 1, expected 0)
  - 4 other tests PASSED (capacity limit, idempotency, empty state, mixed kinds)
- Retry budget remaining: 7

## State 6: Implementation
- Target: `crates/vb_runtime/src/shard/impl_.rs` line 331-341
- Change: Add `self.pending_timers.clear()` after shutdown is detected in `drain_for_shutdown()`
- Rationale: When `tick()` returns `false` (shutdown command processed), `drain_for_shutdown` returns `Ok(())` but leaves `pending_timers` populated. The zero-leak graceful shutdown contract requires all pending suspended timers to be evicted.
- No other files require changes.
- Implementation constraint: Must not affect the capacity-limit path (Err(ShutdownInProgress)) where shutdown was NOT confirmed.

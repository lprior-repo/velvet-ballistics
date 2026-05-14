bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Test Plan Review — Mode 1: Plan Inquisition

## Review Criteria (Six Axes)

### Axis 1 — Contract Parity
- `drain_for_shutdown` has 6 BDD scenarios covering all postconditions. ✓
- `Err(ShutdownInProgress)` has explicit scenario asserting exact variant. ✓
- All contract clauses (P1-P3, PO1-PO4, I1-I4) are covered by at least one scenario. ✓

### Axis 2 — Assertion Sharpness
- Then clauses use exact values: `Ok(())`, `pending_timers.len() == 0`, `Err(ShutdownInProgress)`, `shutting_down == true`. ✓
- No `is_ok()` or `is_err()` without specifying the inner value. ✓
- No `Some(_)` without inner value specification. ✓

### Axis 3 — Trophy Allocation
- 6 tests for 1 modified public function = 6× coverage (> 5× threshold). ✓
- No pure function with non-trivial input space (this is a side-effecting method). N/A.
- No parser/deserializer boundaries. N/A.
- Integration/unit ratio 2:4 is reasonable for an internal state mutation fix. ✓

### Axis 4 — Boundary Completeness
- Empty input: `test_drain_for_shutdown_handles_empty_timer_state` ✓
- Capacity limit: `test_shutdown_is_processed_successfully_even_when_timer_queue_is_full` ✓
- Mixed kinds: `test_drain_for_shutdown_clears_mixed_wait_and_ask_timers` ✓
- Orphaned entries: `test_drain_for_shutdown_handles_timers_without_valid_backing_runs_gracefully` ✓
- Idempotency: `test_calling_drain_for_shutdown_repeatedly_is_idempotent` ✓
- All boundaries covered. ✓

### Axis 5 — Mutation Survivability
- Delete `.clear()` call → caught by behavior 1 ✓
- Move `.clear()` before loop → caught by behavior 2 ✓
- Replace `.clear()` with partial remove → caught by behavior 6 ✓
- All critical mutations have catching tests. ✓

### Axis 6 — Holzmann Plan Audit
- Preconditions explicitly stated in Given clauses. ✓
- No iteration loops in test logic (no ceiling needed). ✓
- Side effects in setup named explicitly. ✓

## Findings
- LETHAL: 0
- MAJOR: 0
- MINOR: 0

STATUS: APPROVED

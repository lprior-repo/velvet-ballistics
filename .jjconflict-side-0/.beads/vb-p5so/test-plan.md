bead_id: vb-p5so
bead_title: "runtime: Forcefully clear pending suspended timers on drain_for_shutdown"
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Test Plan: Forcefully Clear Pending Timers on drain_for_shutdown

## Summary
- Behaviors identified: 6
- Trophy allocation: 4 unit / 2 integration / 0 e2e / 0 static (deviation: 100% unit because this is a single-method state-mutation bug fix on an internal data structure)
- Proptest invariants: 1
- Fuzz targets: 0 (no parsing boundaries)
- Kani harnesses: 0 (no unsafe, no arithmetic, no state machine transitions)

## 1. Behavior Inventory

1. `[Shard] [clears all pending timers] [when drain_for_shutdown processes a Shutdown command]`
2. `[Shard] [leaves pending timers unchanged] [when drain_for_shutdown hits capacity limit before Shutdown]`
3. `[Shard] [is idempotent] [when drain_for_shutdown is called twice]`
4. `[Shard] [handles empty pending_timers gracefully] [when drain_for_shutdown runs with no suspended runs]`
5. `[Shard] [handles timers without valid backing runs gracefully] [when a run was already cancelled but timer entry remains]`
6. `[Shard] [clears both Wait and Ask timers] [when multiple runs have mixed timer kinds]`

## 2. Trophy Allocation

| Layer | Count | Justification |
|-------|-------|---------------|
| Unit | 4 | All behaviors are directly observable on Shard's internal state |
| Integration | 2 | Full workflow: submit → suspend → shutdown → assert |
| E2E | 0 | No external API boundary change |
| Static | 0 | No new error variants, no parsing |

## 3. BDD Scenarios

### Behavior 1: drain_for_shutdown clears all pending timers
```
Given: A shard with active runs that have pending Wait and Ask timers
When: drain_for_shutdown is called and processes a Shutdown command
Then:
  - drain_for_shutdown returns Ok(())
  - pending_timers.len() == 0
  - shutting_down == true
```
Test name: `test_drain_for_shutdown_removes_all_pending_timers_and_returns_them`

### Behavior 2: drain_for_shutdown leaves timers unchanged on capacity limit
```
Given: A shard with a full command queue and pending timers, but no Shutdown command
When: drain_for_shutdown is called
Then:
  - drain_for_shutdown returns Err(ShutdownInProgress)
  - pending_timers.len() == original count
  - shutting_down == false
```
Test name: `test_shutdown_is_processed_successfully_even_when_timer_queue_is_full`

### Behavior 3: drain_for_shutdown is idempotent
```
Given: A shard that has already been shut down (shutting_down == true)
When: drain_for_shutdown is called a second time
Then:
  - drain_for_shutdown returns Ok(())
  - pending_timers.len() == 0
  - No panic or error
```
Test name: `test_calling_drain_for_shutdown_repeatedly_is_idempotent`

### Behavior 4: drain_for_shutdown handles empty timer state
```
Given: A shard with no pending timers
When: drain_for_shutdown is called and processes Shutdown
Then:
  - drain_for_shutdown returns Ok(())
  - pending_timers.len() == 0
  - shutting_down == true
```
Test name: `test_drain_for_shutdown_handles_empty_timer_state`

### Behavior 5: drain_for_shutdown handles orphaned timer entries
```
Given: A shard where pending_timers has an entry but the run no longer exists in self.runs
When: drain_for_shutdown is called and processes Shutdown
Then:
  - drain_for_shutdown returns Ok(())
  - pending_timers.len() == 0
  - No panic
```
Test name: `test_drain_for_shutdown_handles_timers_without_valid_backing_runs_gracefully`

### Behavior 6: drain_for_shutdown clears mixed timer kinds
```
Given: A shard with runs suspended on both Wait and Ask timers
When: drain_for_shutdown is called and processes Shutdown
Then:
  - drain_for_shutdown returns Ok(())
  - pending_timers.len() == 0
  - All timer entries removed regardless of kind
```
Test name: `test_drain_for_shutdown_clears_mixed_wait_and_ask_timers`

## 4. Proptest Invariants

### Proptest: pending_timer_count_after_shutdown
Invariant: For any valid Shard state, after `drain_for_shutdown` returns `Ok(())`, `pending_timer_count() == 0`
Strategy: Generate shards with 0..MAX_ACTIVE_RUNS pending timers of random kinds
Anti-invariant: N/A (should always hold)

## 5. Fuzz Targets
None — no parsing or deserialization boundaries are touched.

## 6. Kani Harnesses
None — the change is a single safe `IndexMap::clear()` call. No arithmetic, no unsafe, no concurrent state.

## 7. Mutation Checkpoints

| Critical Mutation | Catching Test |
|---|---|
| Remove `.clear()` call from `drain_for_shutdown` | `test_drain_for_shutdown_removes_all_pending_timers_and_returns_them` |
| Move `.clear()` before the loop | `test_shutdown_is_processed_successfully_even_when_timer_queue_is_full` |
| Replace `.clear()` with `.swap_remove(&single_run)` | `test_drain_for_shutdown_clears_mixed_wait_and_ask_timers` |

Threshold: 100% mutation kill rate (3 mutations, 3 tests).

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: shutdown clears timers | Shard with pending timers | Ok(()) + pending_timers == 0 | unit |
| Error: capacity limit | Full queue, no shutdown | Err(ShutdownInProgress) + timers unchanged | unit |
| Edge: empty timers | No pending timers | Ok(()) + pending_timers == 0 | unit |
| Edge: idempotent | Already shut down | Ok(()) + pending_timers == 0 | unit |
| Edge: orphaned entries | Timer without run | Ok(()) + pending_timers == 0 | unit |
| Boundary: mixed kinds | Wait + Ask timers | Ok(()) + pending_timers == 0 | unit |

## Open Questions
None.

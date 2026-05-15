bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Test Plan: Hydrate RunFrame from Snapshot + Journal

## Summary
- Behaviors identified: 19
- Trophy allocation: 24 unit / 4 integration / 0 e2e
- Proptest invariants: 3
- Fuzz targets: 1
- Kani harnesses: 8
- Mutation threshold: ≥90%

## 1. Behavior Inventory

1. [Hydration] Hydrates a faithful RunFrame from valid snapshot + tail events
2. [Hydration] Hydrates a RunFrame from events only (no snapshot)
3. [Rejection] Rejects snapshot with mismatched run_id
4. [Rejection] Rejects tail events belonging to a different run
5. [Rejection] Rejects tail event with seq <= snapshot.seq
6. [Rejection] Rejects corrupt snapshot bytes
7. [Rejection] Rejects when both snapshot and events are empty/missing
8. [Rejection] Rejects when derived step_count is zero
9. [Rejection] Rejects when dimensions overflow u16
10. [State] PC is set from the last state-affecting event
11. [State] Step states reflect snapshot base + tail transitions
12. [State] Slots reflect snapshot base overwritten by tail SlotWrittenEvents
13. [State] Taint reflects snapshot base overwritten by tail SlotWrittenEvents
14. [State] Executed counter equals count of applied tail events
15. [State] Parallel in-flight is reconstructed from action events
16. [Invariant] Dimension integrity: arrays match declared counts
17. [Invariant] Slot-taint parity for all initialized slots
18. [Invariant] Deterministic: same inputs → same output
19. [Invariant] No silent defaults on missing/corrupt data

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| Unit | 24 | Pure calc layer: frame construction, event application, error mapping |
| Integration | 4 | Real FjallJournal roundtrip: snapshot write → read → hydrate |
| E2E | 0 | No user-facing surface; this is an internal runtime boundary |
| Static | project | Clippy, forbid(unsafe_code), zero-unwrap lint gates |

Deviation: E2E is 0% because this is a library-internal recovery function with no CLI/API exposure.

## 3. BDD Scenarios

### Behavior: Hydrates faithful RunFrame from snapshot + tail events
```
Given: A valid RunSnapshot with 2 slots (values [42, null], taint [Clean, Secret])
  and tail events: StepStarted(step=0), StepSucceeded(step=0, output=0)
When: hydrate_run_frame(snapshot, tail_events, run_id) is called
Then: Returns Ok(RunFrame)
  and frame.run_id() == run_id
  and frame.step_count() == 1
  and frame.slot_count() == 2
  and frame.pc() == StepIdx::new(0)
  and frame.step_state(0) == StepState::Succeeded
  and frame.read_slot(0) == SlotValue::I64(42)
  and frame.read_taint(0) == Taint::Clean
  and frame.read_taint(1) == Taint::Secret
```
Test name: `hydrate_run_frame_reconstructs_frame_from_snapshot_and_tail_events`

### Behavior: Hydrates RunFrame from events only
```
Given: No snapshot (empty bytes)
  and events: StepStarted(step=0), StepSucceeded(step=0, output=0), SlotWrittenEvent(slot=0, value=I64(7))
When: hydrate_run_frame_from_events(events, run_id) is called
Then: Returns Ok(RunFrame)
  and frame.step_count() == 1
  and frame.slot_count() == 1
  and frame.read_slot(0) == SlotValue::I64(7)
```
Test name: `hydrate_run_frame_from_events_reconstructs_without_snapshot`

### Behavior: Rejects snapshot with mismatched run_id
```
Given: A snapshot with run_id=1
  and requested run_id=2
When: hydrate_run_frame(snapshot, [], run_id=2) is called
Then: Returns Err(RecoveryError::ReplayDivergence)
  and error detail contains "snapshot run_id mismatch"
```
Test name: `hydrate_run_frame_rejects_mismatched_snapshot_run_id`

### Behavior: Rejects tail events for wrong run
```
Given: A valid snapshot for run_id=1
  and a tail event with run_id=2
When: hydrate_run_frame(snapshot, [event], run_id=1) is called
Then: Returns Err(RecoveryError::ReplayDivergence)
```
Test name: `hydrate_run_frame_rejects_tail_event_for_wrong_run`

### Behavior: Rejects tail event seq <= snapshot.seq
```
Given: A snapshot with seq=10
  and a tail event with seq=9
When: hydrate_run_frame(snapshot, [event], run_id) is called
Then: Returns Err(RecoveryError::ReplayDivergence)
  and error detail contains "tail event seq not after snapshot"
```
Test name: `hydrate_run_frame_rejects_tail_event_before_snapshot_seq`

### Behavior: Rejects corrupt snapshot bytes
```
Given: A snapshot with slots_bytes = [0xFF, 0xFF] (invalid postcard)
  and valid taint_bytes
When: hydrate_run_frame(snapshot, [], run_id) is called
Then: Returns Err(RecoveryError::CorruptSnapshot)
```
Test name: `hydrate_run_frame_rejects_corrupt_snapshot_slots_bytes`

### Behavior: Rejects empty snapshot and empty events
```
Given: Empty snapshot bytes and empty tail events
When: hydrate_run_frame(snapshot, [], run_id) is called
Then: Returns Err(RecoveryError::NoRecoveryData)
```
Test name: `hydrate_run_frame_rejects_empty_snapshot_and_empty_events`

### Behavior: Rejects zero step count
```
Given: Events with no step references (only RunAccepted)
When: hydrate_run_frame_from_events(events, run_id) is called
Then: Returns Err(RecoveryError::InvalidCompiledWorkflow { reason: "step_count_zero" })
```
Test name: `hydrate_run_frame_from_events_rejects_zero_step_count`

### Behavior: PC from last step event
```
Given: Events: StepStarted(0), StepSucceeded(0), StepStarted(1)
When: hydrate_run_frame_from_events(events, run_id) is called
Then: frame.pc() == StepIdx::new(1)
```
Test name: `hydrate_run_frame_pc_set_from_last_step_event`

### Behavior: Step states reflect snapshot + tail
```
Given: Snapshot with step 0 in Succeeded state
  and tail: StepStarted(1)
When: hydrate_run_frame(snapshot, tail, run_id) is called
Then: frame.step_state(0) == StepState::Succeeded
  and frame.step_state(1) == StepState::Running
```
Test name: `hydrate_run_frame_states_merge_snapshot_and_tail`

### Behavior: Slots overwritten by tail events
```
Given: Snapshot with slot 0 = I64(1)
  and tail: SlotWrittenEvent(slot=0, value=I64(2))
When: hydrate_run_frame(snapshot, tail, run_id) is called
Then: frame.read_slot(0) == SlotValue::I64(2)
```
Test name: `hydrate_run_frame_slots_overwritten_by_tail_events`

### Behavior: Taint overwritten by tail events
```
Given: Snapshot with slot 0 taint = Clean
  and tail: SlotWrittenEvent(slot=0, value=I64(2)) — no explicit taint in event
When: hydrate_run_frame(snapshot, tail, run_id) is called
Then: frame.read_taint(0) == Taint::Clean  (preserved from snapshot)
```
Test name: `hydrate_run_frame_taint_preserved_when_tail_has_no_taint`

### Behavior: Executed counter from tail events
```
Given: Valid snapshot
  and tail: StepStarted(0), StepSucceeded(0), SlotWrittenEvent(0)
When: hydrate_run_frame(snapshot, tail, run_id) is called
Then: frame.executed() == 3
```
Test name: `hydrate_run_frame_executed_counter_matches_tail_event_count`

### Behavior: Parallel in-flight from action events
```
Given: Events: ActionScheduled(action=1, step=0), ActionScheduled(action=2, step=1), ActionCompletedEvent(action=1, step=0)
When: hydrate_run_frame_from_events(events, run_id) is called
Then: frame.parallel_in_flight() == 1
  and frame.max_parallel_in_flight() == 2
```
Test name: `hydrate_run_frame_reconstructs_parallel_in_flight`

### Behavior: Dimension integrity
```
Given: Any valid hydration
Then: frame.states.len() == frame.step_count()
  and frame.slots.len() == frame.slot_count()
  and frame.taint.len() == frame.slot_count()
```
Test name: `hydrate_run_frame_maintains_dimension_integrity`

### Behavior: Slot-taint parity
```
Given: Any valid hydration
Then: For every initialized slot, a corresponding taint marker exists
```
Test name: `hydrate_run_frame_maintains_slot_taint_parity`

### Behavior: Deterministic hydration
```
Given: Same snapshot and same tail events
When: hydrate_run_frame is called twice
Then: Both results are identical (or both are the same error)
```
Test name: `hydrate_run_frame_is_deterministic`

### Behavior: No silent defaults
```
Given: Missing slot data in snapshot (None where Some expected)
When: hydrate_run_frame is called
Then: Returns Err, does not default to Clean/Null
```
Test name: `hydrate_run_frame_no_silent_defaults_on_missing_data`

## 4. Proptest Invariants

### Proptest: hydrate_run_frame roundtrip
Invariant: For any valid snapshot + tail events, hydration either succeeds with a frame whose fields match the inputs, or returns a typed error.
Strategy: Generate random `RunSnapshot` with postcard-encoded slots/taint, generate tail `JournalEvent`s with seq > snapshot.seq.
Anti-invariant: Overlapping seqs must always fail.

### Proptest: decode_snapshot_slots
Invariant: For any decodable byte sequence, the decoded slots can be re-encoded to identical bytes.
Strategy: Generate `Vec<RecoveredSlotEntry>`, postcard-encode, then decode.
Anti-invariant: Random garbage bytes must fail decode gracefully (no panic).

### Proptest: dimension arithmetic
Invariant: `max_index + 1` never overflows for indices within `u16::MAX - 1`.
Strategy: Generate `Option<StepIdx>` and `Option<SlotIdx>` where `.get()` ≤ `u16::MAX - 1`.
Anti-invariant: `u16::MAX` must return `FrameDimensionOverflow`.

## 5. Fuzz Targets

### Fuzz Target: decode_snapshot_slots
Input type: bytes (arbitrary Vec<u8>)
Risk: Panic on malformed postcard data, OOM on length-prefix attacks, invalid SlotValue construction
Corpus seeds:
- Empty bytes
- Valid postcard-encoded slot entries
- Truncated postcard data
- Invalid length prefixes

## 6. Kani Harnesses

### Kani Harness: snapshot_run_id_match (PO-001)
Property: If snapshot.run != run_id, function returns Err
Bound: Snapshot with run_id in [0, 3], requested run_id in [0, 3]
Rationale: Critical precondition that must hold for all small run_id values

### Kani Harness: tail_seq_after_snapshot (PO-002)
Property: Any tail event with seq <= snapshot.seq causes Err
Bound: seq values in [0, 3]
Rationale: Ordering invariant must be enforced for all small seq values

### Kani Harness: step_count_positive (PO-003)
Property: If derived step_count == 0, function returns Err
Bound: step_count in [0, 3]
Rationale: Empty frame is never a valid success

### Kani Harness: dimension_overflow (PO-004)
Property: If max_index == u16::MAX, function returns FrameDimensionOverflow
Bound: max_index in [u16::MAX - 2, u16::MAX]
Rationale: Overflow must be caught at boundary

### Kani Harness: executed_counter (PO-005)
Property: executed == count of applied tail events
Bound: tail events count in [0, 3]
Rationale: Counter accuracy is critical for recovery fidelity

### Kani Harness: dimension_integrity (PO-006)
Property: states.len() == step_count AND slots.len() == slot_count AND taint.len() == slot_count
Bound: step_count in [1, 3], slot_count in [0, 3]
Rationale: Array bounds must always match declared dimensions

### Kani Harness: slot_taint_parity (PO-007)
Property: For every initialized slot, taint is readable
Bound: slot_count in [1, 3], initialize pattern in [0, 7]
Rationale: Taint/value desync is a security bug

### Kani Harness: deterministic (PO-008)
Property: Same inputs → same output
Bound: Small fixed snapshot + 2 tail events
Rationale: Recovery must be deterministic for consensus

### Kani Harness: no_empty_success (PO-012)
Property: If any required field is missing, function returns Err
Bound: Small combinations of missing snapshot fields
Rationale: No empty-frame success path

## 7. Mutation Checkpoints

Critical mutations to survive:
- `if snapshot.run != run_id` → remove or invert: caught by `test_rejects_mismatched_snapshot_run_id`
- `if event.seq() <= snapshot.seq` → remove or invert: caught by `test_rejects_tail_event_before_snapshot_seq`
- `step_count = max_step_idx + 1` → `step_count = max_step_idx`: caught by dimension integrity tests
- `frame.increment_executed()` → remove call: caught by executed counter tests
- `tracker.mark_completed()` → remove call: caught by parallel in-flight tests
- `return Err(...)` → `return Ok(frame)` on error paths: caught by every error path test

Threshold: 90% mutation kill rate minimum.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| happy path: snapshot + tail | valid snapshot, valid tail | Ok(RunFrame) | unit |
| happy path: events only | empty snapshot, valid events | Ok(RunFrame) | unit |
| error: mismatched run_id | snapshot.run=1, run_id=2 | Err(ReplayDivergence) | unit |
| error: wrong run in tail | tail event for run=2 | Err(ReplayDivergence) | unit |
| error: tail before snapshot | tail seq=5, snapshot seq=10 | Err(ReplayDivergence) | unit |
| error: corrupt snapshot bytes | invalid postcard | Err(CorruptSnapshot) | unit |
| error: empty everything | empty snapshot, no events | Err(NoRecoveryData) | unit |
| error: zero step count | only RunAccepted event | Err(InvalidCompiledWorkflow) | unit |
| error: dimension overflow | max_step_idx = u16::MAX | Err(FrameDimensionOverflow) | unit |
| boundary: min step_count | 1 step | Ok(frame with 1 state) | unit |
| boundary: min slot_count | 0 slots | Ok(frame with empty arrays) | unit |
| boundary: max slot_count | slot_count = u16::MAX - 1 | Ok(frame) | unit |
| invariant: dimension integrity | any valid input | arrays match counts | unit, kani |
| invariant: slot-taint parity | any valid input | initialized slots have taint | unit, kani |
| invariant: deterministic | same inputs twice | identical output | unit, kani, proptest |

## Open Questions

None. Contract and codebase context are sufficient for test writing.

# Test Plan: vb-2yb8 Per-Primitive Durability Proof Matrix

## Summary
- Behaviors identified: 14
- Trophy allocation: 6 unit / 6 integration / 2 e2e
- Proptest invariants: 2
- Fuzz targets: 1
- Kani harnesses: 0 (matrix is static data, not arithmetic)

## 1. Behavior Inventory

1. Matrix contains a row for every YAML primitive
2. Matrix row maps primitive to correct CompiledNodeKind
3. Matrix row names at least one RecordKind journal event
4. Matrix row specifies the correct storage partition
5. Matrix row identifies the ack point (handler return vs journal append)
6. Matrix row links to existing test evidence
7. Gate test fails when a primitive row is missing
8. Gate test fails when a row omits replay proof
9. Gate test fails when a row claims ack-before-persist
10. Submit handler persists RunSubmitted before returning Ok
11. ActionCompleted handler persists SlotWritten+StepSucceeded+ActionCompleted before Ok
12. ActionFailed handler persists ActionFailed before retry or fail_run
13. AskAnswered handler persists AskAnswered+SlotWritten+StepSucceeded before Ok
14. Cancel handler persists RunCancelled before removing run state

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit | 6 | Matrix structure, row validation, const assertions |
| Integration | 6 | Handler-level persistence ordering with VolatileRuntimeJournal |
| E2E | 2 | Full submit→complete→replay cycle; full submit→cancel→replay cycle |
| Static | 1 | Clippy lint for missing test evidence links |

## 3. BDD Scenarios

### Behavior 1: Matrix contains row for every primitive
```
Given: The durability matrix is loaded
When: Iterating over all 11 YAML primitives
Then: Each primitive has a corresponding DurabilityRow
```
Test: `fn matrix_has_row_for_every_primitive()`

### Behavior 2: Row maps to correct RecordKind
```
Given: A primitive like "wait"
When: Looking up its matrix row
Then: The row names WaitScheduled and WaitResolved RecordKinds
```
Test: `fn wait_row_names_wait_scheduled_and_wait_resolved()`

### Behavior 3: Gate fails on missing row
```
Given: A primitive "future_primitive" not yet in the matrix
When: Running the durability gate test
Then: The test fails with MissingPrimitiveRow
```
Test: `fn gate_fails_when_primitive_row_is_missing()`

### Behavior 4: Gate fails on missing replay proof
```
Given: A row for "do" with empty test_evidence
When: Running the durability gate test
Then: The test fails with MissingReplayProof
```
Test: `fn gate_fails_when_row_omits_replay_evidence()`

### Behavior 5: Gate fails on ack-before-persist claim
```
Given: A row where ack_point is BeforeJournalAppend
When: Running the durability gate test
Then: The test fails with AckBeforePersist
```
Test: `fn gate_fails_when_row_claims_ack_before_persist()`

### Behavior 6: Submit persists before ack
```
Given: A shard with a VolatileRuntimeJournal
When: Submitting a run
Then: RunSubmitted and RunAdmission events are in the journal before tick returns Ok
```
Test: `fn submit_handler_persists_before_ack()`

### Behavior 7: ActionCompleted persists before ack
```
Given: A shard with a suspended run awaiting action
When: ActionCompleted command is processed
Then: SlotWritten, StepSucceeded, ActionCompleted are in the journal before tick returns Ok
```
Test: `fn action_completed_persists_before_ack()`

### Behavior 8: ActionFailed persists before ack
```
Given: A shard with a suspended run awaiting action
When: ActionFailed command is processed
Then: ActionFailed is in the journal before tick returns Ok
```
Test: `fn action_failed_persists_before_ack()`

### Behavior 9: AskAnswered persists before ack
```
Given: A shard with a suspended ask step
When: AskAnswered command is processed
Then: AskAnswered, SlotWritten, StepSucceeded are in the journal before tick returns Ok
```
Test: `fn ask_answered_persists_before_ack()`

### Behavior 10: Cancel persists before ack
```
Given: A shard with an active run
When: Cancel command is processed
Then: RunCancelled is in the journal before tick returns Ok
```
Test: `fn cancel_persists_before_ack()`

### Behavior 11: TimerFired persists before ack
```
Given: A shard with a run awaiting a timer
When: TimerFired command is processed
Then: WaitResolved is in the journal before tick returns Ok
```
Test: `fn timer_fired_persists_before_ack()`

### Behavior 12: Resume re-emits same event sequence
```
Given: A run that was submitted and driven
When: The run is resumed from its current PC
Then: The journal contains the same events as the original execution
```
Test: `fn resume_replays_identical_event_sequence()`

### Behavior 13: E2E submit to finish replay
```
Given: A compiled workflow with set→finish
When: Submit and drive to completion
Then: Journal replay from RunSubmitted through RunFinished reproduces the same run state
```
Test: `fn e2e_submit_finish_replay_produces_identical_state()`

### Behavior 14: E2E submit to cancel replay
```
Given: A compiled workflow with a suspended do step
When: Submit and then cancel
Then: Journal replay from RunSubmitted through RunCancelled reproduces cancelled state
```
Test: `fn e2e_submit_cancel_replay_produces_cancelled_state()`

## 4. Proptest Invariants

### Proptest: Matrix completeness
Invariant: For any valid primitive name, matrix lookup returns a row.
Strategy: Select from the 11 canonical primitive names.
Anti-invariant: Random strings not in the primitive set should fail lookup.

### Proptest: Event ordering
Invariant: For any ShardCommand that mutates state, the journal event count after tick is >= count before.
Strategy: Generate valid command sequences.
Anti-invariant: Commands that fail with QueueFull or RunNotFound emit no new events.

## 5. Fuzz Targets

### Fuzz Target: RecordKind deserialization
Input type: bytes (arbitrary u16 values)
Risk: Invalid RecordKind IDs could cause storage corruption or panic
Corpus seeds: Valid RecordKind IDs 1-50, edge values 0, 65535

## 6. Kani Harnesses

None required. The matrix is static data; no arithmetic or index bounds to prove.

## 7. Mutation Checkpoints

Critical mutations to survive:
- Removing a row from DURABILITY_MATRIX → caught by `matrix_has_row_for_every_primitive`
- Changing an ack_point from AfterJournalAppend to BeforeJournalAppend → caught by gate test
- Removing a test_evidence link → caught by `gate_fails_when_row_omits_replay_evidence`
- Removing journal append in handle_cancel → caught by `cancel_persists_before_ack`

Threshold: 90% mutation kill rate.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Matrix complete | All 11 primitives | Ok(()) | unit |
| Missing primitive | Unknown primitive | Err(MissingPrimitiveRow) | unit |
| Missing replay proof | Row with empty evidence | Err(MissingReplayProof) | unit |
| Ack before persist | Row with wrong ack point | Err(AckBeforePersist) | unit |
| Submit persists | Valid Submit command | Events in journal | integration |
| ActionCompleted persists | Valid ActionCompleted | Events in journal | integration |
| ActionFailed persists | Valid ActionFailed | Events in journal | integration |
| AskAnswered persists | Valid AskAnswered | Events in journal | integration |
| Cancel persists | Valid Cancel | Events in journal | integration |
| TimerFired persists | Valid TimerFired | Events in journal | integration |
| E2E finish replay | Full workflow | Identical state | e2e |
| E2E cancel replay | Full workflow | Identical state | e2e |

## Open Questions
- Should the matrix include `CompiledNodeKind::ErrorHandler` and `CompiledNodeKind::Retry` as pseudo-primitives? → Yes, include as meta-rows.
- Should we verify `handle_resume` explicitly or is it covered by `drive_run` replay? → Cover via integration test that resume re-emits same sequence.

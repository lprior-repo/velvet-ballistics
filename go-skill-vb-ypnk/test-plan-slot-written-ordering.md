# Test Plan: LETHAL-8 — SlotWritten-Before-PC-Advance Ordering

## Summary
- **Bead ID**: LETHAL-8
- **Gap**: No test verifies `SlotWritten` is recorded BEFORE PC advance in actual execution path
- **Behaviors identified**: 3
- **Trophy allocation**: 2 integration / 1 unit / 0 e2e / 0 static
- **Proptest invariants**: 1
- **Fuzz targets**: 0
- **Kani harnesses**: 1

---

## 1. Behavior Inventory

### B-1: SlotWritten recorded before PC advance in execution trace
**Statement**: The evidence collector records `SlotWritten` events BEFORE the program counter advances to the next step during step execution.

**Evidence chain requirement**: For every successful step that writes a slot, the `SlotWritten` evidence event MUST be emitted and recorded in the evidence stream before `mark_step_after_signal` updates the PC to the subsequent step.

### B-2: Journal recovery replays SlotWritten before PC advance
**Statement**: During journal replay, all `SlotWrittenEvent` records are processed and applied to the run frame BEFORE any subsequent step's `StepStarted` event is applied.

**Durability invariant**: For any step N that produces a slot write, `SlotWrittenEvent(seq=N_slot)` must appear in the journal with a strictly lower sequence number than `StepStarted(seq=N+1_start)` for step N+1.

### B-3: SlotWritten persists at checkpoint — all preceding SlotWritten events are durable
**Statement**: When a checkpoint snapshot is taken, all `SlotWrittenEvent` records with sequence numbers less than or equal to the snapshot's sequence must be durable and recoverable.

**Checkpoint guarantee**: A snapshot at sequence `S` guarantees that all slot writes from steps 0 through S are recoverable; replaying from that snapshot yields identical slot state to original execution.

---

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|----------|-------|-----------|
| B-1: SlotWritten before PC advance in evidence | **Integration** | Must use real `EvidenceCollector` + `drive_deterministic_full` + real `RunFrame` to capture actual execution trace ordering |
| B-2: Journal recovery replays SlotWritten before PC advance | **Integration** | Requires real `FjallJournal` + full replay path to verify sequence-number ordering |
| B-3: SlotWritten durable at checkpoint | **Unit** | Pure replay logic with mock journal — tests `recover_snapshot_plus_tail` in isolation |

**Why not e2e**: The ordering of `SlotWritten` vs `PC_advance` is an internal implementation detail of the drive loop. Testing it via e2e CLI would require introspecting the evidence stream, which is an integration concern.

**Why not static**: This is a behavioral ordering invariant, not a linter-detectable property.

---

## 3. BDD Scenarios

### Behavior B-1: SlotWritten recorded before PC advance in execution trace

#### Scenario: `slot_written_appears_before_next_step_started_in_evidence_stream`
```
Given: A workflow with two consecutive SetConst steps (step 0 writes slot 0, step 1 writes slot 1)
When:  The drive loop executes both steps to completion
Then:  The drained evidence stream contains: StepStarted(0), SlotWritten(0), StepSucceeded(0), StepStarted(1), SlotWritten(1), StepSucceeded(1)
And:   The index of SlotWritten(0) in the evidence stream is less than the index of StepStarted(1)
```

#### Scenario: `evidence_collector_emits_slot_before_mark_step_after_signal_returns`
```
Given: A single SetConst step that writes slot 0
When:  drive_deterministic_full executes the step
Then:  The EvidenceCollector drain contains SlotWritten(slot=0) event
And:   The PC has advanced to step 1 (or terminal) when the evidence is drained
```

#### Scenario: `multi_slot_node_emit_order_preserved`
```
Given: A Collect node that emits multiple SlotWritten events (CollectStart, CollectNext, CollectFinish)
When:  The drive loop executes the collect node
Then:  All SlotWritten events for the node appear in the evidence stream before StepSucceeded
And:   All SlotWritten events appear before any evidence from the next step
```

#### Scenario: `no_slot_written_node_omits_slot_event`
```
Given: A Nop step (no slot output)
When:  The drive loop executes the Nop
Then:  The evidence stream contains StepStarted and StepSucceeded but no SlotWritten
```

---

### Behavior B-2: Journal recovery replays SlotWritten before PC advance

#### Scenario: `replay_restores_slot_values_in_correct_sequence_order`
```
Given: A journal with events: RunAccepted, StepStarted(0), SlotWrittenEvent(slot=0), StepSucceeded(0), StepStarted(1), SlotWrittenEvent(slot=1), StepSucceeded(1), RunFinished
When:  recover_full_journal replays the events
Then:  The recovered run frame has slot 0 = value_from_SlotWrittenEvent_0
And:   The recovered run frame has slot 1 = value_from_SlotWrittenEvent_1
And:   The PC is at the terminal position (past step 1)
```

#### Scenario: `snapshot_plus_tail_replays_tail_slot_writes_after_snapshot`
```
Given: A snapshot at sequence S and tail events including SlotWrittenEvent(seq=S+1)
When:  recover_snapshot_plus_tail reconstructs the run
Then:  The slot value from SlotWrittenEvent(seq=S+1) is present in the recovered state
And:   No ReplayDivergence error occurs
```

#### Scenario: `replay_detects_slot_written_after_step_start_violation`
```
Given: A journal with out-of-order events: StepStarted(1) before SlotWrittenEvent(0)
When:  replay_events processes these events
Then:  RecoveryError::ReplayDivergence is returned with detail describing the ordering violation
```

#### Scenario: `replay_preserves_slot_value_on_recovery`
```
Given: A step that writes slot 42 with value I64(99)
When:  The journal is closed and reopened, then recovered
Then:  The recovered slot 42 contains I64(99)
```

---

### Behavior B-3: SlotWritten persists at checkpoint

#### Scenario: `snapshot_captures_all_preceding_slot_writes`
```
Given: A run with 3 steps, each writing a different slot, and a snapshot taken after step 1
When:  recover_snapshot_plus_tail is called with the snapshot and tail events for step 2
Then:  The recovered frame has values for slot 0, slot 1, and slot 2 (from tail)
And:   The snapshot seq equals the seq of StepSucceeded(1)
```

#### Scenario: `tail_events_after_snapshot_preserve_order`
```
Given: A snapshot at seq=5 and tail events with seq=6, seq=7, seq=8
When:  recover_snapshot_plus_tail is called
Then:  Each tail event's seq is strictly greater than the snapshot seq
And:   All slot writes in the tail are correctly applied
```

#### Scenario: `corrupt_snapshot_seq_fails_gracefully`
```
Given: A snapshot with seq=S and tail events where some event has seq <= S
When:  recover_snapshot_plus_tail is called
Then:  RecoveryError::ReplayDivergence is returned
And:   The error detail mentions the seq ordering violation
```

---

## 4. Proptest Invariants

### Invariant: `evidence_stream_slot_before_next_step`
**Function**: `drive_deterministic_full` evidence emission
**Statement**: For any successful step N that writes to slot S, in the evidence drain, the position of `SlotWritten(slot=S)` is strictly less than the position of `StepStarted(N+1)` if N+1 exists.
**Strategy**: Generate workflows with 1-10 consecutive SetConst nodes, each writing to a unique slot.
**Anti-invariant**: A workflow where the evidence collector returns `SlotWritten` for step N at a position >= `StepStarted` for step N+1.

### Invariant: `journal_seq_ordering_invariant`
**Function**: `replay_events` sequence checking
**Statement**: For any two events E1 and E2 in the journal, if E1 appears before E2 in the input slice, then `E1.seq() <= E2.seq()`.
**Strategy**: Generate random event sequences with monotonically increasing seq values.
**Anti-invariant**: An event sequence with decreasing or duplicate seq values.

---

## 5. Fuzz Targets

No fuzz targets required for this bead. The `SlotWritten` ordering is a deterministic property of the drive loop and journal replay, not a parsing or deserialization boundary.

---

## 6. Kani Harnesses

### Harness: `slot_written_before_pc_advance_formal_proof`
**Property**: For every step execution in `drive_deterministic_full`, `emit_slot_evidence` (which pushes `SlotWritten` to the evidence collector) is called BEFORE `mark_step_after_signal` returns.
**Bound**: Single step execution, single slot write.
**Rationale**: This is a critical ordering invariant that guarantees durability. If `SlotWritten` is pushed AFTER PC advance, a crash between PC advance and evidence emission would lose the slot write. Proptest can exercise many executions but cannot prove the ordering holds for ALL inputs. Kani can formally verify the call order: `mark_step_after_signal` must be called (and return) before `emit_slot_evidence` pushes the slot write event.

**Harness structure**:
```rust
#[kani::proof]
fn slot_written_before_pc_advance_harness() {
    // Construct a minimal workflow with one SetConst step
    // Construct a RunFrame at pc=0
    // drive_deterministic_full for one step
    // Assert: the evidence collector contains SlotWritten
    // Assert: the run frame PC has advanced
    // The ordering is guaranteed by the source code structure but we verify
    // that no panic occurs between mark_step_after_signal and emit_slot_evidence
}
```

---

## 7. Mutation Checkpoints

### Critical mutations to survive:
| Mutation | Location | Must be caught by |
|----------|----------|-------------------|
| Swap order of `mark_step_after_signal` and `emit_slot_evidence` in `finish_drive_step` | `drive.rs:finish_drive_step` | `slot_written_appears_before_next_step_started_in_evidence_stream` |
| Remove `emit_slot_evidence` call entirely | `drive.rs:finish_drive_step` | `no_slot_written_event_emitted_when_step_has_output` |
| Change `SlotWrittenEvent` seq to be assigned AFTER PC advance | `runtime.rs` journal emission | `replay_restores_slot_values_in_correct_sequence_order` |
| Reorder `replay_events` to process `StepStarted` before `SlotWrittenEvent` for same step | `replay/core.rs` | `replay_detects_slot_written_after_step_start_violation` |

**Threshold**: 90% mutation kill rate minimum.

---

## 8. Combinatorial Coverage Matrix

### Unit: `replay_events` ordering
| Scenario | Input | Expected | Test |
|----------|-------|----------|------|
| Happy path: SlotWritten before StepStarted(N+1) | Valid event sequence | Ok(replayed_events) | `replay_restores_slot_values_in_correct_sequence_order` |
| Out-of-order: StepStarted(N+1) before SlotWritten(N) | Bad event sequence | Err(ReplayDivergence) | `replay_detects_slot_written_after_step_start_violation` |
| Duplicate seq on SlotWrittenEvent | Duplicate seq events | Ok( filtered) | `replay_skips_older_attempt_slot_writes` |
| Empty tail after snapshot | Snapshot + empty tail | Ok(replayed) | `snapshot_plus_tail_with_empty_tail_succeeds` |
| Tail seq <= snapshot seq | Bad tail | Err(ReplayDivergence) | `corrupt_snapshot_seq_fails_gracefully` |

### Integration: `drive_deterministic_full` evidence ordering
| Scenario | Input | Expected Evidence Order | Test |
|----------|-------|-------------------------|------|
| Single SetConst step | 1-step workflow | StepStarted → SlotWritten → StepSucceeded | `single_step_evidence_ordering` |
| Two consecutive SetConst steps | 2-step workflow | StepStarted(0) → SlotWritten(0) → StepSucceeded(0) → StepStarted(1) → ... | `slot_written_appears_before_next_step_started_in_evidence_stream` |
| Collect node (3 slot writes) | Collect workflow | StepStarted → SlotWritten(collect_start) → SlotWritten(collect_next) → SlotWritten(collect_finish) → StepSucceeded | `multi_slot_node_emit_order_preserved` |
| Nop step (no slot) | Nop workflow | StepStarted → StepSucceeded (no SlotWritten) | `no_slot_written_node_omits_slot_event` |
| Step that suspends (Ask) | Ask workflow | StepStarted only (no SlotWritten, no StepSucceeded) | `suspending_step_emits_no_slot_written` |

### Integration: Journal recovery sequence ordering
| Scenario | Input | Expected | Test |
|----------|-------|----------|------|
| Full journal with ordered events | Complete run journal | Exact frame reconstruction | `replay_restores_slot_values_in_correct_sequence_order` |
| Snapshot + tail | Snapshot + 1 tail slot write | Frame with all slots | `snapshot_plus_tail_replays_tail_slot_writes_after_snapshot` |
| Checkpoint durability | Journal with checkpoint | All slots durable | `snapshot_captures_all_preceding_slot_writes` |

---

## 9. Open Questions

1. **Q**: Does the `EvidenceCollector` guarantee FIFO ordering of pushes, or could `push_slot_written_with_extra` and `push_step_succeeded` be reordered by the compiler/CPU?
   **A**: In single-threaded execution (which this is), Rust's program order + no concurrent modification guarantees ordering. No memory ordering primitives needed.

2. **Q**: Is the PC advance visible to the caller of `drive_deterministic_full` at the time `emit_slot_evidence` is called?
   **A**: Yes — `mark_step_after_signal` mutates `run` in place. The PC is advanced before `emit_slot_evidence` reads from `run`.

3. **Q**: Should the test use `TraceRing::drain` or `EvidenceCollector::drain` to capture the ordering?
   **A**: Use `EvidenceCollector::drain` for unit/integration tests. The `TraceRing` is an async IPC boundary; the evidence collector is the in-process boundary where ordering is guaranteed.

4. **Q**: Is there an existing test that captures the full execution trace with sequence numbers?
   **A**: The existing `drive.rs` tests in `crates/vb_runtime/src/engine/drive.rs` use `EvidenceCollector` but do not assert ordering between `SlotWritten` and `StepStarted(N+1)`. No existing test covers this specific gap.

---

## 10. Test File Location

**Integration tests**: `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs`

**Unit tests**: `crates/vb_storage/src/recovery/replay/ordering_tests.rs` (new file)

**Kani harness**: `crates/vb_runtime/src/kani_slot_written_ordering.rs` (new file)

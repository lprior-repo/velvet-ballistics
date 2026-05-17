# Codebase Map — vb-2yb8: Per-primitive durability proof matrix

## 1. Primitive List (from velvet-ballistics-MASTER.md §10)

Canonical YAML step primitives:
- `set` (alias: `save`)
- `do` (alias: `run`)
- `choose`
- `for_each` (alias: `foreach`)
- `together`
- `collect`
- `reduce`
- `repeat`
- `wait`
- `ask`
- `finish`

Runtime executes **CompiledNodeKind** IR, not YAML primitives directly. The mapping from primitive → IR node kind → journal event is what the durability matrix must trace.

## 2. Journal Event Types

### 2.1 RuntimeJournalEvent (crates/vb_runtime/src/journal.rs)
```
RunSubmitted { run, workflow }
RunAdmission { admission }
RunFinished { run, result }
RunFailed { run }
RunCancelled { run }
ActionScheduled { run, step, action }
ActionCompleted { run, step, action }
ActionFailed { run, step, action }
WaitScheduled { run, step }
WaitResolved { run, step }
AskScheduled { run, step }
AskAnswered { run, step, slot }
SlotWritten { run, slot, value, extra }
StepStarted { run, step }
StepSucceeded { run, step, output }
```

### 2.2 JournalEvent (crates/vb_storage/src/events.rs)
Storage-facing enum with `EventSeq` per run. Maps 1:1 to `RecordKind`.

### 2.3 RecordKind (crates/vb_storage/src/records.rs)
```
RunAccepted=10, StepStarted=11, SlotWritten=12,
ActionScheduled=13, ActionCompleted=14, ActionFailed=15,
WaitScheduled=16, AskScheduled=17, AskAnswered=18,
RetryScheduled=19, StepFailed=20, RunCancelled=21,
RunFinished=22, RunFailed=23, RunAdmission=24,
Snapshot=30, Blob=40, IndexUpdate=50
```

## 3. Shard Command Paths

ShardCommand (crates/vb_runtime/src/shard/types.rs):
```
Submit { run, workflow, caps }
SubmitWithInputs { run, workflow, inputs, caps }
Resume { run }
ActionCompleted { ticket, output }
ActionCompletedLegacy { run, step }
ActionFailed { ticket, failure }
AskAnswered { answer }
TimerFired { run }
Cancel { run }
Inspect { run, correlation }
Shutdown
```

## 4. Lifecycle Handlers (crates/vb_runtime/src/shard/lifecycle.rs)

| Handler | Command | Events Emitted | Ack Point |
|---------|---------|----------------|-----------|
| handle_submit | Submit, SubmitWithInputs | RunSubmitted, RunAdmission, StepStarted, StepSucceeded, SlotWritten, ActionScheduled, WaitScheduled, AskScheduled, RunFinished | After journal append + run insert |
| handle_resume | Resume | Same family as submit | After drive_run |
| handle_action_completion | ActionCompleted | SlotWritten, StepSucceeded, ActionCompleted | After journal append |
| handle_legacy_action_completion | ActionCompletedLegacy | StepSucceeded | After journal append |
| handle_action_failure | ActionFailed | ActionFailed, then retry or fail_run | After journal append |
| handle_ask_answer | AskAnswered | AskAnswered, SlotWritten, StepSucceeded | After journal append |
| handle_timer | TimerFired | WaitResolved, StepStarted, StepSucceeded, etc. | After flush_evidence |
| handle_cancel | Cancel | RunCancelled | After journal append |
| handle_inspect | Inspect | None (read-only) | N/A |

## 5. Evidence Flow

The engine drives deterministic steps and emits EvidenceEvent:
- StepStarted { step }
- StepSucceeded { step, output }
- SlotWritten { slot, value, extra }

Shard::flush_evidence drains the collector and appends to both trace_ring and journal.

## 6. Key Gaps for Durability Matrix

1. **No explicit matrix data structure exists** — primitives are scattered across CompiledNodeKind, shard handlers, and journal events.
2. **No unified proof linking** — each primitive's persistence-before-ack ordering is implicit in handler code, not declarative.
3. **Missing RecordKind entries** — `RunSubmitted` has no direct RecordKind (only `RunAccepted` in storage). `StepFailed` exists in RecordKind but not in RuntimeJournalEvent.
4. **No release durability gate** — the matrix must be wired into CI.
5. **Ack-before-persist is not mechanically verified** — it requires reading each handler.

## 7. Existing Test Families

From crates/vb_runtime/src/shard/tests.rs:
- ShardConfig validation (capacity boundaries)
- Queue operations (enqueue, capacity, full)
- Tick processing (empty, shutdown)
- drain_for_shutdown
- Frame pool metrics
- Run lifecycle: submit, action completion, action failure, cancel, timer, resume, ask
- Error handler routing
- Capacity overflow
- Inspect (found, not found)
- Counters

## 8. Files to Modify

- `crates/vb_runtime/src/durability_matrix.rs` — new module for the matrix
- `crates/vb_runtime/src/lib.rs` — expose module
- `crates/vb_runtime/src/shard/lifecycle.rs` — wire verification hooks
- `crates/vb_runtime/tests/durability_matrix.rs` — integration tests
- `.moon/tasks.yml` or release gate — wire matrix check

## 9. Research Questions Status

| Question | Status | Answer |
|----------|--------|--------|
| Which primitives are mandatory for release? | Partial | All 11 YAML primitives; matrix must cover all |
| Where should matrix live for CI enforcement? | Open | `vb_runtime` crate with gate test |


# TLA+ Temporal Model Plan

## Boundary
- **Temporal/workflow behavior**: Runtime lifecycle state machine, resume state transition, journal event ordering, fail-closed error behavior, and CLI-to-runtime command routing.
- **Rust/core behavior excluded from TLA+ and handled by Verus/tests**: Pure state transition predicates, journal data structure invariants, typestate field presence, and append ordering within a single journal.
- **External systems abstracted**: Storage backend (FJALL/LSM-tree), CLI argument parsing, and structured output formatting.
- **Non-applicability rationale**: None. Temporal/state-over-time behavior is present.

---

## TLA+-Owned Clauses
- INV-001 -> ResumeStateMachine.tla::ValidTransition
- POST-001 -> ResumeStateMachine.tla::JournalAppendBeforeSuccess
- INV-002 -> ResumeStateMachine.tla::JournalImmutable
- POST-003 -> ResumeStateMachine.tla::FailClosedOnInvalidResume
- INV-004 -> ResumeStateMachine.tla::FailedNotResumable

---

## Model Shape

### Module/Model Path
`specs/ResumeStateMachine.tla` or `specs/vb_runtime/ResumeStateMachine.tla`

### Variables
```
RuntimeState: [run_id |-> RuntimeStateValue]
Journal: SEQ OF RuntimeJournalEvent
PendingResume: set of RunId
ResumedSet: set of RunId
```

### RuntimeStateValue Enum
```
Initial, Running, Resumable, Resuming, Failed
```

### RuntimeJournalEvent Variants
```
Initialized {run_id, timestamp}
Running {run_id, timestamp}
Resumed {run_id, timestamp}
Failed {run_id, timestamp, reason}
```

### Init Action
```
RuntimeState = [run_id \in RunIds |-> Initial]
Journal = <<>>
PendingResume = {}
ResumedSet = {}
```

### Actions
```
StartRun(run_id):
  /\ RuntimeState[run_id] = Initial
  /\ RuntimeState' = [RuntimeState EXCEPT ![run_id] = Running]
  /\ Journal' = Append(Journal, Running{run_id, Now()})
  /\ UNCHANGED <<PendingResume, ResumedSet>>

Suspend(run_id):
  /\ RuntimeState[run_id] = Running
  /\ RuntimeState' = [RuntimeState EXCEPT ![run_id] = Resumable]
  /\ UNCHANGED <<Journal, PendingResume, ResumedSet>>

Resume(run_id):
  /\ RuntimeState[run_id] = Resumable
  /\ RuntimeState' = [RuntimeState EXCEPT ![run_id] = Resuming]
  /\ PendingResume' = PendingResume \cup {run_id}
  /\ UNCHANGED <<Journal, ResumedSet>>

CompleteResume(run_id):
  /\ run_id \in PendingResume
  /\ RuntimeState[run_id] = Resuming
  /\ RuntimeState' = [RuntimeState EXCEPT ![run_id] = Running]
  /\ Journal' = Append(Journal, Resumed{run_id, Now()})
  /\ PendingResume' = PendingResume \setminus {run_id}
  /\ ResumedSet' = ResumedSet \cup {run_id}

FailResume(run_id, reason):
  /\ run_id \in PendingResume
  /\ RuntimeState[run_id] = Resuming
  /\ RuntimeState' = [RuntimeState EXCEPT ![run_id] = Failed]
  /\ Journal' = Append(Journal, Failed{run_id, Now(), reason})
  /\ PendingResume' = PendingResume \setminus {run_id}
  /\ UNCHANGED <<ResumedSet>>

FailRun(run_id, reason):
  /\ RuntimeState[run_id] = Running
  /\ RuntimeState' = [RuntimeState EXCEPT ![run_id] = Failed]
  /\ Journal' = Append(Journal, Failed{run_id, Now(), reason})
  /\ UNCHANGED <<PendingResume, ResumedSet>>
```

### State Constraints
```
Len(Journal) <= MaxJournalLength
Cardinality(RunIds) <= MaxRunIds
```

### Symmetry Sets
```
{Initial, Running, Resumable, Resuming, Failed}
```

### Bounded Model Limits for TLC
```
MaxJournalLength = 100
MaxRunIds = 10
```

---

## Properties

### Safety Invariants
- **NoDoubleRunning**: \A run_id: RuntimeState[run_id] = Running => run_id \notin PendingResume
- **ValidTransition**: \A run_id: ValidStateTransition(RuntimeState[run_id])
- **JournalImmutable**: \A i \in 1..Len(Journal)-1: Journal[i] is never modified after append
- **FailedNotResumable**: \A run_id: RuntimeState[run_id] = Failed => Resume(run_id) is disabled
- **ResumeIdempotent**: \A run_id: run_id \in ResumedSet => CompleteResume(run_id) is disabled

### Liveness/Eventuality
- **EventuallyResumed**: \A run_id \in ResumedSet: eventually (RuntimeState[run_id] = Running)
- **EventuallyTerminalOrFailed**: \A run_id: eventually (RuntimeState[run_id] \in {Running, Failed})
- **NoStarvation**: \A run_id \in PendingResume: eventually (run_id \notin PendingResume)

### Fairness Assumptions
- Weak fairness on CompleteResume and FailResume when run_id \in PendingResume
- Weak fairness on StartRun, Suspend, FailRun

### Deadlock Freedom
- No cyclic dependency on PendingResume that prevents completion
- All enabled actions eventually complete or the system reaches a terminal state (Running or Failed)

### Refinement to Rust/runtime Behavior
- TLA+ `RuntimeState` refines Rust `vb_runtime::shard::types::RuntimeState` enum variants
- TLA+ `Journal` refines Rust `RuntimeJournal` append log
- TLA+ `CompleteResume` refines Rust `Shard::handle_resume` success path
- TLA+ `FailResume` refines Rust `Shard::handle_resume` error path
- TLA+ `PendingResume` tracks in-flight resume requests to enforce journal-append-before-success

---

## Journal Append-Before-Success Temporal Ordering (POST-001)

### TLA+ Temporal Property
```
JournalAppendBeforeSuccess == \A run_id \in ResumedSet:
  (EXISTS i: Journal[i].run_id = run_id /\ Journal[i].type = "Resumed")
  =>
  (RuntimeState[run_id] = Running)
```

This enforces that a `Resumed` event appears in the journal before we consider the resume operation successful in the runtime state.

---

## Evidence Commands

### TLC
```bash
tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla
```

Expected: TLC reports no invariant violations (ValidTransition, NoDoubleRunning, FailedNotResumable, JournalImmutable), no deadlock, and temporal properties (EventuallyTerminalOrFailed, NoStarvation) satisfied for ResumeStateMachine.cfg bounds.

### Apalache
```bash
apalache-mc check --config=specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla
```

Expected: Apalache reports all INV_ and TEMPORAL_ properties satisfied within bounded model.

---

## Waivers
- None. All temporal/state-over-time behavior for the resume lifecycle is covered by TLA+.

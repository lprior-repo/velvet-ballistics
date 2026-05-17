# TLA+ Temporal Model Plan: CLI Lifecycle Event-Applied Tracker

## Boundary

### Temporal/Workflow Behavior (TLA+-Owned)
- **Lifecycle State Machine**: `Pending → Active → WaitingAnswer ↔ Cancelled` with retry/answer transitions
- **State Consistency**: Journal-derived state is always consistent with external observers
- **Event Immutability**: Events are append-only; never modified or deleted
- **Valid Transitions**: Only transitions validated by `check_lifecycle_transition` are allowed

### Rust/Core Behavior Excluded from TLA+ (Verus/Kani/tests)
- Pure function `derive_lifecycle_state_from_events` correctness
- `check_lifecycle_transition` implementation correctness
- Fjall journal storage implementation
- Memory safety and undefined behavior

### External Systems Abstracted
- No external systems involved; purely local state machine

### Non-applicability Rationale
- N/A — This refactoring IS a temporal/state-over-time problem (journal append + state derivation)

## TLA+-Owned Clauses

| Clause ID | Description | TLA+ Module/Invariant |
|-----------|-------------|----------------------|
| INV-001 | State-Journal Consistency | `LifecycleModule::ConsistentState` |
| INV-002 | No Divergence | `LifecycleModule::NoDivergence` |
| INV-003 | Valid Transitions Only | `LifecycleModule::ValidTransition` |
| INV-004 | Event Immutability | `LifecycleModule::EventsImmutable` |
| INV-005 | Terminal States Final | `LifecycleModule::TerminalFinal` |

## Model Shape

### Module/Model Path
- `specs/Lifecycle.tla` (target path in `velvet-ballistics` checkout)

### Variables

```tla
VARIABLES
  (* Per-run state, derived from journal events *)
  runState,        \* [RunId -> LifecycleState]
  (* Event log: append-only journal *)
  eventLog,        \* [RunId -> Seq(JournalEvent)]
  (* Transition guard *)
  transitionValid  \* [RunId -> BOOLEAN]
```

### State Shape

```tla
\* LifecycleState domain
CONSTANTS
  Pending, Active, WaitingAnswer, Completed, Failed, Cancelled

\* JournalEvent domain
CONSTANTS
  RunCancelled, RunResumed, RunRetried, RunAnswered, RunAccepted,
  RunAdmission, RunFailedEvent, WaitScheduledEvent, AskScheduledEvent,
  AskAnsweredEvent, ActionFailedEvent
```

### Init Action

```tla
Init ==
  /\ runState = [r \in Runs |-> Pending]
  /\ eventLog = [r \in Runs |-> <<>>]
  /\ transitionValid = [r \in Runs |-> TRUE]
```

### Actions/Next Relation

```tla
Cancel(run) ==
  /\ runState[run] \in {Active, WaitingAnswer, Failed}
  /\ runState' = [runState EXCEPT ![run] = Cancelled]
  /\ eventLog' = [eventLog EXCEPT ![run] = Append(eventLog[run], RunCancelled)]
  /\ UNCHANGED transitionValid

Resume(run) ==
  /\ runState[run] = Cancelled
  /\ runState' = [runState EXCEPT ![run] = Active]
  /\ eventLog' = [eventLog EXCEPT ![run] = Append(eventLog[run], RunResumed)]

Retry(run) ==
  /\ runState[run] = Failed
  /\ runState' = [runState EXCEPT ![run] = Active]
  /\ eventLog' = [eventLog EXCEPT ![run] = Append(eventLog[run], RunRetried)]

Answer(run, answer) ==
  /\ runState[run] = WaitingAnswer
  /\ runState' = [runState EXCEPT ![run] = Completed]
  /\ eventLog' = [eventLog EXCEPT ![run] = Append(eventLog[run], RunAnswered)]

AskScheduled(run) ==
  /\ runState[run] = Active
  /\ runState' = [runState EXCEPT ![run] = WaitingAnswer]
  /\ eventLog' = [eventLog EXCEPT ![run] = Append(eventLog[run], AskScheduledEvent)]
```

### State Derivation Invariant

```tla
\* The state derived from the last event MUST match the tracked state
DeriveConsistent(run) ==
  LET lastEvent == eventLog[run][Len(eventLog[run])]
  IN
  CASE lastEvent = RunCancelled     -> runState[run] = Cancelled
    [] lastEvent \in {RunResumed, RunRetried, RunAccepted, RunAdmission} -> runState[run] = Active
    [] lastEvent \in {RunAnswered, RunFinished} -> runState[run] = Completed
    [] lastEvent = RunFailedEvent   -> runState[run] = Failed
    [] lastEvent \in {WaitScheduledEvent, AskScheduledEvent, AskAnsweredEvent} -> runState[run] = WaitingAnswer
    [] lastEvent = ActionFailedEvent -> runState[run] = Failed
    [] OTHER                        -> TRUE

ConsistentState ==
  \A run \in Runs : DeriveConsistent(run)
```

### Safety Invariants

```tla
NoDivergence ==
  \A run \in Runs :
    runState[run] = DeriveState(eventLog[run])

ValidTransition(run, newState) ==
  CASE runState[run] = Pending /\ newState = Active -> TRUE
    [] runState[run] = Active /\ newState \in {WaitingAnswer, Failed, Cancelled} -> TRUE
    [] runState[run] = WaitingAnswer /\ newState \in {Completed, Cancelled} -> TRUE
    [] runState[run] = Failed /\ newState = Active -> TRUE
    [] OTHER -> FALSE
```

### Temporal Properties

| Property | TLA+ Expression |
|----------|-----------------|
| Eventual Terminal | `\A run \in Runs : <> (runState[run] \in {Completed, Cancelled})` |
| No Infinite Retry | `~<>(SF(runState[run] = Failed /\ runState[run]' = Active))` — retry bounded |
| Terminal Finality | `\A run \in Runs : (runState[run] \in {Completed, Cancelled}) => [] (runState[run] = runState[run])` |

### Fairness Assumptions

- **Weak Fairness** on `Cancel`, `Resume`, `Retry`, `Answer`, `AskScheduled` when enabled
- No strong fairness required; terminal states are intentional

### Deadlock Freedom

```tla
DeadlockFree ==
  \A run \in Runs :
    \E action \in Actions :
      action.enabled
```

Note: `Completed` and `Cancelled` runs have no enabled actions — this is by design, not deadlock.

### Bounded Model Limits for TLC/Apalache

```tla
CONSTANT RunIds \* Set of RunId values, e.g., {r1, r2}
CONSTANT MaxEventsPerRun \* Upper bound on event sequence length, e.g., 10
```

Symmetry sets disabled for RunIds to avoid spurious counterexamples.

## Refinement to Rust/Runtime Behavior

| TLA+ Variable | Rust Equivalent |
|---------------|-----------------|
| `runState[run]` | `derive_lifecycle_state_from_events(journal.events_for_run(run))` |
| `eventLog[run]` | `journal.events_for_run(run)` |
| `transitionValid` | `check_lifecycle_transition(current, cmd)` |

**Refinement Relation**: The Rust implementation refines the TLA+ model when:
1. `journal.events_for_run(run)` returns events in append order
2. `derive_lifecycle_state_from_events` implements the same last-event → state mapping as `DeriveState`
3. `check_lifecycle_transition` implements the same transition validation as `ValidTransition`

## Evidence Command

```bash
# TLC model checker
tlc -config specs/Lifecycle.cfg specs/Lifecycle.tla

# Apalache symbolic checker (optional)
apalache-mc check --config specs/Lifecycle.cfg specs/Lifecycle.tla
```

Expected: TLC reports no invariant violations (`ConsistentState`, `NoDivergence`), no deadlock, and temporal properties satisfied within configured bounds.

## Waivers

| Waiver ID | Clause | Reason | Expiry |
|-----------|--------|--------|--------|
| WAIVER-001 | Eventual Terminal | Some runs may never reach terminal state (e.g., infinite retry loop with external intervention) | N/A — acknowledged design choice |

## Verification Scope Post-Refactoring

The TLA+ model verifies:
- **Before**: The state machine invariants with in-memory tracker (pre-refactoring)
- **After**: The state machine invariants WITHOUT in-memory tracker (post-refactoring)

Both should satisfy the same invariants — the TLA+ model proves the refactoring preserves correctness.

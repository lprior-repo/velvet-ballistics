# Theorem Kernel Projection: CLI Lifecycle Event-Applied Tracker

## Boundary

### TLA+-Owned Temporal Model
- Full lifecycle state machine (`Pending → Active → WaitingAnswer ↔ Cancelled`)
- Journal-state consistency invariants
- Event append-only semantics
- Transition validation temporal properties

### Verus-Owned Rust Core
- Pure function `derive_lifecycle_state_from_events` correctness
- `check_lifecycle_transition` implementation
- State machine transition validity
- No panics, overflows, or undefined behavior in lifecycle operations

### Theorem-Owned Kernel
- **None required** for this refactoring

### Rust/Runtime Shell
- `FjallJournal` storage implementation
- `LazyLock<Mutex<RunStateTracker>>` concurrency
- I/O, async scheduling, wall-clock time

### External Systems Excluded from Theorem Proof
- N/A — purely local state machine

## Theorem-Owned Clauses

**None** — This refactoring does not require Lean/Aeneas/Hax theorem proving. The critical pure logic is:

1. `derive_lifecycle_state_from_events` — a total function from event sequences to states
2. `check_lifecycle_transition` — boolean transition validation

Both are well-suited for:
- **Verus** spec functions with `by (compute)` for pure evaluation
- **Kani** harnesses for bounded model checking of transition sequences
- **TLA+** model checking for state machine properties

## Lean/Aeneas/Hax Non-Applicability Rationale

The lifecycle state machine has:
- **Small finite state space**: 6 states × bounded event sequences
- **Deterministic transitions**: Each (state, event) pair maps to exactly one next state
- **No algebraic complexity**: Not a protocol lattice, parser grammar, or arithmetic bound theorem
- **No refinement chain needed**: TLA+ refinement to Rust is sufficient

**Verdict**: Verus + TLA+ provides adequate assurance without theorem proving overhead.

## Waivers

| Waiver ID | Clause | Reason | Compensating Evidence |
|-----------|--------|--------|----------------------|
| WAIVER-LEAN-001 | Theorem projection | Problem is finite-state and deterministic; TLA+ model checking + Verus specs provide equivalent assurance | TLA+ `ConsistentState` invariant, Verus `spec_derive_lifecycle_state_from_events` with `by (compute)` |
| WAIVER-LEAN-002 | Algebraic state extraction | No complex protocol lattice, parser algebra, or arithmetic bounds requiring proof assistant | Direct TLA+ model + Verus pure function specification |

## Verus-Owned Clauses (Primary Proof Surface)

### VERUS-INV-003: Valid Transitions Only

**Contract Clause**: INV-003

**Rust Target**: `vb_core::workflow::check_lifecycle_transition`

**Verus Spec/Proof Surface**:
```verus
spec fn spec_check_lifecycle_transition(s: LifecycleState, cmd: LifecycleCommand) -> bool {
    match (s, cmd) {
        (Pending, LifecycleCommand::Resume) => true,
        (Active, LifecycleCommand::Cancel) => true,
        (Active, LifecycleCommand::Retry) => true,
        (WaitingAnswer, LifecycleCommand::Answer(_, _)) => true,
        (WaitingAnswer, LifecycleCommand::Cancel) => true,
        (Failed, LifecycleCommand::Retry) => true,
        _ => false,
    }
}
```

**Invariant**: For all valid runs, `check_lifecycle_transition` returns true if and only if the transition is valid per the state machine.

**Trusted Boundary**: `LifecycleState` and `LifecycleCommand` are non-exhaustive enums enforced by the type system.

**Shell Exclusions**: I/O, async scheduling, storage, wall-clock time.

**Evidence Command**:
```bash
verus crates/vb_core/src/workflow.rs
```

Expected: Verus verifies `spec_check_lifecycle_transition` against `check_lifecycle_transition` implementation.

### VERUS-INV-DERIVE: State Derivation Correctness

**Contract Clause**: INV-001

**Rust Target**: `vb_cli::lifecycle::derive_lifecycle_state_from_events`

**Verus Spec/Proof Surface**:
```verus
spec fn spec_derive_state_from_event(e: JournalEvent) -> LifecycleState {
    match e {
        JournalEvent::RunCancelled => LifecycleState::Cancelled,
        JournalEvent::RunResumed => LifecycleState::Active,
        JournalEvent::RunRetried => LifecycleState::Active,
        // ... etc
    }
}

spec fn spec_derive_lifecycle_state_from_events(events: Seq<JournalEvent>) -> LifecycleState {
    if events.len() == 0 {
        Pending
    } else {
        spec_derive_state_from_event(events[events.len() - 1])
    }
}
```

**Invariant**: For any non-empty event sequence, `derive_lifecycle_state_from_events` returns the state determined by the last event.

**Trusted Boundary**: Input events are well-formed `JournalEvent` variants.

**Shell Exclusions**: I/O, async scheduling, storage, wall-clock time.

**Evidence Command**:
```bash
verus crates/vb_cli/src/lifecycle.rs
```

Expected: Verus verifies `spec_derive_lifecycle_state_from_events` against implementation via `by (compute)`.

## Alternative: If Theorem Proving Were Required

If future complexity requires theorem proving, the kernel would be:

```lean
-- Theorem: State derivation from event sequence is deterministic and total
theorem derive_deterministic (events : List JournalEvent) (s1 s2 : LifecycleState)
  (h1 : derive_state events = some s1)
  (h2 : derive_state events = some s2) :
  s1 = s2 := ...

-- Theorem: Terminal states have no outgoing transitions
theorem terminal_no_outgoing (s : LifecycleState) (h : terminal s)
  (e : JournalEvent) :
  derive_state (events ++ [e]) ≠ some s := ...
```

**But this is not needed for vb-0253.7** — the problem is bounded and well-specified in TLA+/Verus.

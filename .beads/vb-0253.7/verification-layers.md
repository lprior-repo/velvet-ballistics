# Verification Layers: CLI Lifecycle Event-Applied Tracker

## Boundary

### Verus-Owned Kernel
- Pure state derivation: `derive_lifecycle_state_from_events`
- Pure transition validation: `check_lifecycle_transition`
- No I/O, async, storage, or wall-clock dependencies

### TLA+ Temporal Model
- Full lifecycle state machine
- Journal-state consistency
- Event append-only semantics
- Transition validation temporal properties

### Theorem Projection
- **None required** — problem is bounded and well-specified in TLA+/Verus

### Runtime Shell
- `FjallJournal` storage implementation details
- `LazyLock<Mutex<RunStateTracker>>` removal
- I/O, async scheduling, wall-clock time, FFI

### External Systems
- **None** — purely local state machine

## Layer Assignment

| Clause ID | Primary Layer | Secondary Layer | Waiver |
|-----------|--------------|-----------------|--------|
| INV-001 | `tla-plus` | `verus` | No |
| INV-002 | `tla-plus` | `verus` | No |
| INV-003 | `verus` | `kani` | No |
| INV-004 | `tla-plus` | `verus` | No |
| INV-005 | `tla-plus` | `verus` | No |
| PRE-001 | `verus` | `kani` | No |
| PRE-002 | `verus` | `kani` | No |
| PRE-003 | `verus` | `kani` | No |
| PRE-004 | `tla-plus` | `miri` | No |
| POST-001 | `tla-plus` | `verus` | No |
| POST-002 | `tla-plus` | `verus` | No |
| POST-003 | `tla-plus` | `verus` | No |
| POST-004 | `tla-plus` | `verus` | No |
| POST-005 | `verus` | `kani` | No |
| POST-006 | `tla-plus` | `verus` | No |

## Verus Scope

### Rust Target
- `crates/vb_cli/src/lifecycle.rs` (refactored)
- `crates/vb_core/src/workflow.rs` (existing `check_lifecycle_transition`)

### Spec/Proof Functions

```verus
// State derivation specification
spec fn spec_derive_lifecycle_state_from_events(events: Seq<JournalEvent>) -> LifecycleState

// Transition validation specification
spec fn spec_check_lifecycle_transition(s: LifecycleState, cmd: LifecycleCommand) -> bool
```

### Invariants

1. **DeriveCorrect**: `derive_lifecycle_state_from_events` matches `spec_derive_lifecycle_state_from_events` for all event sequences
2. **TransitionValid**: `check_lifecycle_transition` matches `spec_check_lifecycle_transition` for all state/command pairs
3. **NoPanic**: All lifecycle functions are panic-free for valid inputs
4. **NoOverflow**: Event sequence length arithmetic is bounded and overflow-safe

### Trusted Boundary

- `LifecycleState`, `LifecycleCommand`, `JournalEvent` are externally validated sum types
- `RunId` is a validated identifier type
- Journal access is abstracted behind safe interface

### Shell Exclusions

- **Excluded**: Fjall storage implementation, I/O, async scheduling, wall-clock time
- **Included**: Pure function implementations, state derivation logic, transition validation

### Evidence Command

```bash
verus crates/vb_cli/src/lifecycle.rs crates/vb_core/src/workflow.rs
```

Expected: Verus verifies all spec functions against implementations with 0 errors.

## TLA+ Scope

### Module/Model Path
- `specs/Lifecycle.tla` (in `velvet-ballistics` checkout)

### Variables
- `runState: [RunId -> LifecycleState]` — tracked state
- `eventLog: [RunId -> Seq(JournalEvent)]` — journal events
- `transitionValid: [RunId -> BOOLEAN]` — transition guard

### Actions

```tla
Cancel(run), Resume(run), Retry(run), Answer(run, answer), AskScheduled(run)
```

### Safety Invariants

```tla
ConsistentState == \A run \in Runs : runState[run] = DeriveState(eventLog[run])
NoDivergence == \A run \in Runs : runState[run] = DeriveState(eventLog[run])
ValidTransition == \A run, newState : ...
```

### Temporal Properties

| Property | Expression |
|----------|------------|
| Eventual Terminal | `\A run : <> (runState[run] \in {Completed, Cancelled})` |
| Terminal Finality | `(runState[run] \in {Completed, Cancelled}) => [] (runState[run] = runState[run])` |

### Fairness/Deadlock Stance

- **Weak fairness** on enabled lifecycle actions
- **No deadlock** for active runs; terminal runs have no enabled actions by design

### Refinement Boundary

| TLA+ Variable | Rust Expression |
|---------------|----------------|
| `runState[run]` | `derive_lifecycle_state_from_events(journal.events_for_run(run))` |
| `eventLog[run]` | `journal.events_for_run(run)` |

**Refinement Condition**: Rust implementation refines TLA+ model when event ordering and derivation function are identical.

### Evidence Command

```bash
tlc -config specs/Lifecycle.cfg specs/Lifecycle.tla
```

Expected: TLC reports 0 invariant violations, no deadlock, temporal properties satisfied.

## Kani Scope

### Rust Target
- `crates/vb_cli/src/lifecycle.rs` (refactored post-modification)

### Claim
- Bounded state transition sequences never panic
- `derive_lifecycle_state_from_events` is total and returns valid states
- `check_lifecycle_transition` returns correct boolean for all valid inputs

### Harness Strategy

```rust
#[kani::proof]
fn state_derivation_is_total() {
    // Kani Arbitrary for JournalEvent sequence
    let events = kani::any::<Vec<JournalEvent>>();
    let state = derive_lifecycle_state_from_events(&events);
    // state is always valid
}

#[kani::proof]
fn valid_transitions_accepted() {
    let state = kani::any::<LifecycleState>();
    let cmd = kani::any::<LifecycleCommand>();
    if check_lifecycle_transition(state, cmd) {
        // transition is valid
    }
}
```

### Evidence Command

```bash
cargo kani --crate-type=lib -p vb_cli
```

Expected: Kani reports 0 unproven targets for lifecycle module.

## Miri Scope

### Rust Target
- `crates/vb_cli/src/lifecycle.rs` (refactored)

### Concern
- Any new unsafe blocks introduced during refactoring
- Raw pointer handling in journal access

### Evidence Command

```bash
cargo miri test -p vb_cli --lib
```

Expected: Miri reports 0 undefined behavior violations.

## Loom Scope (If Concurrency Testing Needed)

### Concern
- Removal of `static TRACKER` mutex — does this introduce race conditions?
- Journal access from concurrent CLI invocations

### Waiver Justification
- **WAIVER-LOOM-001**: Journal is already thread-safe (Fjall handles concurrency)
- `cancel`, `resume`, `retry`, `answer` are naturally serialized by journal append
- No shared mutable state post-refactoring

## Performance Scope

### Concern
- Journal read on every lifecycle command may add latency

### Claim
- Journal reads are bounded by Fjall LSM-tree read path
- No unbounded loops or recursive operations

### Evidence Command
```bash
cargo bench --bench lifecycle_commands
```

Expected: p99 latency within acceptance threshold against baseline.

**Note**: Performance verification is **MEDIUM priority** — correctness is primary concern.

## API Compatibility Scope

### Concern
- Public API surface unchanged — internal behavior different

### Claim
- External callers see identical behavior (same error variants, same success)

### Evidence Command

```bash
cargo semver-checks -p vb_cli
```

Expected: No semver violations for public lifecycle API.

## Waivers

| Waiver ID | Layer | Clause | Reason | Compensating Evidence |
|-----------|-------|--------|--------|----------------------|
| WAIVER-LOOM-001 | loom | concurrency | Journal is thread-safe; no shared mutable state post-refactoring | TLA+ model with concurrent actions; Kani harness for state transitions |
| WAIVER-PERF-001 | performance | latency | Not a correctness requirement; latency acceptable within existing SLA | Manual benchmark if regression suspected |
| WAIVER-LEAN-001 | lean | theorem | Problem is finite-state and deterministic | TLA+ model checking + Verus specs |

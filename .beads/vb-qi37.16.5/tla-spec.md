# TLA+ Temporal Model Plan

## Boundary

- **Temporal/workflow behavior owned by TLA+**:
  - Lifecycle state machine: valid state transitions (pending → active → waiting_answer → completed/cancelled)
  - Append-only journal semantics: every transition writes exactly one event
  - Replay correctness: replay produces bit-identical state to pre-crash
  - Invalid transition rejection: commands in wrong state are rejected without state mutation
  - Duplicate/stale request detection and rejection
  - Crash recovery: after restart, replay reconstructs exact bead states

- **Rust/core behavior excluded from TLA+**:
  - Verus owns `lifecycle.rs` typestate invariants, `journal.rs` pure event formatting
  - Verus owns `storage.rs` command validation preconditions
  - Kani owns bounded model checking of numeric/indexing paths in storage adapters

- **External systems abstracted**:
  - Storage backend (vb_storage) abstracted as append-only journal interface
  - CLI argument parsing (velvet_ballistics/args.rs) abstracted as command dispatch

- **Non-applicability rationale**: N/A — lifecycle state transitions, journal replay, and recovery are inherently temporal/state-over-time and require TLA+ model checking.

## TLA+-Owned Clauses

| Clause ID | Description |
|-----------|-------------|
| INV-002 | Journal append-only: no event is ever removed or overwritten |
| INV-003 | Valid transition enforcement: cancel/resume/retry/answer only fire from correct prior states |
| INV-004 | Replay bit-identical: replay produces identical bead state map to pre-crash |
| POST-003 | Invalid-transition rejection: wrong-state commands return error and leave state unchanged |
| POST-004 | Duplicate-request rejection: second identical command returns error, no double-write |
| POST-005 | Stale-request rejection: retroactively-applied commands return error |

## Model Shape

- **Module/Model path**: `specs/LifecycleJournal.tla` (written by formal-verifier in State 12)
- **Variables**:
  - `bead_state : [BeadId → LifecycleState]` — current canonical state per bead
  - `journal : Seq[RuntimeJournalEvent]` — append-only event log
  - `commands : Set[LifecycleCommand]` — in-flight commands
  - `crashed : Bool` — crash flag for replay simulation
- **Init action**: `Init == /\ bead_state = [b \in Beads |-> Pending] /\ journal = <<>> /\ crashed = FALSE /\ commands = {}`
- **Next/actions**:
  - `Cancel(b)` — valid if `bead_state[b] \in {Active, WaitingAnswer}`
  - `Resume(b)` — valid if `bead_state[b] = Cancelled`
  - `Retry(b)` — valid if `bead_state[b] = Failed`
  - `Answer(b, ans)` — valid if `bead_state[b] = WaitingAnswer`
  - `Crash` — sets `crashed = TRUE`, represents power loss
  - `Replay` — consumes journal, reconstructs `bead_state`
- **State constraints**: Finite `Beads` set bounded for TLC; `Len(journal) \leq MaxJournalLen`
- **Symmetry sets**: None (bead IDs are distinguishable)
- **Bounded model limits**:
  - `MaxJournalLen = 1000` for TLC bounded check
  - `Beads = {b1, b2, b3}` for model finding

## Properties

- **Safety invariants**:
  - `NoOverwrite == \forall e \in journal : \nexists f \in journal : e \neq f /\ e.bead_id = f.bead_id /\ e.index > f.index /\ e.command = f.command`
  - `SingleCanonicalState == \forall b \in Beads : \exists! s : bead_state[b] = s`
  - `InvalidTransitionBlocked == \forall cmd \in commands : IsValidTransition(cmd) => journal' = journal \circ <event(cmd)>`

- **Liveness/eventuality**:
  - `EventuallyTerminalOrCancelled == \forall b \in Beads : (<> bead_state[b] \in {Completed, Cancelled})`
  - `JournalGrowth == \forall e : event_written(e) => <> Len(journal) > 0`

- **Fairness assumptions**:
  - Weak fairness on `Cancel`, `Resume`, `Retry`, `Answer` when command is enabled
  - Weak fairness on `Replay` after `Crash`

- **Deadlock freedom**:
  - Model check with `DieOnDeadlock = FALSE` to confirm no livelock

- **Refinement to Rust/runtime behavior**:
  - TLA+ `bead_state` refines Rust `lifecycle.rs` typestate `enum LifecycleState`
  - TLA+ `journal` refines Rust `RuntimeJournalEvent` vector written via `journal.rs::append_event`
  - TLA+ `Replay` action refines Rust `journal.rs::replay()` reconstruction
  - CLI commands cancel/resume/retry/answer map 1-to-1 to TLA+ actions

## Evidence Command

```bash
# TLC model check
tlc -config specs/LifecycleJournal.cfg specs/LifecycleJournal.tla

# Apalache symbolic check (if preferred)
apalache-mc check --config=specs/LifecycleJournal.cfg specs/LifecycleJournal.tla
```

Expected evidence: TLC reports no invariant violations (NoOverwrite, SingleCanonicalState, InvalidTransitionBlocked), no deadlock, and temporal properties (EventuallyTerminalOrCancelled, JournalGrowth) satisfied for the bounded model.

## Waivers

None — all lifecycle/journal/replay behavior is within TLA+ scope for this bead.

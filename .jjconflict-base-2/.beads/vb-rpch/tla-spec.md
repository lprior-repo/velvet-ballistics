# TLA+ Temporal Model Plan — vb-rpch

## Boundary

- **Temporal/workflow behavior owned by TLA+**: Journal event replay sequence, snapshot-plus-tail ordering, digest verification pipeline, incomplete-run discovery, terminal event detection, recovery state machine transitions, and non-idempotent action blocking semantics.
- **Rust/core behavior excluded from TLA+ and handled by Verus/Kani/tests**: Pure recovery type invariants (`UnsupportedRecoveryState::union`, `ActionReplayTracker` monotonicity), dimension bound arithmetic, snapshot decoding, slot/taint decoding, frame construction from seed.
- **External systems abstracted**: Fjall persistence layer is modeled as an event sequence provider; no internal Fjall structure is modeled.
- **Non-applicability rationale**: Not applicable — this bead has significant temporal/state-machine behavior (journal replay, recovery pipeline, event ordering invariants) that warrants TLA+ modeling.

---

## TLA+-Owned Clauses

### TLA-001 → specs/RecoveryReplay.tla::ReplaySeqOrder
Journal replay events are processed in strictly ascending `seq` order per attempt, and step indices are monotonically non-decreasing within each attempt.

### TLA-002 → specs/RecoveryReplay.tla::TailCausalAfterSnapshot
For snapshot-plus-tail hydration, all tail events have `seq > snapshot.seq`.

### TLA-003 → specs/RecoveryReplay.tla::OnlyIncompleteRuns
`recover_all_incomplete_runs` returns only runs whose latest-attempt journal has no terminal event (RunFinished, RunCancelled, RunFailedEvent).

### TLA-004 → specs/RecoveryReplay.tla::NoResolvedReExecution
Once `ActionReplayTracker` marks an `(action, step)` as completed or failed, that pair never appears as a pending scheduled action in subsequent replay output.

### TLA-005 → specs/RecoveryReplay.tla::RecoveryErrorExhaustive
Every `RecoveryError` variant is reachable from some defined input combination to the recovery functions.

### TLA-006 → specs/RecoveryReplay.tla::DigestVerificationOrder
Workflow source digest is verified before compiled IR digest in `verify_digests` at any `DigestCheck` level that includes both.

---

## Model Shape

### Module/Model Path
`specs/RecoveryReplay.tla` — single module covering journal recovery semantics

### Variables
```
Events: Seq of JournalEvent
Snapshots: [run -> Snapshot | None]
Tracker: [action_step -> {"unresolved", "completed", "failed"}]
TerminalFlags: [run -> {"none", "cancelled", "finished", "failed"}]
DigestCheckLevel: {"WorkflowSourceOnly", "WorkflowAndIr", "Full"}
```

### Init Action
```
Init ==
    /\ Events = << >>
    /\ Snapshots = [run \in Runs |-> None]
    /\ Tracker = [action_step \in ActionSteps |-> "unresolved"]
    /\ TerminalFlags = [run \in Runs |-> "none"]
    /\ DigestCheckLevel = "WorkflowSourceOnly"
```

### Actions
- `ReplayEvent(e)` — advance replay state with one event, update Tracker on action completion/failure
- `CheckDigest(level)` — verify workflow/IR digests at specified level
- `SnapshotPlusTail(snap, tail)` — apply snapshot + ordered tail events
- `ExtractTerminal(run)` — set TerminalFlags based on latest terminal event of max attempt
- `DiscoverIncomplete` — collect runs with TerminalFlags = "none"
- `BlockNonIdempotent(action, step)` — error when resolved action appears again

### State Constraints
- `Len(Events) <= MAX_EVENTS` (bounded for TLC model checking)
- `step.get() <= MAX_STEP_IDX` (u16 bound)
- `seq.get() <= MAX_EVENT_SEQ` (bounded for TLC)

### Symmetry Sets
- `Runs` — small bounded set of run IDs for model checking
- `ActionSteps` — product of action ID × step index

### Bounded Model Limits
- `MAX_EVENTS = 20`
- `MAX_STEP_IDX = 10`
- `MAX_ACTION_STEPS = 10`
- `MAX_EVENT_SEQ = 100`

---

## Properties

### Safety Invariants
- **ReplaySeqOrder**: For all events in replay output, seq is non-decreasing per attempt
- **TailCausalAfterSnapshot**: All tail event seqs > snapshot seq
- **OnlyIncompleteRuns**: DiscoverIncomplete only returns runs with no terminal event of max attempt
- **NoResolvedReExecution**: Tracker never shows "unresolved" for an action_step that was marked completed/failed
- **DigestVerificationOrder**: Workflow digest verified before IR digest

### Liveness/Eventuality
- **EventuallyTerminalOrRecoverable**: Every non-terminal run eventually reaches either a terminal state or is included in incomplete runs set
- **EventuallyAllDigestsVerified**: At `DigestCheck::Full`, all digests are eventually verified or error returned

### Fairness Assumptions
- Weak fairness on `ReplayEvent` action
- Weak fairness on `DiscoverIncomplete` action

### Deadlock Freedom
- `DeadlockFree`: Model never reaches a state with no enabled actions and incomplete work remaining
- `NoPanicOnOverflow`: FrameDimensionOverflow is caught and returned as typed error, not panic

### Refinement to Rust/runtime Behavior
- Rust `recover_runtime_summary` refines TLA+ `ExtractTerminal` by extracting counts (steps_started, steps_succeeded, actions_scheduled, actions_resolved, suspensions, slots_written) from the same event sequence
- Rust `recover_runtime_frame_seed` refines TLA+ `ReplayEvent` by additionally computing step_count, slot_count, first_step, pc, steps, slots, pending_actions, and unsupported markers
- Rust `replay_events` refines TLA+ `ReplayEvent` by skipping older-attempt events and blocking non-idempotent re-execution
- Rust `recover_all_incomplete_runs` refines TLA+ `DiscoverIncomplete` by iterating run headers and checking for absence of terminal event of max attempt
- Rust `verify_digests` refines TLA+ `CheckDigest` by calling journal lookup and returning typed `RecoveryError` on mismatch

---

## Evidence Command
```
tlc -config specs/RecoveryReplay.cfg specs/RecoveryReplay.tla
```
Expected: TLC reports no invariant violations, no deadlock, and all temporal properties satisfied for RecoveryReplay.cfg bounds.

---

## Waivers

- **GAP-3 ActionAbiMismatch / PolicyDigestMismatch**: Not reachable via public recovery API at `DigestCheck::Full` because `expected_action_abi_digests` and `expected_policy_digests` lookup functions are not yet implemented. TLA+ model records these as `\* TODO: GAP-3 verify_digests Full-level checks unimplemented` in the `CheckDigest` action. Compensating evidence: existing unit tests cover the typed error definitions; future bead vb-ty9 will add the lookup and make these reachable.
- **TerminalStateMismatch**: No expected-terminal parameter in public `recover_runtime_summary` / `recover_runtime_frame_seed` APIs. TLA+ model records this as a future property `EventuallyTerminalMatchesExpected` which requires API extension. Compensating evidence: `extract_terminal` is tested in BDD B-017 (DEFERRED_GLOBAL).

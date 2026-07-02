# TLA+ Temporal Model Plan — vb-qi37.1.4

## Boundary

- **Temporal/workflow behavior**: Recovery event replay, lifecycle event handling, unsupported state propagation through replay, and fail-closed gating at the runtime boundary.
- **Rust/core behavior excluded from TLA+**: Frame construction, slot write operations, step state marking, PC bounds checking, `RunFrame` internal invariants, `ActionReplayTracker` HashSet operations. These are handled by Verus proofs.
- **External systems abstracted**: Fjall journal durability, snapshot encoding/decoding, wire format for `JournalEvent`.
- **Non-applicability rationale**: Not applicable — temporal/state-over-time behavior is central to this contract.

---

## TLA+-Owned Clauses

| Clause | Module | Property |
|---|---|---|
| INV-RC-007 | `RecoveryReplay.tla` | `RunResumed`, `RunRetried`, `RunAnswered` events are present in replay output and visible to downstream state extraction |
| WF-RC-001 | `RecoveryReplay.tla` | Liveness: every valid `RecoveryFrameSeed` eventually reaches `hydrate_run_frame` termination (ok or error) |
| SF-RC-001 | `RecoveryReplay.tla` | Safety: `reject_unsupported_live_frame_state` never returns `Ok` when any of the 4 unsupported flags are `true` |
| ACT-RC-001 | `RecoveryReplay.tla` | `action_payloads: true` in seed implies the frame cannot be used to consume action results |

---

## Model Shape

### Module: `RecoveryReplay`

```
VARIABLES
  seed         \in [unsupported: [slot_values: BOOL, slot_taint: BOOL,
                                   action_payloads: BOOL, pending_actions: BOOL],
                  pending_actions: SEQ(PendingAction)],
  replay_buf  \in Seq(JournalEvent),
  hydration_ok \in BOOLEAN

TypeOK ==
  /\ seed \in [unsupported: UnsupportedState, pending_actions: Seq(PendingAction)]
  /\ replay_buf \in Seq(JournalEvent)
  /\ hydration_ok \in BOOLEAN
```

### UnsupportedState
```
UnsupportedState == [
  slot_values: BOOLEAN,
  slot_taint: BOOLEAN,
  action_payloads: BOOLEAN,
  pending_actions: BOOLEAN
]
```

### Init
```
Init ==
  /\ seed = [unsupported |-> [slot_values |-> FALSE, slot_taint |-> FALSE,
                               action_payloads |-> FALSE, pending_actions |-> FALSE],
             pending_actions |-> <<>>]
  /\ replay_buf = <<>>
  /\ hydration_ok = FALSE
```

### Actions

**SetActionPayloadsUnsupported**
```
SetActionPayloadsUnsupported ==
  /\ seed.unsupported.action_payloads = FALSE
  /\ seed' = [seed EXCEPT !.unsupported.action_payloads = TRUE]
  /\ UNCHANGED <<replay_buf, hydration_ok>>
```

**SetSlotValuesUnsupported**
```
SetSlotValuesUnsupported ==
  /\ seed.unsupported.slot_values = FALSE
  /\ seed' = [seed EXCEPT !.unsupported.slot_values = TRUE]
  /\ UNCHANGED <<replay_buf, hydration_ok>>
```

**RejectUnsupportedState** (the fail-closed gate)
```
RejectUnsupportedState ==
  /\ \/ seed.unsupported.slot_values = TRUE
     \/ seed.unsupported.slot_taint = TRUE
     \/ seed.unsupported.action_payloads = TRUE
     \/ (seed.unsupported.pending_actions = TRUE /\ Len(seed.pending_actions) > 0)
  /\ hydration_ok' = FALSE
  /\ UNCHANGED <<seed, replay_buf>>
```

**AcceptSupportedState** (hydration succeeds)
```
AcceptSupportedState ==
  /\ seed.unsupported.slot_values = FALSE
  /\ seed.unsupported.slot_taint = FALSE
  /\ seed.unsupported.action_payloads = FALSE
  /\ \/ seed.unsupported.pending_actions = FALSE
     \/ Len(seed.pending_actions) = 0
  /\ hydration_ok' = TRUE
  /\ UNCHANGED <<seed, replay_buf>>
```

**ReplayLifecycleEvent** (RunResumed/RunRetried/RunAnswered absorbed)
```
ReplayLifecycleEvent(e) ==
  /\ e \in {RunResumed, RunRetried, RunAnswered}
  /\ replay_buf' = Append(replay_buf, e)
  /\ UNCHANGED <<seed, hydration_ok>>
```

### Invariants

**SafeHydration** (fail-closed safety)
```
SafeHydration ==
  hydration_ok = TRUE
    => /\ seed.unsupported.slot_values = FALSE
       /\ seed.unsupported.slot_taint = FALSE
       /\ seed.unsupported.action_payloads = FALSE
       /\ \/ seed.unsupported.pending_actions = FALSE
          \/ Len(seed.pending_actions) = 0
```

**LifecycleEventsNotDropped**
```
LifecycleEventsNotDropped ==
  \A e \in {RunResumed, RunRetried, RunAnswered}:
    e \in DOMAIN replay_buf
```

### Temporal Properties

**EventuallyHydratedOrRejected**
```
EventuallyHydratedOrRejected ==
  <>(\/ hydration_ok = TRUE \/ hydration_ok = FALSE)
```

**NoSpuriousActionPayloads**
```
NoSpuriousActionPayloads ==
  seed.unsupported.action_payloads = TRUE
    => hydration_ok = FALSE
```

### Fairness

- Weak fairness on `SetSlotValuesUnsupported` and `SetActionPayloadsUnsupported` — these represent storage-side detection and may fire at any time.
- No fairness on `RejectUnsupportedState` — it is a deterministic gate.
- No fairness on `AcceptSupportedState` — it depends on all unsupported flags being false.

### Bounded Model Limits

- `pending_actions` bounded by `Nat \in 0..10` for TLC.
- `replay_buf` bounded by `Nat \in 0..20` for TLC.
- Symmetry set disabled for `UnsupportedState` (all combinations of 4 bools = 16 states).

---

## Properties

| Property | Type | Description |
|---|---|---|
| `SafeHydration` | Invariant | Hydration succeeds only when all 4 unsupported flags are false (or pending_actions empty) |
| `LifecycleEventsNotDropped` | Invariant | Lifecycle events appear in replay buffer |
| `EventuallyHydratedOrRejected` | Liveness | Recovery always terminates |
| `NoSpuriousActionPayloads` | State constraint | `action_payloads: true` forces hydration failure |

---

## Refinement to Rust/runtime Behavior

| TLA+ Symbol | Rust/runtime Refinement |
|---|---|
| `seed.unsupported` | `RecoveryFrameSeed::unsupported` (all 4 flags) |
| `seed.pending_actions` | `RecoveryFrameSeed::pending_actions` |
| `RejectUnsupportedState` | `reject_unsupported_live_frame_state()` in `vb_runtime/src/recovery.rs` |
| `replay_buf` | Output of `replay_events()` in `vb_storage/src/recovery/replay/core.rs` |
| `hydration_ok = TRUE` | `hydrate_run_frame() -> Ok(frame)` |
| `hydration_ok = FALSE` | `hydrate_run_frame() -> Err(RuntimeError::InvalidRecoveryHydration)` |

---

## Evidence Command

```
tlc -config RecoveryReplay.cfg RecoveryReplay.tla
```

Expected: TLC reports no invariant violations, no deadlock, and all temporal properties satisfied for the model bounds above.

---

## Waivers

| Clause | Owner | Reason | Expiry |
|---|---|---|---|
| Fjall journal durability | Storage layer | Out of scope for runtime boundary | N/A |
| Snapshot codec correctness | Kani harness | Covered by `vb_storage/src/kani_codec.rs` | N/A |
| Action retry backoff policy | Out of scope | Not a recovery safety concern | N/A |

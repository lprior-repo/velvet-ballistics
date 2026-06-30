# Boundary Map — vb_storage vs vb_core for Idempotency Hydration

## Crate Boundary

```
┌─────────────────────────────────────────────────────────────┐
│                        vb_core                              │
│  crates/vb_core/src/action.rs                              │
│  crates/vb_core/src/ids.rs                                  │
│  crates/vb_core/src/frame.rs                               │
├─────────────────────────────────────────────────────────────┤
│  EXPORTS:                                                  │
│  - ActionTicket (entity)                                   │
│  - Idempotency, SideEffect, RetrySafety (enums)            │
│  - compute_action_idempotency_key (pure fn)                │
│  - action_ticket_has_valid_key (pred)                      │
│  - validate_idempotency_key_ingredients (fn)                │
│  - verify_idempotency (fn)                                 │
│  - ActionContract, ActionInput, ActionOutput (structs)       │
│  - ActionError, IdempotencyViolation (errors)               │
│  - ActionOutcome, ActionOutputReady, ActionFailure          │
│  - ActionJournalEvent (journal serialization)               │
├─────────────────────────────────────────────────────────────┤
│  OWNED DOMAIN:                                             │
│  - ActionTicket identity and invariants                     │
│  - Idempotency key computation (pure, no side effects)    │
│  - Key ingredient validation (pure, no I/O)                │
│  - ActionContract static verification                      │
│  - Error types for action subsystem                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ imports ActionTicket, Idempotency,
                              │ RetrySafety, compute_action_idempotency_key
                              │ from vb_core (cfg: no deps on vb_storage)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      vb_storage                             │
│  crates/vb_storage/src/recovery/types.rs                    │
│  crates/vb_storage/src/recovery/hydrate.rs                 │
│  crates/vb_storage/src/recovery/hydrate_support.rs         │
│  crates/vb_storage/src/recovery/replay/                    │
│  crates/vb_storage/src/events.rs                            │
├─────────────────────────────────────────────────────────────┤
│  IMPORTS FROM vb_core:                                     │
│  - ActionTicket, ActionId, RunId, SeqNo, StepIdx            │
│  - Idempotency, RetrySafety, SideEffect                    │
│  - compute_action_idempotency_key                          │
│  - ActionJournalEvent                                       │
├─────────────────────────────────────────────────────────────┤
│  OWNED DOMAIN:                                             │
│  - ActionReplayTracker (entity)                            │
│  - RecoveryError, RecoveryHydration (types)                 │
│  - RunFrame hydration (snapshot + tail events)            │
│  - JournalEvent durable record types                       │
│  - ActionReplayEffect enum                                 │
│  - SnapshotRecoveryInputViolation                          │
└─────────────────────────────────────────────────────────────┘
```

## Boundary Rules

### vb_core → vb_storage (one-way import)

**Allowed**: vb_storage may import any public type from vb_core.

**NOT Allowed**: vb_core may NOT import from vb_storage. The core is pure and has no knowledge of storage.

### Hydration Boundary

```
Hydration Input (vb_storage owns)          Hydration Output (vb_core RunFrame)
┌────────────────────────────────┐         ┌────────────────────────────────┐
│ - RunSnapshot (slots, taint)  │         │ - RunFrame (live runtime)       │
│ - tail_events: Vec<JournalEvent>│  ──►   │   reconstructed from snapshot   │
│ - run_id: RunId                │         │   and tail events               │
│ - ActionReplayTracker          │         │                                 │
│   (internal to hydration)      │         │                                 │
└────────────────────────────────┘         └────────────────────────────────┘
```

**vb_storage is responsible for**:
- Reading snapshot bytes and decoding into `RunSnapshot`.
- Iterating `tail_events` in sequence order.
- Maintaining `ActionReplayTracker` state during replay.
- Enforcing idempotency constraints via `ActionReplayTracker`.
- Returning `RecoveryResult<vb_core::RunFrame>`.

**vb_core is responsible for**:
- The structure of `RunFrame` and its slot/taint state.
- The pure `compute_action_idempotency_key` function.
- The validation predicates for key ingredients.

### Journal/Storage Ownership

| Concern | Owner | Location |
|---------|-------|----------|
| `ActionScheduledTicket` event record | vb_storage (events.rs) | Journal |
| `ActionCompletedEnvelope` event record | vb_storage (events.rs) | Journal with `value_digest` |
| `ActionFailedEvent` event record | vb_storage (events.rs) | Journal |
| ActionReplayTracker persistence | vb_storage | In-memory during hydration only |
| Snapshot persistence | vb_storage | Fjall keyspace |
| ActionTicket identity validation | vb_core (via vb_storage calls) | `action_ticket_has_valid_key` |
| Idempotency key validation | vb_core (via vb_storage calls) | `validate_idempotency_key_ingredients` |

### Key Transfer Point

The key transfer from vb_storage to vb_core happens at:

```rust
// vb_storage/src/recovery/hydrate_support.rs:57
pub(crate) fn verify_action_ticket_event(run: RunId, ticket: ActionTicket) -> RecoveryResult<()>
```

This calls `vb_core::action_ticket_has_valid_key(ticket)` to verify the ticket's idempotency_key is canonical.

### Forbidden Boundary Crossings

1. **F1**: vb_core MUST NOT contain `use vb_storage::*` or any reference to `ActionReplayTracker`.
2. **F2**: vb_core MUST NOT call any vb_storage function. It is side-effect free.
3. **F3**: vb_storage hydration MUST NOT create `ActionTicket` directly; it must use events already recorded by vb_runtime.
4. **F4**: `ActionReplayTracker` MUST NOT be serialized to the journal; it is reconstructed from events on each hydration.

### Async Boundary

Hydration is **synchronous** (no `async`). The `hydrate_run_frame` and `hydrate_run_frame_from_events` functions are `fn`, not `async fn`. Recovery is I/O-free once snapshot bytes are loaded.

### Time/FFI Boundary

There is **no FFI** in the hydration path. All data comes from:
- In-memory `RunSnapshot` (loaded from vb_storage keyspace)
- In-memory `&[JournalEvent]` slice (loaded from vb_storage journal)

No external time services are consulted during hydration. `idempotency_key` computation is purely functional from `(run, seq, action)`.

### Storage Boundary

```
vb_storage keyspace (Fjall):
  - "compiled_ir" keyspace: stores CompiledIrRecord (artifact + digest)
  - "snapshot" keyspace: stores RunSnapshot per (run_id, seq)
  - "journal" keyspace: stores JournalEvent records per (run_id, seq)

vb_storage hydration reads from these keyspaces and produces vb_core::RunFrame.
```
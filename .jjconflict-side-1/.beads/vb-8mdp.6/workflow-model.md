# Workflow Model — ActionTicket Idempotency Hydration

## Hydration Workflow State Machine

```
┌─────────────────────────────────────────────────────────────────────┐
│                      HYDRATION_WORKFLOW                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐                                                   │
│  │   START      │                                                   │
│  └──────────────┘                                                   │
│         │                                                           │
│         ▼ hydrate_snapshot_tail_preconditions(run_id)               │
│         │                                                           │
│    ┌────┴────┐                                                      │
│    │ PRECOND │                                                      │
│    │  PASS   │◄─── true                                             │
│    └────┬────┘                                                      │
│         │ false                                                     │
│         ▼                                                           │
│  ┌──────────────┐     RecoveryError                                 │
│  │  PRECOND     │ ────────────────────────────────────────────────►│
│  │  FAIL        │                                                   │  ERROR_TERMINAL
│  └──────────────┘                                                   │
│         │                                                           │
│         │ true                                                      │
│         ▼                                                           │
│  ┌──────────────┐                                                   │
│  │ HYDRATING   │◄─── snapshot + tail_events                         │
│  │ (frame new) │                                                   │
│  └──────┬───────┘                                                   │
│         │                                                           │
│         ▼ apply_tail_events(tracker)                                │
│         │                                                           │
│    ┌────┴────┐                                                      │
│    │TAIL_EVT │                                                      │
│    │ APPLY   │                                                      │
│    └────┬────┘                                                      │
│         │                                                           │
│    ┌────┴────────────────────────────────────┐                      │
│    │  match JournalEvent variant:            │                      │
│    │                                         │                      │
│    │  ActionScheduledTicket ──────────────┐  │                      │
│    │       │                               │  │                      │
│    │       ▼ mark_scheduled_ticket_effect  │  │                      │
│    │       │                               │  │                      │
│    │  ┌────┴────┐                          │  │                      │
│    │  │ Apply   │ ──────────────────────────│  │                      │
│    │  │ Duplicate                        │  │  │                      │
│    │  └────┬────┘                          │  │                      │
│    │       │                               │  │                      │
│    │  ActionCompletedEnvelope ──────────┐   │  │                      │
│    │       │                            │   │  │                      │
│    │       ▼ mark_completed_envelope_effect   │  │                      │
│    │       │                            │   │  │                      │
│    │  ┌────┴────┐                       │   │  │                      │
│    │  │ Apply   │                       │   │  │                      │
│    │  │ Duplicate (skip, already done)  │   │  │                      │
│    │  └────┬────┘                       │   │  │                      │
│    │       │                            │   │  │                      │
│    │  ActionFailedEvent ─────────────┐  │   │  │                      │
│    │       │                         │  │   │  │                      │
│    │       ▼ mark_failed             │  │   │  │                      │
│    │       │                         │  │   │  │                      │
│    │  SCHEDULED entry blocked        │  │   │  │                      │
│    └───────┼─────────────────────────┼──┼───┼──────────────────────┤
│            │                         │  │   │                      │
│            │ (continue processing)   │  │   │                      │
│            ▼                         ▼  ▼   ▼                      │
│     ┌──────────────┐                                           │
│     │  TAIL_DONE   │                                           │
│     └──────┬───────┘                                           │
│            │ increment_executed                                 │
│            ▼                                                   │
│     ┌──────────────┐                                           │
│     │   HYDRATED  │                                           │
│     │  RunFrame    │                                           │
│     └──────────────┘                                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

ERROR_TERMINAL:
  - RecoveryError::NonIdempotentActionBlocked { action, step }
  - RecoveryError::ReplayDivergence { step, detail }
  - RecoveryError::WorkflowSourceDigestMismatch { expected, found }
  - RecoveryError::CorruptSnapshot { run, seq }
```

## ActionReplayTracker Sub-State Machine

```
                    new()
                       │
                       ▼
            ┌─────────────────────┐
            │      EMPTY          │
            │ (no entries)        │
            └──────────┬──────────┘
                       │
          mark_scheduled_ticket_effect
                       │
          ┌────────────┼────────────┐
          │            │            │
       Apply       Duplicate     Divergence
          │            │            │
          ▼            ▼            ▼
  ┌───────────┐  ┌───────────┐  ┌───────────┐
  │ SCHEDULED │  │ SCHEDULED │  │  ERROR   │
  │ (entry    │  │ (entry    │  │(Replay   │
  │  stored)  │  │  stored)  │  │ Divergence)
  └─────┬─────┘  └───────────┘  └───────────┘
        │
        │
   mark_completed_envelope_effect
        │
   ┌────┴────┐
   │         │
 Apply   Duplicate
   │         │
   ▼         ▼
┌─────────┐  ┌───────────┐
│COMPLETED│  │ COMPLETED │
│ (entry  │  │ (no new  │
│ stored) │  │  entry)  │
└─────────┘  └───────────┘
```

## Happy Path: Deterministic Idempotency Hydration

1. **Ticket Issued** (normal execution):
   - Runtime calls `issue_action_ticket(run, step, seq, action, attempt, key, capacity)`
   - `ActionScheduledTicket` event written to journal with full ticket
   - `ActionCompletedEnvelope` event written with `value_digest`

2. **Hydration Reconstruction**:
   - `hydrate_run_frame` called with snapshot + tail_events
   - `apply_tail_events` iterates events in sequence order
   - For `ActionScheduledTicket`: `mark_scheduled_ticket_effect` → `Apply` (first time) or `Duplicate` (subsequent)
   - For `ActionCompletedEnvelope`: `mark_completed_envelope_effect` → `Apply` (first time) or `Duplicate` (subsequent)
   - Tracker prevents non-idempotent re-execution

3. **Determinism Guarantee**:
   - Same `(run, seq, action)` always produces same `idempotency_key`
   - `value_digest` in envelope ensures byte-exact output matching
   - Divergent tickets or envelopes produce typed errors, not silent corruption

## Failure Paths

| Path | Trigger | Error | Recovery |
|------|---------|-------|----------|
| F1 | `KeyRequired` action with `idempotency_key == 0` | `NonIdempotentActionBlocked` | Must replay with valid key |
| F2 | Second scheduled ticket with different ticket for same `(action, step)` | `ReplayDivergence` | Manual intervention |
| F3 | Completion envelope with different `value_digest` | `ReplayDivergence` | Manual intervention |
| F4 | Completion for unresolved action | `ReplayDivergence` | Must have schedule first |
| F5 | Non-idempotent action encountered during recovery | `NonIdempotentActionBlocked` | Cannot replay; fail closed |

## Terminal States

- **HYDRATED(RunFrame)**: Successful reconstruction with tracker tracking all action states.
- **ERROR(RecoveryError)**: Typed error describing the specific failure.
- **No partial state**: Hydration is atomic — either fully succeeds or returns typed error.
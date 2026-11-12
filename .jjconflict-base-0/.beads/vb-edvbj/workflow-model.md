# Workflow Model — vb-edvbj

## Process: dispatching a `RuntimeJournalEvent` into persistent storage

### Actors

- **Caller** — any upstream site that calls `RuntimeJournal::append_sequenced` (today: `RuntimeShard::append_journal_event` at `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:194-199`).
- **Dispatcher** — `StorageRuntimeJournal::storage_event` (`chunk_002.rs:270-303`).
- **Layer helpers** — `run_storage_event`, `action_storage_event`, `boundary_storage_event` (`chunk_002.rs:41-268`). All `Option`-returning except `boundary_storage_event` which is `RuntimeResult<Option<JournalEvent>>`.
- **Storage sink** — `FjallJournal::append_journaled` / `append_strict` (depending on `DurabilityProfile`).

### Legal states (pre-fix vs post-fix)

| State | Pre-fix | Post-fix |
| ----- | ------- | -------- |
| Variant hits an explicit run-layer arm (e.g. `RunFailed { run }`) | `Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` | `Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` (preserved) |
| Variant hits an explicit run/action-layer arm with `Some(...)` | `Ok(JournalEvent::...)` | `Ok(JournalEvent::...)` (preserved) |
| Variant hits an explicit boundary-layer arm (e.g. `WaitScheduled`) | `Ok(Some(JournalEvent::WaitScheduledEvent))` → unwrapped → `Ok(JournalEvent)` | `Ok(Some(JournalEvent::WaitScheduledEvent))` → unwrapped → `Ok(JournalEvent)` (preserved) |
| **All helpers return `None`** | **Fabricate `Ok(JournalEvent::RunFailedEvent)`** (BUG) | **`Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })`** (FIX) |
| Boundary helper returns `Err(_)` (e.g. `EncodeFailed`) | Propagated via `?` | Propagated via `?` (preserved) |

### State machine

```
                   ┌──────────────────────────────┐
                   │  caller: append_sequenced    │
                   │  (event, seq)                │
                   └──────────────┬───────────────┘
                                  │
                                  ▼
                   ┌──────────────────────────────┐
                   │  dispatcher storage_event    │
                   └──────────────┬───────────────┘
                                  │
            ┌─────────────────────┼────────────────────┐
            │                     │                    │
   run/arm  ▼              action/arm  ▼          boundary/arm ▼
   run_storage_event       action_storage_event   boundary_storage_event
            │                     │                    │
   Some(e) │     None ←──        │   None ←──          │   Err(_) → bubble
            ▼                     ▼                    ▼
   ┌────────────────┐   ┌────────────────┐    ┌──────────────────┐
   │ Ok(Some(e))    │   │ Ok(None)        │   │ Ok(Some(e))       │
   │ → Ok(e)        │   │ → Err(UNMAP)   │    │ → Ok(e)            │
   │                │   │   (post-fix)    │   │  Ok(None) →        │
   │                │   │                │   │   Err(UNMAP)       │
   │                │   │                │   │   (post-fix)       │
   └────────────────┘   └────────────────┘    └──────────────────┘
                  ▲                              ▲
                  └──────────────┬───────────────┘
                                 ▼
                   ┌──────────────────────────────┐
                   │ RuntimeResult<JournalEvent>  │
                   └──────────────────────────────┘
                                  │
                                  ▼
                   ┌──────────────────────────────┐
                   │ FjallJournal::append_*       │
                   │   (StorageRuntimeJournal      │
                   │    .append_storage_event)     │
                   └──────────────────────────────┘
```

### Terminal states

1. **`Ok(JournalEvent)` written to Fjall.** Pure success. Reachable via:
   - Explicit run-layer mapping.
   - Explicit action-layer mapping.
   - Explicit boundary-layer `Some(...)` mapping.
2. **`Err(RuntimeError::UnmappedRuntimeJournalEvent)` propagates to caller.** Reachable via:
   - All three layer helpers return `None` (today: only `Resumed` does this — every other variant has an explicit arm in at least one helper).
   - Or the boundary helper returns `Ok(None)` (any variant that the dispatcher's boundary-arm group sends to the boundary and the boundary does not handle — today, none).

   The caller pattern is preserved: caller uses `?`. Eventually `RuntimeShard::append_journal_event` returns the error to whatever site called it (e.g. `handle_resume`).
3. **`Err(RuntimeError::EncodeFailed)`** propagates from boundary layer (unchanged).
4. **`Err(RuntimeError::StorageJournalAppend { source })`** propagates from `FjallJournal::append_*` (unchanged).

### Idempotence requirement

After fix:
- `append_sequenced` MUST NOT write a fabricated `JournalEvent::RunFailedEvent` for an unmapped variant.
- A retry of the call after the fix on the same `event/seq` MUST yield the same error (`Err(UnmappedRuntimeJournalEvent { event_kind })`), not silently succeed.
- Pre-existing Fjall state from the buggy era that does contain a fabricated `RunFailedEvent` for what was actually a `Resumed` MUST be handled by out-of-band analysis, not by `storage_event` (which is forward-only).

### Cancellation path

`storage_event` is synchronous and non-cancellable. No cancellation hazard introduced.

### Retry / shutdown interaction

`QueuedStorageRuntimeJournal::append_sequenced` already rejects `DurabilityProfile::Strict` with `Err(UnsupportedAsyncStrictAck)` before reaching `storage_event`. The new typed error surfaces independently of that rejection. Retry loops that previously depended on a successful `Ok(RunFailedEvent)` after `Resumed` were the broken path; after the fix they surface a typed error and the caller decides — typically by surfacing it to the operator through the same channel as `RunNotFound`.

### Temporal / recovery correctness (RE-019 forward-port)

Today, recovery in `crates/vb_storage/src/recovery/replay/observation/normalize.rs:126-127` and `crates/vb_storage/src/journal/incident.rs:203` observe `JournalEvent::RunResumed` and classify the run as `LifecycleState::Active`. The buggy dispatcher corrupts this by writing `JournalEvent::RunFailedEvent` for what was really `Resumed`, so on replay the run is mis-classified as `LifecycleState::Failed`. The fix guarantees that no `Resumed` event is ever written to storage as a `RunFailedEvent`. After the fix:

- If the integration chooses Option-A-only (this bead), `Resumed` events through `StorageRuntimeJournal` will surface `Err(UnmappedRuntimeJournalEvent)` instead of being journaled. Recovery code that depends on `Resumed` reaching the journal needs the parallel Option-B fix in a follow-up bead (out of scope here).
- Until that follow-up, recovery for storage-journaled runs is blocked at `append_sequenced`. The flag-down behaviour is correct (it stops persisting fake failures) but the recovery path is not yet restored.

This is recorded as hazard **H-1** in `hazard-analysis.md`.

### Operational invariants

- After fix, no successful `append_sequenced` may persist a `JournalEvent` whose `seq` field does not match the caller-supplied `seq`. (preserved)
- After fix, the only path that produces `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` is `RuntimeJournalEvent::RunFailed { run } → run_storage_event` arm.
- After fix, `append_sequenced` returns `Err(UnmappedRuntimeJournalEvent)` deterministically for any unmapped variant.

## Forbidden transitions

- **Any `Ok(JournalEvent::RunFailedEvent)` produced from a `RuntimeJournalEvent` other than `RunFailed { run }`.** Strict state-machine invariant; the bug fix enforces this invariant and removes the prior freedom.
- **Any successful `Ok(JournalEvent)` that hides an unmapped variant as a silent failure-class record.** Forbidden.

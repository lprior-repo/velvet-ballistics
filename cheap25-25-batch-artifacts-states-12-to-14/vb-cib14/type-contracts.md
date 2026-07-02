# Type Contracts: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Desired Type Shape

These are contracts for downstream implementation/proof planning, not implementation code.

```text
// Pure mapping decision (extracted from boundary_storage_event arm):
resume_storage_decision(event: RuntimeJournalEvent::Resumed, seq: EventSeq)
  -> Result<JournalEvent::RunResumed, RuntimeError::ResumeTimestampOverflow>

// With overflow split:
resume_storage_decision(event: RuntimeJournalEvent::Resumed, seq: EventSeq)
  -> Result<JournalEvent::RunResumed, RuntimeError::ResumeTimestampOverflow { run, timestamp }>
```

## Public API Contracts

| API | Precondition | Success postcondition | Failure postcondition |
|---|---|---|---|
| `StorageRuntimeJournal::storage_event(RuntimeJournalEvent::Resumed { run, timestamp }, seq)` | `event.run_id() != RunId(0)` (consistency with `JournalEvent::is_valid()`); `seq != EventSeq::MAX`. | Returns `Ok(JournalEvent::RunResumed { run, seq, timestamp })` where `timestamp = DateTime::<Utc>::from_timestamp(i64::try_from(timestamp_u64)?, 0).expect("from_timestamp returns Some for i64 in valid range")` is bound to the converted chrono value. | Returns `Err(RuntimeError::ResumeTimestampOverflow { run, timestamp })` when `timestamp > i64::MAX`; never panics; never silently rewrites to `RunFailedEvent`. |
| `StorageRuntimeJournal::append_sequenced(RuntimeJournalEvent::Resumed, seq)` | Same as above plus `FjallJournal` is reachable. | `Ok(())` and the durable journal contains exactly one `JournalEvent::RunResumed` with the supplied `seq` and converted `timestamp`. | `Err(ResumeTimestampOverflow)` propagates without touching the durable journal; `Err(journal_append_failed)` only if the conversion succeeded and the durable append failed. |
| `StorageRuntimeJournal::storage_event(_)` totality | Caller supplies any `RuntimeJournalEvent`. | After this fix, every variant has an explicit arm. Once vb-edvbj removes the catch-all, dispatch must compile (no missing arms) and produce the typed event or error. | No variant may fall through to a synthetic `RunFailedEvent`. |

## Shard Internal Type Contracts

### Boundary Dispatch Family

`boundary_storage_event` must recognize the closed family:

```text
BoundaryEventFamily = WaitScheduled | WaitResolved
                    | AskScheduled | AskAnswered | AskTimedOut
                    | SlotWritten
                    | Resumed
```

- Each family member must produce exactly one `JournalEvent` carrying the shard-owned `seq`.
- Non-family variants must continue to fall into `run_storage_event` or `action_storage_event` or be explicit no-ops (the existing `Resumed { .. } => None` patterns in `run_storage_event` and `action_storage_event` are intentional).

### Resumed Mapping Function Contract

Conceptual pure core:

```text
map_resumed_to_run_resumed(
    run: RunId,
    timestamp_u64: u64,
    seq: EventSeq,
) -> Result<JournalEvent::RunResumed, RuntimeError::ResumeTimestampOverflow>
```

Required invariants:

- `RunId(run)` is preserved exactly. The mapper does not normalize or zero the run id.
- `seq` is passed through without transformation.
- `timestamp_u64` is converted via `i64::try_from(timestamp_u64) → i64_secs`, then `DateTime::<Utc>::from_timestamp(i64_secs, 0)`. The two-step conversion is non-negotiable: skipping `i64::try_from` would either panic (cast) or wrap (modular); neither is acceptable.
- The conversion function is total over `u64`. `Err(ResumeTimestampOverflow { run, timestamp: timestamp_u64 })` is returned exactly when `timestamp_u64 > i64::MAX`.
- The function must not panic for any `u64` input. `DateTime::<Utc>::from_timestamp(_, 0)` must be called only after `i64::try_from` succeeds.

### Storage Totality Contract

```text
forall ev: RuntimeJournalEvent, seq: EventSeq.
    storage_event(ev, seq) -> Result<JournalEvent, RuntimeError>   (no implicit fallthrough)
```

- The dispatch in `storage_event` must remain exhaustive over `RuntimeJournalEvent`.
- After vb-edvbj deletes the `RunFailedEvent` fallback (lines 298-302), the function body must continue to compile.
- No new fallback or "synthetic event" arm may be added.

### Single-Clone Dispatch Contract

For every call to `StorageRuntimeJournal::storage_event(ev, seq)`:

- `STORAGE_EVENT_CLONE_COUNT` increases by exactly 1 in test builds.
- The event is moved into exactly one helper (`run_storage_event`, `action_storage_event`, or `boundary_storage_event`); it is never cloned again before consumption.
- The `Resumed` arm must follow the same `&event` + `clone_for_dispatch` pattern as the existing boundary arms.

## Storage Type Contracts

| Symbol | Contract |
|---|---|
| `JournalEvent::RunResumed` | Carries `run: RunId`, `seq: EventSeq`, `timestamp: DateTime<Utc>`. Already defined at `crates/vb_storage/src/events.rs:289-297`. |
| `JournalEvent::record_kind()` | Returns `RecordKind::RunResumed` for `RunResumed`. Already implemented at `events.rs:424`. |
| `JournalEvent::run_id()` | Returns `run` for `RunResumed`. Already implemented at `events.rs:358`. |
| `JournalEvent::seq()` | Returns `seq` for `RunResumed`. Already implemented at `events.rs:392`. |
| `JournalEvent::is_valid()` | `RunResumed` is in the no-attempt, no-ticket set; `run_id().get() != 0` and `seq().get() != u64::MAX`. Mirror at `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:792,839`. |
| `incident.rs::lifecycle_state(RunResumed)` | Returns `LifecycleState::Active` at `crates/vb_storage/src/journal/incident.rs:203`. |
| `recovery/hydrate.rs::is_in_flight_or_completed(RunResumed)` | Returns `Ok(false)` at `crates/vb_storage/src/recovery/hydrate.rs:754`. |
| `recovery/replay/observation/normalize.rs` | `RunResumed` is in the recovery observation classifier at lines 60, 126. |

## Error Surface Contract

| Domain condition | Required classification | Notes |
|---|---|---|
| `RuntimeJournalEvent::Resumed { timestamp, .. }` where `timestamp > i64::MAX` | `RuntimeError::ResumeTimestampOverflow { run, timestamp }` (new variant). | Must not panic; must not wrap; must not silently clamp to `i64::MAX`. |
| `StorageRuntimeJournal::storage_event` exhaustiveness loss | Compile-time error from Rust's match-exhaustiveness checker. | This is the desired failure mode once vb-edvbj deletes the catch-all and any new variant is missing — the compiler must reject it, not a silent rewrite. |
| Existing journal append failure after conversion succeeds | `RuntimeError::StorageJournalAppend` (or the existing journal-error carrier). | Conversion errors and append errors must not be conflated. |
| Existing runtime dispatch errors (e.g. `EncodeFailed`) | Unchanged. | `SlotWritten` path through `boundary_storage_event` already uses `encoded_slot_taint_extra`; that path is not altered by this fix. |

## Conversion Helper Contract (Conceptual)

```text
// Pure helper, free of I/O, no panics:
fn convert_resume_timestamp(timestamp_u64: u64, run: RunId)
    -> Result<DateTime<Utc>, RuntimeError::ResumeTimestampOverflow>;

// Pre: timestamp_u64 is the value carried by RuntimeJournalEvent::Resumed.
// Post: returns Ok(DateTime::<Utc>::from_timestamp(secs, 0))
//       where secs = i64::try_from(timestamp_u64).map_err(|_| ResumeTimestampOverflow)?
//       and from_timestamp is total over the i64 range (Some(_) for the realistic range).
// Failure: returns Err(ResumeTimestampOverflow { run, timestamp: timestamp_u64 }) iff timestamp_u64 > i64::MAX.
// Properties:
//   1. The function never panics.
//   2. The function never silently clamps or wraps.
//   3. The function never reads global state.
//   4. The function is deterministic.
```

## Illegal States to Make Unrepresentable

- `RuntimeJournalEvent::Resumed` flowing into a `JournalEvent::RunFailedEvent`.
- `storage_event` falling through to a synthetic event when a real event variant exists.
- `RuntimeJournalEvent::Resumed { timestamp: u64 }` flowing into a storage event without timestamp conversion (e.g. as raw `u64`).
- `seq` being silently discarded or invented for `RunResumed`.
- Two full-event clones during a single `storage_event` call.
- `i64::try_from(timestamp)` skipped or replaced with `as i64` cast.
- `DateTime::<Utc>::from_timestamp` called on a `u64` directly.

## Verus Mirror Binding (Strong / Weak / Drift)

- `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:616-624`: mirror of `JournalEvent::RunResumed { run, seq, timestamp }` shape.
- `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:715,748,792,839`: mirror references for `RunResumed` in `run_id`, `seq`, and `is_valid` mirrors. The production `events.rs` already matches these mirror signatures, so the fix introduces no new mirror drift; the mirrors stay accurate after the mapper arm is added.
- The fix must NOT alter the `JournalEvent::RunResumed` shape. Any change to the field order, naming, or types would invalidate the mirror. The mapper arm is the only production code site that changes.
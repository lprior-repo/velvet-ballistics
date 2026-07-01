# Domain Model: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Scope

- Bead: `vb-cib14`
- State: 3 / `rust-contract`
- Coupled with: `vb-edvbj` (deletes the synthetic `RunFailedEvent` fallback) — must land in the same release.
- Feature slice: runtime-to-storage event dispatch for the resume lifecycle event. Replaces the silent `Resumed → RunFailedEvent` rewrite that currently mis-classifies resumed runs as failed during recovery/replay.
- Out of scope for this state: production Rust implementation, behavior tests, verifier artifacts, proof obligations, and proof review approval.

## Ubiquitous Language

| Term | Meaning | Contract relevance |
|---|---|---|
| `RuntimeJournalEvent::Resumed` | Runtime-side event produced by `Shard::append_resumed_event`. Carries `run: RunId` and a `timestamp: u64` seconds-since-epoch from `current_timestamp()`. | Source shape that must reach `JournalEvent::RunResumed`. |
| `JournalEvent::RunResumed` | Storage-side event with `run: RunId`, `seq: EventSeq`, `timestamp: DateTime<Utc>`. Already exists in `vb_storage/src/events.rs:289-297` and is recognized by `record_kind()` (`RecordKind::RunResumed`), `run_id()`, and `seq()`. | Target shape of the dispatch. |
| `StorageRuntimeJournal` | Runtime journal adapter that maps `RuntimeJournalEvent` to `JournalEvent` via three per-family helpers (`run_storage_event`, `action_storage_event`, `boundary_storage_event`) and a final catch-all synthetic `RunFailedEvent`. | Site of the fix; the buggy `_ =>` arm in `storage_event` is replaced by an explicit `Resumed` arm. |
| `ResumedEvent::BoundaryDispatcher` | Domain concept: the family of `RuntimeJournalEvent` variants currently routed through `boundary_storage_event` (`WaitScheduled`, `WaitResolved`, `Ask*`, `SlotWritten`, `Resumed`). | `Resumed` belongs here, consistent with the existing per-arm shape. |
| `ResumedMapping` | Pure decision: `RuntimeJournalEvent::Resumed { run, timestamp: u64 } → JournalEvent::RunResumed { run, seq: EventSeq, timestamp: DateTime<Utc> }`. | The conversion contract — `(u64 → DateTime<Utc>)` and `seq` pass-through. |
| `ResumeTimestamp` | `u64` seconds since UNIX epoch, sourced from `current_timestamp()` in `shard/lifecycle/chunk_003.rs:24-28`. | Must be parseable into a chrono `DateTime<Utc>` via `i64::try_from` + `from_timestamp`. |
| `ResumeStorageDispatchTotality` | Invariant: `StorageRuntimeJournal::storage_event` must be total over `RuntimeJournalEvent` without recourse to a silent rewrite to `RunFailedEvent`. | Required for `vb-edvbj` to delete the fallback arm without breaking dispatch. |
| `SingleCloneDispatch` | Invariant: `storage_event` performs exactly one full-event clone per dispatch, enforced by `STORAGE_EVENT_CLONE_COUNT`. | The `Resumed` arm must preserve this invariant. |
| `LifecycleState::Active` | Recovery/replay classification of `JournalEvent::RunResumed` at `crates/vb_storage/src/journal/incident.rs:203`. | The user-visible contract: a successfully resumed run is `Active`, not `Failed`. |
| `RecoveryRunResumed` | The replay/recovery classifier arm at `crates/vb_storage/src/recovery/hydrate.rs:754` and `crates/vb_storage/src/recovery/replay/observation/normalize.rs:60,126`. | Already classifies `RunResumed` correctly; the storage mapper must produce this variant. |
| `ChronoTimestampConversion` | The typed `u64 → DateTime<Utc>` conversion: `i64::try_from(u64) → i64`, then `DateTime::<Utc>::from_timestamp(secs, 0)`. | Must be explicit, total, and produce a typed runtime error on overflow. |
| `ResumeDispatchError` | New typed `RuntimeError` variant for the conversion failure (e.g. `ResumeTimestampOverflow`). | Surfaces the only realistic failure mode for this conversion (currently unreachable for legal UNIX timestamps but required for hostile-input/long-running-system safety). |

## Aggregates and Entities

### ResumedEvent Aggregate

- Identity: pair `(RunId, EventSeq)`.
- Pre-storage: `RuntimeJournalEvent::Resumed { run: RunId, timestamp: u64 }` carrying the monotonic timestamp from `current_timestamp()`.
- Post-storage: `JournalEvent::RunResumed { run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }` carrying the shard-owned per-run `EventSeq` and a chrono timestamp.

### RuntimeJournalEvent → JournalEvent Dispatcher

- Site: `StorageRuntimeJournal::storage_event` in `crates/vb_runtime/src/journal/chunk_002.rs:270-303`.
- Shape: dispatch on `&event` with `clone_for_dispatch` so the full event is cloned at most once before the matched arm consumes it.
- Three per-family helpers today (`run_storage_event`, `action_storage_event`, `boundary_storage_event`) return `Option<JournalEvent>` or `RuntimeResult<Option<JournalEvent>>`. None of them currently maps `Resumed`.

### Shard Aggregate (referenced, unchanged)

- `Shard::handle_resume` (lines 307-331) and `Shard::append_resumed_event` (lines 344-358) are the public-facing producers of `RuntimeJournalEvent::Resumed`. No change required in this bead.

## Value Objects

| Value object | Invariant |
|---|---|
| `RuntimeJournalEvent::Resumed { run, timestamp }` | `run: RunId`; `timestamp: u64` seconds since UNIX epoch. Construction lives in shard lifecycle. |
| `JournalEvent::RunResumed { run, seq, timestamp }` | `run: RunId`; `seq: EventSeq` (owned by the shard); `timestamp: DateTime<Utc>`. `record_kind() == RecordKind::RunResumed`. |
| `ResumeTimestamp::i64Seconds` | Internal proof concept: the result of `i64::try_from(timestamp)` when `timestamp <= i64::MAX`. Unrepresentable when overflow occurs. |
| `ResumeTimestamp::overflow` | Internal proof concept: `timestamp > i64::MAX`. The mapper must convert this into a typed `ResumeDispatchError`, not silently clamp or wrap. |
| `ResumedStorageDecision` | Closed set: `Some(RunResumed { .. })` (dispatch produced the target) or `Err(ResumeTimestampOverflow)` (hostile-input-safe failure). |
| `BoundaryDispatchFamily` | Closed set of variants the boundary dispatcher recognizes: `WaitScheduled`, `WaitResolved`, `AskScheduled`, `AskAnswered`, `AskTimedOut`, `SlotWritten`, `Resumed`. |
| `StorageEventCloneCount` | Atomic counter incremented by `clone_for_dispatch` in test builds. Captures the single-clone invariant. |

## Policies

1. **Resumed → RunResumed policy:** every `RuntimeJournalEvent::Resumed` received by `StorageRuntimeJournal::storage_event` MUST be mapped to exactly one `JournalEvent::RunResumed` carrying the shard-owned `EventSeq` and a chrono conversion of the `u64` timestamp.
2. **No silent rewrite policy:** `Resumed` must NEVER be rewritten as `JournalEvent::RunFailedEvent`. The fallback `Ok(JournalEvent::RunFailedEvent { … })` at `chunk_002.rs:298-302` is a bug, not a feature.
3. **Totality policy (paired with vb-edvbj):** after this fix, `storage_event` MUST be total over `RuntimeJournalEvent`. Once `vb-edvbj` deletes the catch-all `RunFailedEvent` fallback, no `RuntimeJournalEvent` variant may fall through to an unwritten arm.
4. **Single-clone policy:** the new arm MUST NOT introduce a second full-event clone. `clone_for_dispatch` continues to fire exactly once per `storage_event` call.
5. **Explicit timestamp conversion policy:** the `u64 → DateTime<Utc>` conversion MUST be performed via `i64::try_from` + `DateTime::<Utc>::from_timestamp`, returning a typed `RuntimeError` variant on overflow rather than panicking, clamping, or wrapping.
6. **Dispatch-family policy:** `Resumed` MUST live in `boundary_storage_event` (or a dedicated arm inside `storage_event` after dispatch) and MUST NOT be reclassified as a run-lifecycle or action-lifecycle variant.
7. **Recovery-state policy:** storage of `JournalEvent::RunResumed` MUST cause `incident.rs::lifecycle_state` to return `LifecycleState::Active`. The current bug producing `LifecycleState::Failed` is the symptom that the fix removes.

## Domain Decisions

- The new `Resumed` arm lives in `boundary_storage_event` (option (a) in the codebase map's open question), returning `Ok(Some(JournalEvent::RunResumed { run, seq, timestamp }))`. This mirrors the existing `WaitScheduled → WaitScheduledEvent` shape and avoids touching the seq-discarding catch-alls in `run_storage_event` and `action_storage_event`.
- `seq` for `RunResumed` is the per-run `EventSeq` passed into `storage_event` by the shard. The mapper never invents or increments it.
- The `u64` seconds-since-epoch field on `RuntimeJournalEvent::Resumed` is converted at the storage boundary. The runtime event signature stays `u64`; downstream storage `DateTime<Utc>` is the only `chrono` exposure.
- A new typed error variant `RuntimeError::ResumeTimestampOverflow { run, timestamp }` is required so that the conversion failure surfaces explicitly and the function stays total. The error must be constructed at the conversion site (`i64::try_from(timestamp).map_err(...)`) and must not panic.
- The fix MUST keep `storage_event` total without the catch-all. vb-edvbj's deletion of the `RunFailedEvent` fallback is the structural follow-up that this fix relies on; the runtime test must cover the dispatch without the fallback.
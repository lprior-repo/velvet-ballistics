# Boundary Map: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Pure Core Boundary

Conceptual pure decisions:

- Decide whether a `u64` seconds-since-epoch value fits in `i64`.
- Decide whether `DateTime::<Utc>::from_timestamp(secs, 0)` is `Some(_)` for a legal `secs`.
- Decide the `JournalEvent::RunResumed { run, seq, timestamp }` shape from `RuntimeJournalEvent::Resumed { run, timestamp_u64 }` and `seq`.
- Decide the boundary-dispatcher family membership for `Resumed`.

These decisions should be expressible without I/O, Fjall, time, or runtime mutation. The conversion function `convert_resume_timestamp(timestamp_u64, run) -> Result<DateTime<Utc>, RuntimeError::ResumeTimestampOverflow>` is the pure core.

## Imperative Runtime Boundary

Files/symbols from State 2 map (the surface that changes for this bead):

- `crates/vb_runtime/src/journal/chunk_002.rs`
  - `StorageRuntimeJournal::storage_event` (lines 270–303) — top-level dispatcher.
  - `StorageRuntimeJournal::boundary_storage_event` (lines 193–268) — where the new `Resumed` arm lives.
  - `clone_for_dispatch` (lines 318–324) — single-clone helper, unchanged.

Files/symbols NOT changed by this bead (but referenced):

- `crates/vb_runtime/src/journal/chunk_001.rs`
  - `RuntimeJournalEvent::Resumed { run, timestamp }` (lines 188–194) — source shape, unchanged.
  - `RuntimeJournalEvent::run_id()` (lines 200–224) — already covers `Resumed`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
  - `Shard::handle_resume` (lines 307–331), `Shard::append_resumed_event` (lines 344–358) — producers, unchanged.
- `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs`
  - `current_timestamp()` (lines 24–28) — `u64` seconds source, unchanged.

Mutation responsibilities added by this fix:

- Convert `u64` → `DateTime<Utc>` inside the new `boundary_storage_event` arm.
- Return `Err(RuntimeError::ResumeTimestampOverflow { run, timestamp })` on overflow.
- Preserve the existing single-clone invariant (one `clone_for_dispatch` per `storage_event` call).

## Storage Boundary

Files/symbols:

- `crates/vb_storage/src/events.rs`
  - `JournalEvent::RunResumed { run, seq, timestamp }` (lines 289–297) — already exists; the mapper only references it.
  - `JournalEvent::record_kind()` (lines 401–429, line 424) — `RunResumed` → `RecordKind::RunResumed`.
  - `JournalEvent::run_id()` (lines 336–363, line 358) — already lists `RunResumed`.
  - `JournalEvent::seq()` (lines 369–397, line 392) — already lists `RunResumed`.
- `crates/vb_storage/src/journal/incident.rs:203`
  - `lifecycle_state(JournalEvent::RunResumed) == LifecycleState::Active` — the recovery/replay classifier that the user-visible symptom (Failed) was bypassing.
- `crates/vb_storage/src/recovery/hydrate.rs:754`
  - `is_in_flight_or_completed(JournalEvent::RunResumed) == Ok(false)`.
- `crates/vb_storage/src/recovery/replay/observation/normalize.rs:60,126`
  - `RunResumed` is in the recovery observation classifier.
- `crates/vb_storage/src/recovery/replay/summary/apply.rs:79–81`
  - `RunResumed` is a lifecycle event without per-event sequence history concern.

Boundary contracts:

- The mapper must produce exactly the shape `{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }`.
- The mapper must NOT introduce a new `JournalEvent` variant — `RunResumed` already exists in `vb_storage`.
- The mapper must NOT introduce a new `RecordKind` — `RecordKind::RunResumed` already exists.
- Encode/decode of `RunResumed` is unchanged. Existing `journal_event_tests.rs` and `regression_tests_vb_1rqz7.rs` continue to pass.

## Time Boundary

- `current_timestamp()` (shard/lifecycle/chunk_003.rs:24–28) returns `u64` seconds since UNIX epoch.
- The conversion to `DateTime<Utc>` happens at the storage boundary inside the mapper.
- The runtime event signature (`u64`) is preserved. The storage event signature (`DateTime<Utc>`) is the only `chrono` exposure introduced by the conversion.
- A future improvement could carry `DateTime<Utc>` in the runtime event directly, but that would change shard types and is explicitly out of scope for this bead.

## Storage Backend (Fjall) Boundary

- The durable append happens via `FjallJournal::append_journaled` or `append_strict` (chosen by `DurabilityProfile`).
- The mapper arm does not touch Fjall directly. The append is performed by `append_storage_event(&JournalEvent::RunResumed { .. })`.
- The mapper arm does not change the durability profile or the queueing behavior.
- The same append path serves `StorageRuntimeJournal` and `QueuedStorageRuntimeJournal` (via `chunk_003.rs:12`).

## Conversion Boundary

The typed conversion:

```text
RuntimeJournalEvent::Resumed.timestamp : u64
   |
   |  i64::try_from(timestamp_u64) -> i64_secs
   |  -> Err if timestamp_u64 > i64::MAX
   |
   |  DateTime::<Utc>::from_timestamp(i64_secs, 0) -> DateTime<Utc>
   |  -> returns None only for far-future i64 values;
   |     for all realistic UNIX timestamps (current era < 2^31)
   |     it is always Some(_)
   |
   v
JournalEvent::RunResumed.timestamp : DateTime<Utc>
```

Two-step conversion is mandatory. Skipping `i64::try_from` (e.g. using `as i64` cast) would either panic on overflow or wrap silently. Skipping the `from_timestamp` call (e.g. constructing `DateTime::<Utc>::from_naive_utc_and_offset`) would lose the chrono type-level guarantee.

## Forbidden Boundary Crossings

- Runtime core must not parse YAML, JSON, or HTTP for this feature.
- Storage adapter must not silently accept a `Resumed` event and rewrite it.
- Mapper must not panic on any `u64` input.
- Mapper must not invoke `SystemTime::now()` or any clock — the timestamp is supplied by the caller (`current_timestamp()` at the shard).
- Mapper must not hold locks, perform I/O, or mutate global state.
- Mapper must not introduce a second full-event clone.
- Mapper must not silently fall through to the catch-all `RunFailedEvent` fallback once vb-edvbj deletes it.
- The new `RuntimeError::ResumeTimestampOverflow` variant must not be a unit variant — it must carry `run` and `timestamp` for diagnostics.

## Dependency Surface (cargo, all unchanged)

- `crates/vb_runtime/Cargo.toml:9` — `chrono.workspace = true` (already a direct dependency).
- `crates/vb_storage/src/events.rs:5` — `use chrono::{DateTime, Utc};` (already imported).
- No new crate dependency is required. No workspace `Cargo.toml` change is required.

## Coupled Bead Surface (vb-edvbj)

- `vb-edvbj` deletes the `Ok(JournalEvent::RunFailedEvent { .. })` catch-all at `chunk_002.rs:298–302`.
- After both beads land, `storage_event` has no silent fallback; every variant reaches an explicit arm.
- The compile-time exhaustive match requirement is the structural enforcement that keeps the dispatch total.
- The beads are STRONG-coupled for release; both must land together.
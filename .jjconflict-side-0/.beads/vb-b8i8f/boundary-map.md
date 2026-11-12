# Boundary Map: vb-b8i8f Cancel/Kill Lattice Recovery

## Pure Core Boundary

Conceptual pure decisions:

- Classify lifecycle state as `Missing`, `Live`, or `Terminal(kind)`.
- Decide `Cancel`/`Kill` transition result.
- Decide whether stale authority is valid.
- Classify record kind `28` as known journal kind.

These decisions should be expressible without I/O, clocks, queues, Fjall, or runtime mutation.

## Imperative Runtime Boundary

Files/symbols from State 2 map:

- `crates/vb_runtime/src/runtime.rs`
  - `Runtime::cancel_run`
  - required `Runtime::kill_run`
  - `Runtime::snapshot_run`
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
  - `Shard::handle_cancel`
  - `Shard::handle_kill`
  - `Shard::take_run_state`
- `crates/vb_runtime/src/shard/types.rs`
  - `ShardCommand::Cancel`
  - `ShardCommand::Kill`
  - `PendingTimer`

Mutation responsibilities:

- live run removal
- frame release
- pending timer removal
- terminal marker insertion
- trace/counter updates
- command queue processing

## Storage Boundary

Files/symbols:

- `crates/vb_runtime/src/journal/chunk_002.rs`
  - `StorageRuntimeJournal::run_storage_event`
  - `StorageRuntimeJournal::append_storage_event`
- `crates/vb_storage/src/records.rs`
  - `RecordKind::RunKilled`
- `crates/vb_storage/src/events.rs`
  - `JournalEvent::RunKilled`
- `crates/vb_storage/src/codec/validation.rs`
  - `is_known_record_kind`
  - `validate_known_kind`
  - `validate_kind_family`
- `crates/vb_storage/src/journal/internal.rs`
  - append path encodes event record kind
- `crates/vb_storage/src/journal/replay.rs`
  - decode/replay path for run events

Boundary contracts:

- Runtime terminal event mapping must not emit a storage event that the storage codec rejects.
- Encode and decode must agree that `RunKilled=28` belongs to `MAGIC_JOURNAL_EVENT`.
- Replay must preserve contiguous per-run event sequence semantics.

## Time Boundary

- Timer generation/deadline/kind authority is external to pure lifecycle decision.
- Cancel/kill invalidates pending timer authority by removing the timer record.
- Later timer fires are stale and must not be treated as live wait/ask resumption.

## External Action / Ask Boundary

- Action tickets and ask tickets are authority tokens from the live run epoch.
- Cancel/kill closes that epoch.
- Completion/failure/answer after terminalization is stale authority and must not mutate slots or append completion events.

## Observability Boundary

- Trace ring may record successful terminalization once.
- Counters may increment once for terminal cancel/kill.
- Snapshot/inspect must be read-only and must not be a backdoor to mutate or resurrect terminal state.

## Forbidden Boundary Crossings

- Runtime core must not parse YAML/JSON or use HTTP for this feature.
- Storage codec must not silently accept unknown future kinds outside family validation.
- Runtime must not hide failed terminal event persistence by returning successful cancel/kill.

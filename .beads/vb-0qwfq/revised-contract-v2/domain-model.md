# Domain Model — Run Sequence Number Allocation

## Entities

| Entity | Description | Ownership |
|--------|-------------|-----------|
| `EventSeq` | Monotonic u64 per-run event sequence. Starts at 0 for the first accepted event in a run. | Runtime (per-shard) |
| `RunHeader` | Persistent run identity, compiled digest, policy pin. Created at submission. | Storage (`run_header` keyspace) |
| `RunInputs` | Retained run input payload. Bound to run at submission. | Storage (`run_inputs` keyspace) |
| `CompiledIrRecord` | Accepted executable artifact. Stored at submission or pre-admission. | Storage (`compiled_ir` keyspace) |
| `JournalEvent` | Individual lifecycle event with `(run_id, seq)` key. | Storage (`run_event` keyspace) |
| `JournalWriteBatch` | Atomic cross-keyspace write batch. Commits with SyncAll for strict durability. | Fjall (`OwnedWriteBatch`) |
| `write_lock` | Poison-detection-only mutex. Not a serialization guarantee for writers. | Storage (`FjallJournal`) |

## Relationships

- RunHeader + RunInputs + CompiledIrRecord form the **prepared envelope**, staged together in one batch.
- Events are appended separately after the envelope is committed (or as part of the same envelope commit for the acceptance event).
- EventSeq is allocated by the caller (shard) and passed to storage. Storage validates uniqueness via key check.
- Durability profile is a runtime property of the journal writer, not an event property.

## Lifecycle

1. **Prepare**: Caller assembles `RunSubmission` with source, artifact, inputs, header.
2. **Allocate Seq**: Caller reserves EventSeq for each event it will write.
3. **Stage Envelope**: Prepare envelope in `JournalWriteBatch` (source, artifact, inputs, header).
4. **Stage Events**: Append planned events to same or subsequent batch with allocated seq.
5. **Commit**: Call `batch.commit()` (journaled) or `batch.strict().commit()` (SyncAll).
6. **Advance**: Increment allocated sequence for next event.

## Key Invariant

EventSeq is **per-run**, not global. Two runs can each have event at seq 0. Ordering is only meaningful within a single run's event log.
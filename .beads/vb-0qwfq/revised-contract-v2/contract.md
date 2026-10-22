# Contract — Run Sequence Number Allocation and Durability

## Scope

This contract governs the allocation, persistence, and durability of per-run event sequence numbers in the Velvet Ballistics storage layer. It covers the Fjall-backed journal, atomic cross-keyspace write batches, and the interaction between runtime admission and storage.

## Core Invariant

**EventSeq is per-run, monotonic, and allocated by the caller.** The storage layer validates uniqueness via key collision detection but does not issue or manage sequence numbers. The first event in every run is `EventSeq::new(0)`.

## Durability Profile Contract

| Profile | Operation | Durability Guarantee |
|---------|-----------|----------------------|
| Volatile | `append()` | In-memory only; lost on process exit. |
| Journaled | `append_journaled()` / queue | Persisted to WAL, visible immediately, may be lost on power loss before flush. |
| Strict | `append_strict()` | `SyncAll` (fsync) before return; survives power loss. |
| Strict Batch | `append_strict_batch()` | All events durable atomically with single `SyncAll`. |
| Submission | `commit_run_submission()` | Envelope and events durable atomically with policy-matched profile. |

**Key distinction**: `Batch.commit()` alone is NOT equivalent to `SyncAll`. A plain `commit()` writes to Fjall's WAL/memtable and returns immediately. Durability requires `batch.strict().commit()` which sets `PersistMode::SyncAll` before committing.

## Sequence Allocation and Validation Flow

```
Caller prepares RunSubmission with envelope and planned events.
Caller pre-flights validation (workflow, policy, contracts, capability binding).
If validation fails: return error; no durable state written.
Caller allocates EventSeq for each planned event, starting at 0 for new run.
Caller stages envelope + events in JournalWriteBatch.
Caller calls batch.strict().commit() (strict) or batch.commit() (journaled).
On success: return Ok(allocated_next_seq).
On failure: caller retries with same seq values (idempotent).
```

**Validation before mutation** is mandatory: no durable counter or event is written until the submission is known to be acceptable. A rejected run must not leave any trace in the journal.

## Durability and Failure Semantics

### Success Case
- `append_strict()` returns `Ok(())` only after the event is staged AND committed with `SyncAll`.
- `append_strict_batch()` returns `Ok(())` only after all events are staged AND committed with single `SyncAll`.
- Envelope + events in `commit_run_submission()` are durable atomically (single batch).

### Failure Case
- If batch staging fails (invalid event, duplicate key, overflow): batch is aborted, no durable state written. Caller gets `JournalError` with specific cause.
- If `commit()` fails after staging: Fjall `OwnedWriteBatch` rolls back; no durable state written. Caller gets `JournalError::Fjall(...)`.
- **No rollback on arbitrary I/O errors**: If `commit()` itself fails (e.g., disk write error), the batch is rolled back. But if the batch commits and then a later independent write fails, there is no rollback mechanism for already-committed batches. This is the Fjall batch boundary.

### Ambiguity Window
There is no ambiguous "half-durable" state. The Fjall `OwnedWriteBatch` provides atomic visibility: either the entire batch is visible or none of it is. This eliminates the previous hazard where an event was visible in the memtable but not yet durable.

## Concurrency and Locking

- `write_lock` is a poison-detection-only mutex; it does NOT serialize writers.
- Fjall provides internal synchronization for its WAL and memtable operations.
- Cross-thread concurrent appends are safe (Fjall handles locking internally).
- Single-process constraint enforced by `ProcessLock` (file lock).
- Sequence allocation must be caller-serialized (shard-local state).

## Sequence Overflow

- `EventSeq` is `u64`; overflow only possible at `u64::MAX`.
- Caller must use `checked_add` when incrementing.
- On overflow: caller reports `JournalError::SequenceOverflow` before attempting to write.
- No wrapping; `checked_add` returns `None` on overflow.

## Freshness Floor

- `AcceptedArtifact.accepted_at_seq` records the sequence at which the artifact was accepted.
- Runtime admission with `required_at_least` rejects artifacts accepted before that sequence.
- Legacy envelopes with `accepted_at_seq == 0` (pre-floor feature) are accepted by default.
- Freshness floor is a read-time policy check, not a write-time constraint.

## Digest vs Authorization

- Digests (BLAKE3) provide **integrity**: content matches claimed digest.
- Digests do NOT provide **authorization**: attacker can change content and rehash.
- The `accepted_at_seq` freshness floor provides **trust boundary**: only artifacts accepted at or after the floor are trusted.
- Runtime admission validates: envelope integrity + digest binding + freshness floor + capability binding.

## API Compatibility

The contract preserves the following existing public APIs:
- `FjallJournal::open(path, config)`
- `FjallJournal::append_journaled(event)`
- `FjallJournal::append_strict(event)`
- `FjallJournal::append_strict_batch(events)`
- `FjallJournal::persist_strict()`
- `FjallJournal::batch()`
- `JournalWriteBatch::strict()`
- `JournalWriteBatch::commit()`
- `FjallJournal::commit_run_submission(submission, policy)`
- `admission::admit_artifact_run_with_certificate_floor(...)`
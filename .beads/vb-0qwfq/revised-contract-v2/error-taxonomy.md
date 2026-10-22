# Error Taxonomy — Run Sequence Number Allocation

## Sequence Errors

| Error | Code | Trigger |
|-------|------|---------|
| `JournalError::SequenceOverflow` | 0x4002 | Caller supplies `EventSeq::new(u64::MAX)` and attempts to increment. |
| `JournalError::DuplicateEvent { run, seq }` | 0x4001 | Batch contains two events with same `(run, seq)`, or event key already exists in journal. |
| `JournalError::DuplicateStagedKey` | 0x4006 | Same-batch duplicate detected via `staged_event_keys` HashSet. |

## Durability Errors

| Error | Code | Trigger |
|-------|------|---------|
| `JournalError::StrictDurabilityFailed` | 0x4003 | Legacy `persist_strict()` path fails. NOT reachable via batched `append_strict`. |
| `JournalError::Fjall(fjall::Error)` | 0x4004 | Underlying Fjall operation fails (including `OwnedWriteBatch::commit()`). |
| `JournalError::BatchAborted` | 0x4007 | `commit()` called after prior stage step set `aborted = true`. |

## Validation Errors (pre-mutation)

| Error | Code | Trigger |
|-------|------|---------|
| `JournalError::InvalidEvent` | 0x4008 | Event fails `is_valid()` (bad kind, overflowed seq, etc.). |
| `JournalError::RunAlreadyExists` | 0x4009 | Run header, inputs, or events already present for this run. |
| `JournalError::QueueFull` | 0x400A | Batch exceeds `MAX_BATCH_COUNT` operations. |
| `JournalError::PayloadTooLarge` | 0x400B | Encoded record exceeds `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`. |
| `JournalError::JournalBatchBytesExceeded` | 0x400C | Batch exceeds byte limit. |

## Concurrency Errors

| Error | Code | Trigger |
|-------|------|---------|
| `JournalError::WriteLockPoisoned` | 0x4005 | Underlying `write_lock` mutex is poisoned. |
| `JournalError::ProcessLockHeld` | 0x400E | Process lock file already held (journal already open elsewhere). |

## Admission Errors (runtime layer)

| Error | Code | Trigger |
|-------|------|---------|
| `AdmissionError::ArtifactCertificateStale` | runtime | `artifact.accepted_at_seq < required_at_least`. |
| `AdmissionError::ArtifactDigestMismatch` | runtime | Loaded artifact digest differs from requested. |
| `AdmissionError::CapabilityCountMismatch` | runtime | Cardinality-exact capability check fails. |
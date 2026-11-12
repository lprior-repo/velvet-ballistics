# Domain Model: vb-vzcuf Fresh Journal Batch Byte Accounting

## Scope

Build the Rust domain/type contract for replacing the missing storage-layer accumulated journal batch-byte accounting seam. This contract covers `crates/vb_storage` admission of journal events into `JournalWriteBatch`; it does not implement production Rust, tests, or verifier artifacts.

## Ubiquitous Language

- **Journal event append**: staging one encoded `JournalEvent` into the `run_event` keyspace through `JournalWriteBatch::append_event`.
- **Encoded journal event bytes**: the full byte length of the value returned by `encode_record(MAGIC_JOURNAL_EVENT, ..., event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)`, including the storage envelope header and payload bytes.
- **Per-record payload cap**: `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`; enforced during `encode_record`; failure is `JournalError::PayloadTooLarge { len, max }`.
- **Accumulated journal batch byte budget**: a non-zero maximum total of encoded journal event bytes allowed in one `JournalWriteBatch` for journal events only.
- **Staged journal event bytes**: the accumulated sum of encoded values already accepted by `append_event` in the current non-aborted batch.
- **Attempted journal event bytes**: `staged_journal_event_bytes + encoded_len(candidate_event)` if checked addition succeeds.
- **Accumulated byte rejection**: a storage-visible typed error returned when the attempted journal event bytes exceed the configured accumulated journal batch byte budget, or when addition cannot be represented.
- **Count rejection**: existing `JournalError::QueueFull` when `inner.len() >= MAX_BATCH_COUNT` before staging the candidate.
- **Duplicate durable event rejection**: existing `JournalError::DuplicateEvent { run, seq }` when the durable event key already exists; this aborts the batch.
- **No partial mutation**: failed accumulated-byte admission leaves `inner`, `staged_event_keys`, and `staged_journal_event_bytes` unchanged and does not persist the rejected event on later `commit`.

## Aggregate

`JournalWriteBatch<'j>` is the aggregate boundary for admission state.

Required aggregate state after implementation:

1. Fjall `OwnedWriteBatch` for staged storage operations.
2. Borrowed `FjallJournal` capability.
3. Same-batch journal event key set, if retained for side-index/idempotence contracts.
4. `BatchLifecycle` state: `Open` or `Aborted`.
5. `StagedJournalEventBytes`: non-negative accumulated encoded bytes for accepted journal events in the open batch.
6. `JournalBatchByteLimit`: non-zero maximum for accumulated journal event bytes.

## Entities and Value Objects

| Name | Kind | Contract |
| --- | --- | --- |
| `JournalWriteBatch<'j>` | Aggregate | Owns one mutable admission session; `!Send + !Sync`; commits atomically unless aborted. |
| `JournalBatchByteLimit` | Value object | Non-zero bounded integer; sourced from storage default or explicit API; cannot be absent inside an open batch. |
| `StagedJournalEventBytes` | Value object | Sum of accepted encoded journal event value lengths; initialized to zero; monotonically increases only on successful `append_event`. |
| `EncodedJournalEventBytes` | Value object | Length of encoded journal event value after per-record payload validation; representable in the accumulator type. |
| `AttemptedJournalEventBytes` | Value object | Checked sum of staged plus candidate bytes; never computed with unchecked arithmetic. |
| `BatchAdmissionOutcome` | Domain result | Either `Accepted { new_total }` or typed `Rejected(reason)`. |

## Policies

1. **Accounting domain**: accumulated journal batch-byte accounting counts only encoded journal event values inserted by `append_event` into `run_event`. It must not count run headers, snapshots, blobs, indexes, workflow sources, or compiled IR writes unless a future contract explicitly widens the domain.
2. **Limit source**: every `JournalWriteBatch` must have a `JournalBatchByteLimit`. The preferred API is a storage-visible constructor/factory seam such as `JournalWriteBatch::new_with_limits(journal, JournalBatchLimits)` or `FjallJournal::batch_with_limits(...)`, while preserving `new`/`batch` with a safe default. The default should be aligned with the existing core default of `1_048_576` unless product owners choose a different storage default.
3. **Exact-fit rule**: `attempted_bytes <= limit` is accepted. `attempted_bytes > limit` is rejected.
4. **Arithmetic rule**: staged plus candidate length must use checked addition in a representation large enough for encoded value lengths and the configured limit. Overflow is an accumulated byte admission rejection, not a panic and not `PayloadTooLarge`.
5. **Error separation**: duplicate, count, per-record payload, and accumulated batch-byte failures are distinct observable outcomes.
6. **Mutation rule**: candidate encoding may allocate a temporary value before admission, but permanent batch state mutates only after duplicate, count, per-record payload, and accumulated-byte guards have accepted the event.

## Commands

- `CreateJournalWriteBatch(journal, optional_limits)` -> `JournalWriteBatch<Open>` with zero staged journal event bytes and a non-zero byte limit.
- `AppendJournalEvent(open_batch, event)` -> `Accepted` or a typed `JournalError`.
- `QueryStagedJournalEventBytes(open_batch)` -> current byte total; must return zero for newly constructed or aborted batches if public API follows current `len` convention.
- `Commit(batch)` -> durable write or no-op when aborted.

## Events

- `JournalEventEncoded { key, encoded_len }` after successful per-record encoding.
- `JournalEventAccepted { key, previous_total, encoded_len, new_total }` after accumulated admission and staging.
- `JournalEventRejected { key, reason }` with no permanent mutation.
- `BatchAborted { reason }` only for existing aborting failures such as durable duplicates or digest failures; accumulated byte rejection should not abort unless product explicitly decides otherwise.

## Invariants

- I1: `JournalBatchByteLimit > 0` for every open batch.
- I2: `StagedJournalEventBytes == sum(encoded_len(event_value))` for all successfully accepted `append_event` calls in the batch.
- I3: `StagedJournalEventBytes <= JournalBatchByteLimit` for every open, successfully mutated batch state.
- I4: `append_event` cannot panic on accumulated byte addition.
- I5: `PayloadTooLarge` describes one encoded payload attempt; accumulated byte rejection describes the whole batch budget.
- I6: Count rejection is based on operation count and does not alter byte totals.
- I7: Accumulated byte rejection does not set `aborted`; subsequent smaller valid events may still be admitted if they fit.
- I8: Duplicate durable event rejection preserves existing abort semantics and cannot be masked by byte budget checks.
- I9: Same-batch duplicate/idempotent behavior remains unchanged except that accepted duplicates still contribute to encoded bytes only if they create staged journal event bytes under the chosen side-index semantics; this needs implementation decision if same-key inserts collapse in `OwnedWriteBatch`.

## Open Domain Decisions

1. Exact public constructor/factory shape for supplying the byte limit remains open.
2. Exact `JournalError` variant name and fields remain open, but it must expose at least attempted/actual and limit values.
3. Same-batch duplicate accounting needs a final decision: count every accepted append attempt, or count final distinct durable event keys. The safer persistence budget model is to count staged insert values as they are admitted unless implementation can prove replacement semantics and subtract replaced bytes.

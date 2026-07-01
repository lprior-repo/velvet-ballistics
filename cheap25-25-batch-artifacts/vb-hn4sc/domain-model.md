# Domain Model — vb-hn4sc

bead_id: vb-hn4sc
bead_title: Storage: enforce byte-budget limits in queued group commits (P1 bug)
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:30:00Z
authoring_agent: rust-contract

## Ubiquitous Language

| Term | Definition | Synonyms / Anti-synonyms |
|---|---|---|
| **Group commit** | An atomic, bounded drain of queued `JournalEvent`s into a single Fjall `OwnedWriteBatch` followed by one commit. | Synonym: `flush_batch` (the function name). Anti-synonym: per-event `append_event` on `JournalWriteBatch` — that is the *direct* path. |
| **JournalEvent** | A compact, durable record of a runtime step, addressed by `(RunId, EventSeq)`, encoded via `encode_record`. | Same shape as the direct-path value. |
| **Encoded record** | The bytes produced by `encode_record(MAGIC_JOURNAL_EVENT, kind, seq, event, MAX_PAYLOAD)` — includes the 60-byte `RECORD_HEADER_BYTES` envelope and the postcard-encoded payload. | The byte accounting basis. NOT the raw payload. |
| **Byte budget** | The maximum sum of *encoded record lengths* permitted in a single `OwnedWriteBatch` produced by `flush_batch`. | Synonym: `max_journal_batch_bytes`, `byte_budget`. Anti-synonym: per-event payload cap (`max_journal_event_payload_bytes`). |
| **StorageLimits** | Configuration object holding both per-event and per-batch caps for the storage writer. | Shared between direct (`JournalWriteBatch`) and queued (`JournalWriterQueue`) paths. |
| **Rejection** | A typed `Err(JournalError::JournalBatchBytesExceeded { attempted, limit })` returned from `flush_batch` **before** any `owned_batch.commit()` call, leaving the queue holding the rejected event plus any not-yet-staged events. | NOT a panic. NOT a silent truncation. NOT a partial commit. |
| **Atomic drain** | Property of `flush_batch` per master §49: either *all* events staged in this flush become durable, or none do. | Anti-synonym: partial prefix — explicitly forbidden. |
| **Parity** | The queued byte gate and the direct `JournalWriteBatch::append_event` gate emit the *same* `JournalError::JournalBatchBytesExceeded` variant for the *same* `(attempted, limit)` shape; same accounting basis, same checked_add overflow pattern. | NOT a parallel `QueuedBatchBytesExceeded` variant. |

## Aggregates and Entities

### Aggregate: `JournalWriterQueue` (root)

- **Bounded** by `JournalQueueCapacity` (NonZeroUsize, currently 1..=capacity).
- **Batched** by `JournalBatchSize` (NonZeroUsize, currently 1..=batch_size).
- **Protected** by `byte_budget: u64` derived from `StorageLimits::max_journal_batch_bytes`.
- **State:** `{ pending: VecDeque<QueuedJournalEvent>, shutdown: bool }` under `Mutex`.
- **Commands:** `enqueue_journaled`, `enqueue_strict`, `flush_batch`, `drain_all`, `shutdown`, `probe_accepting_writes`, `pending_profile_counts`.
- **No aggregate-level mutable byte accumulator**: byte accounting is computed *per flush* against a transient accumulator local to `flush_batch`. (Per Open Question 5 in the codebase map: Option A — re-encode during flush, memory-neutral, no queue-state drift. Chosen.)

### Entity: `QueuedJournalEvent`

- Carries `event: JournalEvent`, `profile: DurabilityProfile`.
- No pre-computed byte size stored — size is computed during `stage_queued_event` via the existing `encode_record` call.

### Value Object: `EncodedRecordLength` (newtype, contract-only)

- **Type:** `pub struct EncodedRecordLength(pub u64);` — non-negative byte count.
- **Smart constructor:** `EncodedRecordLength::new(value: u64) -> Result<Self, JournalError>` that rejects `value > MAX_ENCODED_RECORD_BYTES` (where `MAX_ENCODED_RECORD_BYTES = RECORD_HEADER_BYTES + MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1_048_636`).
- **Reason:** the byte accounting basis is the encoded record, not the payload; the newtype prevents accidental payload-basis confusion in the gate.

### Value Object: `AccumulatedFlushBytes` (newtype, contract-only)

- **Type:** `pub struct AccumulatedFlushBytes(pub u64);` — running sum inside `flush_batch`.
- **Operations:** `add(self, EncodedRecordLength) -> Result<Self, JournalError>` via `checked_add`; `would_exceed(self, u64) -> bool`.
- **Reason:** the overflow path is identical to the limit-exceeded path (`JournalBatchBytesExceeded`), but the newtype makes the checked_add explicit at the call site.

### Value Object: `StorageLimits` (extended, not new)

- **Current:** `{ max_journal_event_payload_bytes: u32 }`.
- **Required extension:** add `pub max_journal_batch_bytes: u64` defaulting to `DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER = 1_048_636` (i.e. `RECORD_HEADER_BYTES + DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 60 + 1_048_576`).
- **Default:** `StorageLimits::DEFAULT` MUST set `max_journal_batch_bytes = 1_048_636` (encoded-record basis, inclusive of the 60-byte header) so the default accommodates at least one max-size event — matching the existing `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event` evidence. The existing `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576` (payload basis, used by `JournalWriteBatch`) is preserved unchanged; a new sibling constant `DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER` is added in `crates/vb_storage/src/storage_constants.rs` (or appended to `constants.rs`).

### Entity: `JournalWriteBatch` (sister type — parity target)

- Direct, non-queued path. Already enforces `byte_limit: Option<u64>` per `append_event`.
- The contract target: queued path's `flush_batch` must converge on the *same* byte-basis and *same* error variant.

## Commands

| Command | Inputs | Outputs | Failure mode |
|---|---|---|---|
| `enqueue_journaled(event)` | `JournalEvent` | `Ok(())` | `QueueFull`, `QueueShutdown`, `WriteLockPoisoned` |
| `enqueue_strict(event)` | `JournalEvent` | `Ok(())` | same as above |
| `flush_batch(journal)` | `&FjallJournal` | `Ok(JournalWriterFlushReport { drained, written })` | `JournalBatchBytesExceeded { attempted, limit }` (NEW), `DuplicateStagedKey`, `DuplicateEvent`, `Encode`, `Fjall`, `WriteLockPoisoned` |
| `drain_all(journal)` | `&FjallJournal` | `Ok(JournalWriterFlushReport)` | propagates `flush_batch` errors; first oversize byte batch aborts the drain |
| `shutdown(journal)` | `&FjallJournal` | `Ok(JournalWriterFlushReport)` | propagates `drain_all` errors |

## Events (in-domain, not journal events)

- `GroupCommitCommitted { drained: usize, written: usize, encoded_bytes: u64 }` — implicit terminal outcome; observable via `JournalWriterFlushReport`.
- `GroupCommitRejected { attempted: u64, limit: u64, first_rejected_event_byte_size: u64 }` — implicit terminal outcome of `JournalBatchBytesExceeded`.

## Policies / Invariants

1. **GROUP-COMMIT-BYTE-GATE-1 (atomicity before commit).** The byte budget gate must fire *before* `owned_batch.commit()`. A batch where the *N*-th event would push the accumulator above the limit commits only the first *N-1* events; the *N*-th event plus any remaining queued events stay in the queue, and `Err(JournalBatchBytesExceeded)` is returned.
2. **GROUP-COMMIT-BYTE-GATE-2 (basis parity).** The byte accounting basis is the full encoded record length (`encode_record` output, including the 60-byte header). This matches `JournalWriteBatch::append_event` and the parity test `accounting_uses_full_encoded_length_not_payload_length`.
3. **GROUP-COMMIT-BYTE-GATE-3 (single variant).** No new error variant is introduced. The queued path reuses `JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }` (diagnostic code `0x4022`, symbol `JOURNAL_BATCH_BYTES_EXCEEDED`).
4. **GROUP-COMMIT-BYTE-GATE-4 (overflow contract).** `accumulated_bytes + next_event_bytes` MUST use `u64::checked_add`. On overflow, return `JournalBatchBytesExceeded { attempted: u64::MAX, limit }` — same shape as `JournalWriteBatch::append_event`.
5. **GROUP-COMMIT-BYTE-GATE-5 (enqueue does NOT enforce).** Byte budget is enforced at `flush_batch`, NOT at `enqueue_journaled`/`enqueue_strict`. Callers can enqueue freely; the gate fires at commit time. (Open Question 1 resolved: enforce at flush.)
6. **GROUP-COMMIT-BYTE-GATE-6 (duplicate-key precedence).** The byte gate fires *after* the `staged_keys` `DuplicateStagedKey` guard, so the existing test `flush_batch_rejects_same_batch_duplicate_key` continues to pass with no behavior change.
7. **GROUP-COMMIT-BYTE-GATE-7 (default compatibility).** `StorageLimits::DEFAULT.max_journal_batch_bytes` MUST equal `DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER` (1_048_636) — the encoded-record inclusive-of-header value that guarantees at least one max-size event fits — and `vb_core::max_journal_batch_bytes()` MUST equal the existing payload-basis `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` (1_048_576); the two constants are related by `inclusive_of_header = payload + RECORD_HEADER_BYTES`.
8. **GROUP-COMMIT-BYTE-GATE-8 (lock discipline).** The per-flush byte accumulator is local to `flush_batch` and held under the same `state: Mutex<JournalWriterQueueState>` lock that protects `pending`. No accumulator field on `JournalWriterQueue` itself, no separate lock.
9. **GROUP-COMMIT-BYTE-GATE-9 (single encode per flush).** The per-event byte size is computed once per `stage_queued_event` call inside `flush_batch`; events are NOT re-encoded across `drain_all` iterations. (Open Question 5 resolved: Option A.)

## Forbidden / Illegal States

- **Byte accumulator overflow at the type level**: prevented by `u64::checked_add`; never reach `u64::wrapping_add`.
- **`JournalWriterQueue::new` with `byte_budget == 0`**: permitted by type (u64) but functionally equivalent to "always reject on first event ≥ 1 byte"; the contract accepts this as a legal but degenerate configuration (callers can intentionally use it for tests). The minimum meaningful value is `MAX_ENCODED_RECORD_BYTES = 1_048_636` so at least one max-size event fits.
- **`flush_batch` returning `Ok(report)` while committing fewer events than staged**: forbidden. `drained == written` always holds in `Ok`.
- **Partial commit on byte-budget rejection**: forbidden by Invariant 1 (atomicity).
- **New error variant for the queued path**: forbidden by Invariant 3 (single variant).
- **`StorageLimits::DEFAULT` with `max_journal_batch_bytes != DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER`**: forbidden by Invariant 7; enforced by `const` equality in `StorageLimits::DEFAULT`.
- **Re-encoding the same queued event more than once per flush**: forbidden by Invariant 9.

## Open Domain Questions Carried Forward

These are NOT blockers for the contract; they are flagged for downstream agents.

1. **`RuntimeError` classification of `JournalBatchBytesExceeded`** (carried from codebase map item 7). The contract asserts the typed error is the wire signal; whether `RuntimeError::from(JournalError)` exposes a typed budget-exhaustion variant is a `proof-to-implementation` concern.
2. **`JournalWriterQueueProfileCounts.pending_bytes` observability field** (carried from item 6). Optional. Not required for the byte gate itself. Deferred.
3. **Loom model for the new byte accumulator** (carried from item 8). Out of scope for this bead; the real `JournalWriterQueue` Loom model would belong to a follow-up.
4. **Module wiring (`mod stage;` in `writer.rs`)** (carried from item 9). Verified by the parent and isolated workspace having identical layout. Worth a CI smoke but not a contract concern.
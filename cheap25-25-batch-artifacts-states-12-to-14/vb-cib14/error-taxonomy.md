# Error Taxonomy: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Runtime Errors

| Domain condition | Required classification | Existing candidate | Contract |
|---|---|---|---|
| Resume timestamp overflow at storage boundary | `RuntimeError::ResumeTimestampOverflow { run: RunId, timestamp: u64 }` (new variant on `RuntimeError`). | n/a (new) | Must be returned exactly when `i64::try_from(timestamp_u64)` fails. Must carry both `run` and the original `timestamp` for diagnostics. Must not panic. |
| Existing storage append failure after conversion succeeds | `RuntimeError::StorageJournalAppend { source }` or whatever existing journal-error carrier the project uses. | `RuntimeError::from(journal_error)` | Conversion errors and append errors must not be conflated. Append failure is reported by the journal adapter, not the mapper. |
| Existing `EncodeFailed` from `encoded_slot_taint_extra` | `RuntimeError::EncodeFailed` | existing | Unchanged; the `SlotWritten` path is not touched by this fix. |
| Existing `JournalPoisoned`, `JournalFull`, `UnsupportedOperation` | existing variants | existing | Unchanged. |
| Existing `ResumeError::{RunIdNotFound, NotResumable, IncompleteHydration, JournalAppendFailed, JournalAppendFailedWithSource, StructuredOutputFailed}` | existing variants | existing | Unchanged. The mapper error propagates through `JournalAppendFailedWithSource` exactly the way any other journal-append error does. |

## Storage Errors

| Domain condition | Existing error | Contract |
|---|---|---|
| `JournalEvent::RunResumed` decode failure | Postcard / envelope decode errors | Unchanged. `RunResumed` already exists and decodes successfully. |
| Replay sequence gap | Replay sequence failure | Unchanged. `RunResumed` must follow the per-run `EventSeq` ordering invariant. |
| `JournalEvent::is_valid()` rejection | Validity failure | `RunResumed` passes `is_valid()` when `run != RunId(0)` and `seq != EventSeq::MAX`. The mapper must produce only valid events. |

## Error Semantics

- The fix MUST add `RuntimeError::ResumeTimestampOverflow { run, timestamp }` as a new typed variant. Adding this variant is a public-runtime-surface change but is the only way to surface the conversion failure cleanly without leaking `i64::MAX`-related arithmetic into the mapper.
- The mapper must NEVER return a `JournalEvent` for an overflowed timestamp. Returning `Ok(JournalEvent::RunResumed { timestamp: DateTime::<Utc>::from_timestamp(0, 0) })` (silently clamping) is a corruption bug; returning `Ok(JournalEvent::RunFailedEvent { .. })` (silent rewrite) is the P0 bug this bead fixes.
- Existing semantic for `ResumeError::JournalAppendFailedWithSource` already covers the propagation of any journal-append error after conversion succeeds. The shard-side rollback (`apply(RuntimeEvent::ResumeRollback)`) and recovery-state-preservation behavior remain valid.
- The fix MUST NOT introduce any error variant that swallows the original `u64` timestamp. If the variant is added later as a richer enum (with payload), it must preserve `timestamp: u64` and `run: RunId`.

## Forbidden Error Patterns

- `unwrap()`, `expect()`, `panic!()` anywhere in the mapper or its helpers.
- `as i64` casts that silently truncate on `u64 -> i64` overflow.
- Silent clamp to `i64::MAX` followed by `from_timestamp`.
- Treating the conversion failure as a successful `RunResumed` with `timestamp = UNIX_EPOCH`.
- Treating the conversion failure as a successful `RunFailedEvent`.
- Logging/printing the error and continuing with a synthesized event.
- Returning the conversion failure as a generic `RuntimeError::Internal` or `RuntimeError::Other` — the typed variant is required for the contract.

## Existing-Variant Reuse Decision

- Reuse `RuntimeError::EncodeFailed`? **No** — the failure mode is timestamp arithmetic, not slot encoding. Conflating them would mis-route diagnostics.
- Reuse `RuntimeError::JournalPoisoned`? **No** — same reason: this is a mapper arithmetic failure, not a storage poisoning event.
- Add a new variant? **Yes** — `RuntimeError::ResumeTimestampOverflow { run, timestamp }` is the only classification that captures both the failure cause and the diagnostic context (run id + original u64).
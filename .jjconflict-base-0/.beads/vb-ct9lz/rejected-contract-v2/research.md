# Research: Executor catalog artifact issuance pattern

## What the OLD implementation does

`ExecutorCatalog::submit_artifact` (actual source, packet 01-001.txt):
1. Calls `prepare_artifact` — pure admission, seq=0, no persistence
2. Encodes artifact via `postcard::to_allocvec`
3. Writes to compiled_ir keyspace via `journal.put_compiled_ir(&CompiledIrRecord { ... })`
4. Calls `journal.persist_strict()?` — **unconditional**, regardless of policy
5. Reads back via `journal.compiled_ir(artifact.digest)?`
6. Decodes via `postcard::from_bytes`
7. Verifies via `self.verify_artifact(&restored, policy)?`
8. Compares `artifact != restored` → Corrupt on mismatch

The unconditional `persist_strict` is INTENDED behavior in the old implementation. The v1 contract incorrectly called this "wrong for non-Strict policies."

## What the shared helper does

`issue_next_artifact_sequence` (actual source, packet 05-001/05-101.txt):
1. Acquires write lock
2. Reads counter from `artifact_acceptance` keyspace at key `b"last_issued"` (global u64 BE)
3. Increments (checked_add → SequenceOverflow on u64::MAX)
4. Stamps `accepted_at_seq` on artifact (public u64, internal NonZeroU64 wrapper)
5. Rebinds artifact digest
6. Encodes artifact → CompiledIrRecord → envelope via `crate::codec::encode_record`
7. Builds single `fjall::OwnedWriteBatch` with counter + artifact writes
8. Applies policy durability: Strict → SyncAll, Relaxed/Journaled → none
9. Commits batch atomically

All helper functions are private (`fn` not `pub fn`); only `issue_next_artifact_sequence` is exported via `pub(crate)`.

## Actual error types

**ExecutorError** (packet 02-001.txt): 16 variants including `Storage`, `Io`, `Codec(#[from] postcard::Error)`, `Artifact(#[from] crate::JournalError)`, `PackMismatch`, `CatalogRequired`, `ReceiptConflict`, `Capacity`, `Unauthorized`, `InvalidTicket`, `InvalidInput`, `Corrupt`, `Conformance`, `Timeout`, `ReceiptMissing`.

NOT: Journal, Postcard, NotAccepted, DigestMismatch, NotFound, InvalidContract, PackMissing, UnexpectedAction (invented by v1).

**JournalError**: existing variants include `MalformedKeyspaceRow` (prefix 0, expected_len 8, actual_len), `SequenceOverflow`, `StrictDurabilityFailed`, `PostcardEncodeFailed`, `Fjall`. No fake `JournalError::Corrupt` — the actual malformed counter error is `MalformedKeyspaceRow`.

## Actual storage layout

- Fjall, not RocksDB
- Counter: global u64 big-endian at `artifact_acceptance` keyspace key `b"last_issued"`
- Artifacts: compiled_ir keyspace, keyed by digest
- `EventSeq(u64)` is public type; `IssuedArtifactSequence(NonZeroU64)` is private internal wrapper

## What changed vs v1 contract

1. Corrected ExecutorError to actual 16-variant enum from source
2. Corrected error flow: new issuance errors go through `Artifact(JournalError)` wrapper, no new enum
3. Corrected `persist_strict` claim: unconditional persist is INTENDED, not a bug
4. Removed invented JournalError variants (Corrupt, etc.)
5. Corrected accepted_at_seq type: public u64, internal NonZeroU64 wrapper
6. Corrected counter storage: global u64 BE, not per-keyspace
7. Corrected Fjall vs RocksDB
8. Clarified test scope: can be 2-3 small files, not just one bounded module
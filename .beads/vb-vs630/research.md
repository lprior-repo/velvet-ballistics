# Bead vb-vs630 Research: CLI Submission Artifact Sequence Stamp

## Domain Context

The artifact acceptance sequence counter provides total ordering across accepted artifacts for replay, audit, and idempotency. Stored in `artifact_acceptance` keyspace at key `b"last_issued"` as big-endian u64. Counter read at 0 means no artifacts issued yet.

## Two Submission Paths

### Default Path (admission.rs)
`submit_artifact` → `submit_artifact_with_contracts` → `prepare_compiled_artifact` → `issue_next_artifact_sequence` (artifact_acceptance.rs) → `verify_artifact_persisted`. The issuance helper acquires write_lock, reads counter, increments, stamps seq, rebinds digests, encodes, builds raw `fjall::OwnedWriteBatch` with counter+artifact writes, commits with policy durability.

### CLI Path (inputs/submission.rs)
`FjallJournal::commit_run_submission` → `submission.validate_policy` (before lock) → acquire write_lock → duplicate run check (header/inputs/events) → `submission.commit(journal, policy)`. `RunSubmission::commit` takes `&self`, creates `JournalWriteBatch`, stages source/artifact/inputs/header/indexes/events via typed put methods, commits. Does not touch the artifact acceptance counter.

## Locking Analysis

Both paths acquire `journal.write_lock`. `issue_next_artifact_sequence` acquires its own lock internally. Calling it from within `commit_run_submission` (which already holds the lock) would deadlock. Solution: factor out the lock-free prepare logic (read counter, increment, stamp, rebind) into a separate helper that callers invoke under their own lock.

## Batching Constraints

`JournalWriteBatch` (batch/types.rs) wraps `fjall::OwnedWriteBatch` in a `pub(super) inner` field. The `inputs` module cannot access `inner` directly. The default path uses raw `fjall::OwnedWriteBatch` operations (`journal.database.batch()`, `batch.insert`). The CLI path uses `JournalWriteBatch` typed methods. To include the counter write in the CLI batch, a crate-private staging method on `JournalWriteBatch` is needed that delegates to `inner.insert` for the `artifact_acceptance` keyspace.

## Borrowing and Mutation Constraints

`commit_run_submission` takes `&RunSubmission`. `RunSubmission::commit` takes `&self`. To stamp `accepted_at_seq` and rebind digests, the artifact must be mutated. Options:
- Owned checked clone of `RunSubmission` under lock, mutate clone, commit clone
- Bounded rebind scratch that takes ownership of a copy of the artifact

The submission aggregate is immutable by design (private fields prevent partial publication). `RunInputsRecord.artifact_digest` is a private field, but `submission.rs` is in the same `inputs` module as `mod.rs`, so it can access private fields of `RunInputsRecord` directly.

## Digest Rebinding

`bind_artifact_digest` sets BOTH `artifact.digest` and `artifact.verification.digest`. After stamping, the submission's `RunInputsRecord.artifact_digest` and `RunAdmission.artifact_digest` must match. The `validate_binding` check (submission.rs:45-58) verifies all four are consistent. The header's `compiled_digest` is the workflow IR digest and is not affected.

## Policy and Events

All three policies (Relaxed, Journaled, Strict) are accepted by CLI submission. `stage_acceptance` (submission.rs:102-112) writes NO events for Relaxed policy. Journaled/Strict write RunAccepted (seq=0) and RunAdmission (seq=1). The counter increment applies to all three policies equally.

## Error Taxonomy

Must use exact actual error types from the codebase:
- Counter read failure: `JournalError::MalformedKeyspaceRow { prefix: 0, expected_len: 8, actual_len: ... }`
- Counter increment overflow: `JournalError::SequenceOverflow`
- Lock acquisition: `JournalError::WriteLockPoisoned`
- Duplicate run: `JournalError::RunAlreadyExists`
- Fjall IO: `JournalError::Fjall(...)`
- Postcard encoding: `JournalError::PostcardEncodeFailed`
- Digest binding: actual errors from `bind_artifact_digest`; `PayloadDigestMismatch` only for binding identity failures

No invented error types. No generic "atomic rollback" claim — Fjall batch commit has IO uncertainty; Err does not guarantee no-commit.

## Precommit vs Postcommit

Precommit errors (validation, overflow, malformed, duplicate) occur before any mutation — safe to retry. Postcommit Fjall IO errors have uncertain ack boundary — counter may have advanced even if Err returned. Known precommit test hooks (injected failures) cause no mutation.

## Design Decision: Factor Out Issuance Prepare

Extract lock-free prepare logic from `issue_artifact`. Both paths call the helper under their own lock. The default path wraps the helper in its existing lock acquisition. The CLI path calls the helper under its existing lock and includes the counter write in its submission batch.

This preserves:
- Existing public API of both paths
- Existing lock behavior of the default path
- Single atomic transaction for CLI path
- No deadlock risk
- Same counter logic for both paths
# SR-014: `put_recovery_stamp` skips the write lock and the durability barrier

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery_stamps.rs:29`
- **Confidence**: confirmed

## Description

`put_recovery_stamp` writes to the `recovery_stamp` keyspace without
acquiring `write_lock` and without calling `persist_strict()`. The write is
therefore (a) racy with concurrent append paths that do hold the lock, and
(b) not durable until the next explicit `persist_strict()` call from the
caller, which the helper does not prompt. A crash between
`put_recovery_stamp` and the next persist silently loses the stamp, causing
the next recovery invocation to re-replay the run.

## Evidence

```rust
pub fn put_recovery_stamp(
    &self,
    run: vb_core::RunId,
    seq: EventSeq,
    stamp: RecoveryStampRecord,
) -> Result<(), JournalError> {
    let key = recovery_stamp_key(run, seq)?;
    let value = encode_record(...)?;
    self.recovery_stamp.insert(key.to_vec(), value)?;
    Ok(())
}
```

Compare `append_unpersisted` (journal/internal.rs:28):
```rust
pub(crate) fn append_unpersisted(&self, event: &JournalEvent) -> Result<(), JournalError> {
    let _guard = self
        .write_lock
        .lock()
        .map_err(|_| JournalError::WriteLockPoisoned)?;
    ...
}
```

The `recover_full_journal` orchestrator (recovery_ops.rs:51-53) calls
`write_recovery_stamp` → `put_recovery_stamp` and then returns immediately
without a `persist_strict`. The durability gap is documented in comments
("skip-replay semantic — using the stamp to short-circuit replay when the
journal tail is unchanged — is delegated to a follow-up bead"), but the
locking inconsistency is not.

## Adversarial Check

Recovery is typically single-process, so the lock race is theoretical.
And because `recover_full_journal` is idempotent (replay produces the same
tracker state given the same events), losing the stamp on crash only causes
redundant work, not corruption. Both points are valid. The reason this is
still worth fixing is the contract asymmetry: every other write path on
`FjallJournal` takes `write_lock`, so `put_recovery_stamp` is a quiet
outlier that a future refactor might copy-paste into a path where
concurrency matters. The missing persist is more concerning: the function
name implies a durable stamp, but the durability is not provided by the
function itself.

## Suggested Fix

Either:

1. Take the write lock and add an optional `persist: bool` parameter, or
   split into `put_recovery_stamp_unpersisted` and
   `put_recovery_stamp_strict` so callers choose explicitly.
2. Document in the docstring that the stamp is not durable until the next
   `persist_strict()` call, so future callers do not assume atomicity.

Option (2) is the minimum viable fix; option (1) is preferable.

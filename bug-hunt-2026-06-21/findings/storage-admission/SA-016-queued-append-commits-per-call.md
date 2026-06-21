# SA-016: `journal.append_queued_indexed_unpersisted` actually commits per call — misleading "unpersisted" name

- **Severity**: Low
- **Category**: simplification
- **Location**: `crates/vb_storage/src/journal/append/journal_impl.rs:65-76` (caller in `crates/vb_storage/src/queue/writer.rs:148, 174`)
- **Confidence**: confirmed

## Description

Despite the name `append_queued_indexed_unpersisted`, the function delegates to `append_indexed_unpersisted` → `append_event_and_index` which calls `batch.commit()` on a fresh per-call Fjall write batch (`crates/vb_storage/src/journal/append/journal_impl.rs:101-105`). Each invocation therefore writes to the LSM memtable and is visible to readers; "unpersisted" only means "not fsynced". The queue's flush loop calls this N times per flush, producing N independent commits where one batched commit would suffice.

## Evidence

```rust
// crates/vb_storage/src/journal/append/journal_impl.rs:65-76
pub(crate) fn append_queued_indexed_unpersisted(
    &self,
    event: &JournalEvent,
) -> Result<(), JournalError> {
    match self.append_indexed_unpersisted(event) {
        Ok(()) => Ok(()),
        Err(JournalError::DuplicateEvent { run, seq }) => {
            self.accept_equal_duplicate(event, run, seq)
        }
        Err(e) => Err(e),
    }
}

// crates/vb_storage/src/journal/append/journal_impl.rs:78-106
fn append_event_and_index(&self, event: &JournalEvent, intent: ActionIndexIntent) -> Result<...> {
    let _guard = self.write_lock.lock()...;
    let event_key = run_event_key(event.run_id(), event.seq())?;
    if self.events.contains_key(event_key)? { return Err(JournalError::DuplicateEvent { ... }); }
    let value = encode_record(...)?;
    let mut batch = self.database.batch();
    batch.insert(&self.events, event_key.to_vec(), value);
    self.stage_action_index_intent(&mut batch, intent)?;
    batch.commit()?;                                       // <-- LSM commit per event
    Ok(())
}
```

## Adversarial Check

The queue layer's purpose is to batch multiple events into a single durable barrier (`persist_strict`) to amortize fsync cost. The per-event commit in `append_queued_indexed_unpersisted` defeats half of this batching: each event still pays an LSM memtable commit (write lock acquisition, memtable insertion, WAL append) even though the fsync is deferred to flush time. A more batch-friendly design would let `JournalWriteBatch::append_event` stage into a single batch that is committed once at flush time. The current architecture uses `JournalWriteBatch` for cross-keyspace atomic writes but does not use it for the queue flush, which is a missed opportunity.

The naming issue is the simpler part of this finding: `unpersisted` should be renamed `unfsynced` or `committed_unfsynced` to reflect the actual semantics. As written, a reviewer reading `append_queued_indexed_unpersisted` would reasonably assume the data is held in memory only — and miss the per-call commit cost.

## Suggested Fix

Two options:

1. **Rename only**: change `append_queued_indexed_unpersisted` to `append_queued_indexed_unfsynced` to match the actual semantics.
2. **Refactor batching**: change the queue flush path to build a single `JournalWriteBatch`, stage all events via `JournalWriteBatch::append_event`, then commit the batch once at the end of `flush_batch`. This eliminates the per-event commit cost and lets the existing batch byte/count limits enforce backpressure uniformly.

# SA-005: `JournalWriterQueue::flush_batch` holds the mutex across Fjall writes, serializing producers

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_storage/src/queue/writer.rs:114-196`
- **Confidence**: confirmed

## Description

`flush_batch` acquires `self.state.lock()` at the top and holds it for the entire duration of journal writes — including `journal.append_queued_indexed_unpersisted` (which commits a per-event Fjall batch, taking the LSM memtable write path) and `journal.persist_strict` (which performs `SyncAll`, i.e. an fsync). Any concurrent `enqueue_*` or `flush_batch` call blocks for the duration of all writes inside the lock.

## Evidence

```rust
// crates/vb_storage/src/queue/writer.rs:114-196
pub fn flush_batch(
    &self,
    journal: &FjallJournal,
) -> Result<JournalWriterFlushReport, JournalError> {
    let mut state = self
        .state
        .lock()
        .map_err(|_| JournalError::WriteLockPoisoned)?;
    ...
    while written < batch_len {
        let Some(item) = state.pending.get(written) else { break; };
        journal.append_queued_indexed_unpersisted(&item.event)?;  // <-- LSM commit under lock
        written = written.saturating_add(1);
    }
    if has_strict {
        journal.persist_strict()?;                                // <-- fsync under lock
        ...
    }
    ...
}
```

Each `append_queued_indexed_unpersisted` call internally calls `batch.commit()` on the Fjall write batch (`crates/vb_storage/src/journal/append/journal_impl.rs:104`), which is the LSM memtable write path. `persist_strict` calls `self.database.persist(fjall::PersistMode::SyncAll)` (`crates/vb_storage/src/journal/append/journal_impl.rs:44`) which is an fsync — typically 1-10 ms on SSD, 10-100 ms on HDD.

## Adversarial Check

The lock is required for correctness: the drain loop mutates `state.pending` via `pop_front`, so concurrent enqueues must be excluded. But the lock does not need to be held during the journal's own writes. The standard producer-consumer pattern for a bounded queue is: under lock, `take()` (or `drain()`) the batch into a local `Vec`; release the lock; write the local vec to the journal outside the lock. The current design instead holds the lock across the slow IO path, making every enqueue wait for every fsync. For `capacity = 1000` and `batch_size = 10` under strict mode, 100 enqueues can wait behind 10 ms × 100 = 1 second of fsync, defeating the queue's purpose as a throughput amplifier.

## Suggested Fix

```rust
pub fn flush_batch(&self, journal: &FjallJournal) -> Result<...> {
    let batch: Vec<QueuedJournalEvent> = {
        let mut state = self.state.lock().map_err(|_| JournalError::WriteLockPoisoned)?;
        // ... compute batch_len, has_strict under lock ...
        let mut batch = Vec::with_capacity(batch_len);
        for _ in 0..batch_len {
            if let Some(item) = state.pending.pop_front() { batch.push(item); }
        }
        batch
    }; // lock released
    // write `batch` to journal without holding `self.state`'s lock
    ...
    // re-acquire lock only if we need to push anything back on failure
}
```

Failure handling becomes slightly more complex (partial failures need to push un-written items back to the front of the deque under a re-acquired lock), but the throughput win is substantial.

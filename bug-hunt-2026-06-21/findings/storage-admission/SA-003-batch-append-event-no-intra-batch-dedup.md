# SA-003: `append_event` duplicate check only inspects committed state, not staged state — duplicates within the same batch are silently allowed

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/batch/write_event.rs:17-58`
- **Confidence**: confirmed

## Description

`append_event` calls `self.journal.events.contains_key(key)?` to detect duplicates, but this lookup hits Fjall's committed LSM memtable only — it cannot see writes that have been staged into the current `OwnedWriteBatch` but not yet committed. Two calls to `append_event` with the same `(run, seq)` in the same batch both pass the duplicate check; both are inserted into the inner batch; the second insert overwrites the first via Fjall's last-write-wins semantics, silently dropping the first event's value.

## Evidence

```rust
// crates/vb_storage/src/batch/write_event.rs:17-58
pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
    let key = run_event_key(event.run_id(), event.seq())?;
    if self.journal.events.contains_key(key)? {       // <-- only sees committed state
        self.state = BatchState::Aborted;
        return Err(JournalError::DuplicateEvent { ... });
    }
    ...
    self.inner.insert(&self.journal.events, key, value);
    Ok(())
}
```

`self.journal.events.contains_key(key)` queries the Fjall partition, which reflects prior commits but not the in-flight `self.inner` batch. The `OwnedWriteBatch::insert` at line 56 will happily accept the same key twice.

The journal's own `append_event_and_index` (`crates/vb_storage/src/journal/append/journal_impl.rs:78-106`) does the same check at commit-per-call time, but the batched path explicitly defers commits to `JournalWriteBatch::commit`.

## Adversarial Check

In the queued-writer path, the upstream `JournalWriterQueue::enqueue_journaled` / `enqueue_strict` (`crates/vb_storage/src/queue/writer.rs:63-87`) does not itself check for duplicates — it just pushes onto the in-memory deque. The flush path then calls `journal.append_queued_indexed_unpersisted` per item, which goes through `append_event_and_index` and DOES detect duplicates against committed state. So the queue path relies on per-item commit, not batch commit, and is safe.

The danger is direct use of `JournalWriteBatch::append_event`: callers that batch multiple events may inadvertently include two events with the same `(run, seq)` (e.g., retry logic that re-adds an event after a transient error, or a bug in event-source deduplication). The duplicate check passes for both, the second overwrites the first in the batch, and one event is silently lost on commit. The bug is a real correctness defect for batch callers even if no current in-tree caller triggers it.

## Suggested Fix

Maintain a `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` (or equivalent) inside `JournalWriteBatch`, populated in `append_event` and checked alongside `self.journal.events.contains_key(key)`. This mirrors the `staged_ir_hashes` pattern already used by the test-only `put_compiled_ir` (`crates/vb_storage/src/batch/write_compiled_ir.rs:27-34`).

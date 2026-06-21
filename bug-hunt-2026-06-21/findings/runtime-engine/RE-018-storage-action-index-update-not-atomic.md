# RE-018: Direct storage journal appends action tickets and index markers non-atomically

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/journal/chunk_002.rs:301-321`
- **Confidence**: confirmed

## Description

`StorageRuntimeJournal::append_sequenced` writes the journal event first and updates the action index afterward. If the event append succeeds but `put_action_index` fails, the method returns an error after leaving a durable action-scheduled event without its recovery index marker.

## Evidence

`crates/vb_runtime/src/journal/chunk_002.rs:301-321`:

```rust
fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
    // Extract action index update before consuming the event
    let action_index_update = if let RuntimeJournalEvent::ActionScheduledTicket {
        ticket,
        ..
    } = &event
    {
        Some((ticket.action, ticket.run, ticket.step))
    } else {
        None
    };

    let storage_event = Self::storage_event(event, seq)?;
    self.append_storage_event(&storage_event)?;

    // Update action index keyspace when scheduling an action
    if let Some((action, run, step)) = action_index_update {
        self.journal.put_action_index(action, run, step)?;
    }

    Ok(())
}
```

The batch implementation in the same file stages both pieces before one commit at `crates/vb_runtime/src/journal/chunk_002.rs:339-349`, which confirms the intended invariant: event and action index marker should commit together.

## Adversarial Check

This is not solved by strict durability. In strict mode, `append_storage_event` calls `append_strict`, so the event may be forced durable before the index write is attempted. A later `put_action_index` failure leaves storage in a state that says an action was scheduled but cannot be found through the action index used for recovery or duplicate detection.

## Suggested Fix

Use the same `JournalWriteBatch` path for single-event `ActionScheduledTicket` appends: stage `append_event` and `put_action_index`, then commit once. Alternatively route every sequenced append through `append_sequenced_batch` with a one-element slice so the atomic path is shared.

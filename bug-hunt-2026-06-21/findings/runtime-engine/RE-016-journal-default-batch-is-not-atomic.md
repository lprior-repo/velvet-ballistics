# RE-016: `RuntimeJournal::append_sequenced_batch` default violates its atomicity contract

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/journal/chunk_001.rs:244-280`
- **Confidence**: confirmed

## Description

The trait contract says batch append is all-or-nothing, but the default implementation loops over `append_sequenced` one event at a time. Implementations that inherit the default can partially append or enqueue a batch before a later event fails.

## Evidence

`crates/vb_runtime/src/journal/chunk_001.rs:244-259` promises atomic visibility:

```rust
/// Appends a slice of lifecycle events atomically with a single durability
/// commit.
...
/// - On success, all events and their per-event index markers are visible
///   in the journal atomically.
/// - On failure, NO events from the batch are visible in the journal.
```

But the default implementation at `crates/vb_runtime/src/journal/chunk_001.rs:275-280` is sequential:

```rust
for (offset, event) in events.iter().enumerate() {
    let offset_u64 = u64::try_from(offset).map_err(|_| RuntimeError::EncodeFailed)?;
    let seq = EventSeq::new(seq_start.get().saturating_add(offset_u64));
    self.append_sequenced(event.clone(), seq)?;
}
Ok(())
```

`VolatileRuntimeJournal` implements only `append` and `probe` at `crates/vb_runtime/src/journal/chunk_001_volatile.rs:61-84`, so it inherits this non-atomic default. `QueuedStorageRuntimeJournal` implements `append_sequenced` and `drain_for_shutdown` at `crates/vb_runtime/src/journal/chunk_003.rs:1-29`, but it also does not override `append_sequenced_batch`.

## Adversarial Check

The comment acknowledges that the default is not atomic, but the method's public contract still says callers can rely on all-or-nothing behavior. Documentation cannot make a non-atomic implementation atomic. Volatile journals can fail mid-batch on capacity. Queued journals can enqueue earlier events before a later enqueue fails, and other producers can interleave between per-event calls.

## Suggested Fix

Remove the atomicity promise from the default method or make the default return `UnsupportedOperation` unless an implementation overrides it. Implement real batch append for volatile and queued journals by reserving capacity/enqueue space up front and committing the whole batch under one lock or queue batch operation.

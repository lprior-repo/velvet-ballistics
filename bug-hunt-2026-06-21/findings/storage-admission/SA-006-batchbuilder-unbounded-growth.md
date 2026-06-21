# SA-006: `BatchBuilder` has unbounded growth — violates the "bounded Vec/HashMap" engineering rule

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/queue/batch.rs:1-37`
- **Confidence**: confirmed

## Description

`BatchBuilder::push` always succeeds and grows the inner `Vec<JournalEvent>` without limit. The repo's `AGENTS.md` engineering rules state: "Bounded resources: every Vec/HashMap growth must hit a configured cap". `BatchBuilder` is the only queue-shaped type in the storage layer that does not enforce a capacity.

## Evidence

```rust
// crates/vb_storage/src/queue/batch.rs:4-37
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
}
impl BatchBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn push(&mut self, event: JournalEvent) {
        self.events.push(event);                              // <-- never fails
    }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn as_slice(&self) -> &[JournalEvent] { &self.events }
}
```

Contrast with `JournalWriterQueue` (`crates/vb_storage/src/queue/writer.rs:36-44`) which validates `JournalQueueCapacity::try_from_usize(capacity)?` and rejects on `capacity == 0` / overflow, and with `JournalWriteBatch::append_event` (`crates/vb_storage/src/batch/write_event.rs:26-28`) which enforces `MAX_BATCH_COUNT = 10_000`.

## Adversarial Check

`BatchBuilder` is exported via `pub use batch::BatchBuilder;` (`crates/vb_storage/src/queue/mod.rs:12`) and reachable through the crate's public API. It is documented as "Ergonomic builder for batching journal events" with no warning about capacity. A caller building events from an unbounded stream (network, file iterator) can push indefinitely; when the resulting slice is later flushed through the queue or a `JournalWriteBatch`, it either hits `MAX_BATCH_COUNT` late (returning `QueueFull` after wasted work) or, worse, drives the process into OOM if the consumer is slower than the producer. The asymmetry with the strictly-bounded `JournalWriterQueue` is itself a usability defect.

## Suggested Fix

Add a fallible constructor and a bounded `try_push`:

```rust
pub struct BatchBuilder {
    events: Vec<JournalEvent>,
    capacity: usize,
}
impl BatchBuilder {
    pub fn with_capacity(capacity: usize) -> Result<Self, JournalError> {
        Ok(Self { events: Vec::with_capacity(capacity), capacity })
    }
    pub fn try_push(&mut self, event: JournalEvent) -> Result<(), JournalError> {
        if self.events.len() >= self.capacity { return Err(JournalError::QueueFull); }
        self.events.push(event);
        Ok(())
    }
}
```

If backward compatibility requires keeping the unbounded `push`, deprecate it in favor of `try_push` and document the bound.

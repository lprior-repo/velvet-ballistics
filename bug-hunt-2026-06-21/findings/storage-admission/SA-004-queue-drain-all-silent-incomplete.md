# SA-004: `JournalWriterQueue::drain_all` can silently return with items still pending under concurrent enqueue

- **Severity**: High
- **Category**: concurrency
- **Location**: `crates/vb_storage/src/queue/writer.rs:202-228`
- **Confidence**: confirmed

## Description

`drain_all` computes a static iteration bound of `capacity / batch_size + 2` and loops that many times, expecting to fully drain the queue. Between iterations, the mutex is released (each `flush_batch` call acquires and releases the lock independently), so concurrent producers can enqueue new items mid-drain. If producers keep up, the loop exhausts `max_iterations` while the queue is non-empty and returns `Ok(total)` with no indication that items remain.

## Evidence

```rust
// crates/vb_storage/src/queue/writer.rs:202-228
pub fn drain_all(
    &self,
    journal: &FjallJournal,
) -> Result<JournalWriterFlushReport, JournalError> {
    let mut total = JournalWriterFlushReport { drained: 0, written: 0 };

    let max_iterations = self
        .capacity
        .checked_div(self.batch_size)
        .ok_or(JournalError::QueueCapacity)?
        .saturating_add(2);
    for _ in 0..max_iterations {
        let report = self.flush_batch(journal)?;
        if report.drained == 0 {
            return Ok(total);
        }
        total.drained = total.drained.saturating_add(report.drained);
        total.written = total.written.saturating_add(report.written);
    }
    Ok(total)                                            // <-- may return with items pending
}
```

`JournalWriterFlushReport` carries only `{ drained, written }` (see `crates/vb_storage/src/types/queue.rs:67-74`) — there is no `pending_after: usize` field. The caller has no way to distinguish "queue fully drained" from "queue still has N items because the static bound was exhausted".

The doc-comment on line 198-201 claims "Maximum iterations: ceil(capacity / batch_size) + 2. This is a static bound - the queue is bounded by construction." The static bound correctly drains a bounded queue in the absence of concurrency, but the queue's `enqueue_journaled` / `enqueue_strict` methods (line 63-70) are explicitly concurrency-safe (`Mutex<JournalWriterQueueState>`), so concurrent enqueue is part of the API contract.

## Adversarial Check

Considered as a sequential API, the static bound is correct: starting from any `pending.len() <= capacity`, iterating `capacity / batch_size + 2` times with each iteration draining `batch_size` items is more than sufficient. The defect only appears under concurrency, which the queue's design explicitly supports. Even modest producer rates (1 enqueue per flush) net to zero drain progress, and a sustained burst can outpace the static bound. The silent `Ok` return is the dangerous part: `shutdown` (line 231-243) calls `drain_all` as its final step, so a concurrent enqueue during shutdown leaves events un-persisted but reported as "drained" to the operator.

## Suggested Fix

Either (a) re-acquire the lock after the loop and re-check `pending.len() == 0`, returning a `DrainIncomplete` indicator (extend `JournalWriterFlushReport` with `pending_after: usize`), or (b) hold the lock across the entire `drain_all` to make the bound truly static (at the cost of blocking concurrent enqueues during drain). Option (a) is preferable: it preserves throughput and gives callers the data they need to retry.

Additionally, `shutdown` should refuse new enqueues (it does — line 77-79 checks `state.shutdown`) before calling `drain_all`, so any drain-incomplete during shutdown indicates a real bug rather than a race.

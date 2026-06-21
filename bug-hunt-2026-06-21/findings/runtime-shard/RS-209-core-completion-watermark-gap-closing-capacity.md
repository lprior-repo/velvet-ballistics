# RS-209-core-completion-watermark-gap-closing-capacity: Pending-capacity check rejects the completion that would drain the queue

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/shard/completion_watermark.rs:131`
- **Confidence**: confirmed

## Description
`CompletionWatermark::complete` checks pending capacity before draining the contiguous prefix. When the pending queue is full of out-of-order completions, the missing gap-closing completion is rejected even though accepting it would immediately drain pending entries and free capacity.

## Evidence
```rust
131:         self.reject_duplicate_or_drained(seq)?;
132:         self.push_pending(seq)?;
133:         let drained = self.drain_prefix();
...
167:     fn push_pending(&mut self, seq: u64) -> Result<(), CompletionWatermarkError> {
168:         if self.pending.len() >= self.max_pending {
169:             return Err(CompletionWatermarkError::QueueFull {
170:                 capacity: self.max_pending,
171:             });
172:         }
173:         self.pending.push(seq);
```

With `boundary = 0`, `max_pending = 1`, and pending `[2]`, `complete(run, 1)` is the only completion that can advance the boundary. It fails at `push_pending` because `pending.len() == max_pending`, so `drain_prefix` is never reached and the watermark remains stuck.

## Adversarial Check
This is not just a throughput limitation. The rejected sequence is not another out-of-order receipt; it is the exact next prefix item. No later completion can unblock the watermark because sequence `1` must be accepted before `2` can drain. A zero-sized pending queue has the same problem for purely in-order completions, because `seq == boundary + 1` still has to pass through `push_pending`.

## Suggested Fix
Handle `seq == boundary + 1` as a prefix completion before applying out-of-order pending capacity, using `checked_add` for the boundary increment. Then drain any already pending successors. Only non-prefix completions should consume `max_pending` capacity.

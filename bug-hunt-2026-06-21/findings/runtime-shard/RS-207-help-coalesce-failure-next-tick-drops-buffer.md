# RS-207-help: A failed coalesce flush is dropped by the next tick

- **Severity**: High
- **Category**: correctness / durability
- **Location**: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:24`
- **Confidence**: confirmed

## Description

When `flush_coalesce_buffer` fails, it returns the error with the buffer still populated. The next call to `tick` sees the coalesce counter at zero, starts a fresh window, and unconditionally clears `coalesce_buffer`, dropping events that were never durably written.

## Evidence

Flush only clears the buffer after a successful journal write:

```rust
// journal_helpers.rs:89-92
self.journal.append_sequenced_batch(&events, first_seq)?;

self.coalesce_buffer.clear();
Ok(())
```

But `tick` clears the buffer whenever a new coalesce window starts:

```rust
// dispatch.rs:24-29
if self.current_coalesce_window_remaining == 0 {
    let window = self.coalesce_window_ticks;
    self.current_coalesce_window_remaining = window.saturating_sub(1);
    self.coalesce_buffer.clear();
}
```

The flush paths at `dispatch.rs:39-40` and `dispatch.rs:63-64` run when the counter reaches zero. If that flush returns `Err`, the counter remains zero. A later retry through `tick` therefore clears the still-unflushed buffer before any retry append can happen.

## Adversarial Check

This is distinct from the already known cross-run sequence issue. Even if every buffered event belonged to one run and sequence assignment were fixed, a transient journal error at flush time leaves the runtime with buffered events that are supposed to be retried. The next tick discards them as a side effect of opening a new window, converting a transient write failure into permanent journal data loss.

## Suggested Fix

Never clear a non-empty coalesce buffer when starting a new window. If `current_coalesce_window_remaining == 0` and the buffer is non-empty, retry `flush_coalesce_buffer` first and return its error without changing the window. Clear the buffer only after a confirmed successful flush.

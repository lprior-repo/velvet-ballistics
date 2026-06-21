# RS-206-help: Shutdown drains clear pending timers without journaling cancellations

- **Severity**: High
- **Category**: correctness / durability
- **Location**: `crates/vb_runtime/src/shard/impl_parts/chunk_002.rs:95`
- **Confidence**: confirmed

## Description

`drain_pending_and_shutdown` and its helper clear `pending_timers` directly, bypassing the cancellation journaling path that records `WaitCancelled` and `AskCancelled`. Runs with pending timers can therefore disappear from memory without durable cancellation evidence.

## Evidence

The file contains a correct journaling helper:

```rust
// chunk_002.rs:71-92
fn cancel_pending_timers_for_shutdown(&mut self) -> RuntimeResult<()> {
    let pending: Vec<(RunId, StepIdx, PendingTimerKind)> = self.pending_timers.iter() ... .collect();
    for (run, step, kind) in pending {
        match kind {
            PendingTimerKind::Wait => self.append_journal_event(RuntimeJournalEvent::WaitCancelled { run, step })?,
            PendingTimerKind::Ask => self.append_journal_event(RuntimeJournalEvent::AskCancelled { run, step })?,
        }
    }
    self.pending_timers.clear();
    Ok(())
}
```

But the alternate shutdown path bypasses it:

```rust
// chunk_002.rs:95-104
pub fn drain_pending_and_shutdown(&mut self) -> RuntimeResult<()> {
    if self.shutting_down {
        self.pending_timers.clear();
        return Ok(());
    }
    self.drain_pending_commands(self.command_queue.len())?;
    self.shutting_down = true;
    self.pending_timers.clear();
    Ok(())
}

// chunk_002.rs:107-115
if !self.tick()? {
    self.pending_timers.clear();
}
```

Both direct clears delete timer state without appending the cancellation events described in the comment at lines 71-74.

## Adversarial Check

This is not a harmless in-memory cleanup difference. The same impl explicitly documents that shutdown must journal timer cancellations before runs are dropped, and `drain_for_shutdown` calls `cancel_pending_timers_for_shutdown` on the false tick path. The public `drain_pending_and_shutdown` path violates that local durability contract by clearing the same map directly.

## Suggested Fix

Replace the direct `pending_timers.clear()` calls in shutdown drains with `cancel_pending_timers_for_shutdown()?`. If a fast non-durable shutdown mode is required, split it into a separately named method that documents the journal waiver and is not used by normal lifecycle shutdown.

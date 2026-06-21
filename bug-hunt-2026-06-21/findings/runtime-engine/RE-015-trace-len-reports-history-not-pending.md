# RE-015: `TraceRing::len` and `is_empty` report history, not drainable events

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/trace/ring.rs:72-82`
- **Confidence**: confirmed

## Description

`TraceRing::len` and `is_empty` are documented as ring occupancy checks, but they read the remembered history buffer rather than the pending queue drained by `drain` and `drain_into`. After a successful drain, the ring can report non-empty even when there are no pending events left to drain.

## Evidence

`crates/vb_runtime/src/trace/ring.rs:72-82`:

```rust
/// Returns the number of events currently in the ring.
#[must_use]
pub fn len(&self) -> usize {
    self.history.len()
}

/// Returns true if the ring contains no events.
#[must_use]
pub fn is_empty(&self) -> bool {
    self.history.is_empty()
}
```

But `drain` only removes pending events at `crates/vb_runtime/src/trace/ring.rs:111-126`:

```rust
pub fn drain(&mut self) -> Vec<TraceEvent> {
    let mut events = Vec::with_capacity(self.capacity);
    self.drain_into(self.capacity, &mut events);
    events
}

pub fn drain_into(&mut self, limit: usize, events: &mut Vec<TraceEvent>) {
    ...
    let Some(event) = self.pop_pending() else {
        return;
    };
    events.push(event);
}
```

The history buffer is intentionally retained for snapshots, so `len()` remains non-zero after `drain()` empties the pending queue.

## Adversarial Check

The dual-store design is already documented by existing findings, but this is a separate API correctness issue. A caller using `while !ring.is_empty() { ring.drain(); }` can spin because `is_empty` observes history, not pending. If the intended metric is history length, the methods are misnamed and their docs are wrong.

## Suggested Fix

Either make `len` and `is_empty` report pending queue occupancy, or rename them to `history_len` and `history_is_empty`. Add explicit pending/history accessors so callers cannot confuse replayable history with drainable events.

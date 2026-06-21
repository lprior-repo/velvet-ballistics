# RS-008: Coalesce window has an off-by-one — `coalesce_window_ticks: N` produces N−1 dispatches per flush, not N

- **Severity**: Medium
- **Category**: correctness / perf
- **Location**: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:25-65`
- **Confidence**: confirmed

## Description

When `coalesce_window_ticks > 1`, the window counter is reset to `window.saturating_sub(1)` rather than `window`, and the same tick both resets and decrements. The result is that a configured window of `N` only accumulates `N−1` dispatches before flushing, not `N`. With the default `coalesce_window_ticks: 10` (`config.rs:111`), the runtime flushes after 9 dispatches.

## Evidence

```rust
// dispatch.rs:25-65
if self.current_coalesce_window_remaining == 0 {
    let window = self.coalesce_window_ticks;
    self.current_coalesce_window_remaining = window.saturating_sub(1);  // ← starts at N-1
    self.coalesce_buffer.clear();
}
let Some(cmd) = self.command_queue.pop() else { … };
self.dispatch_command(cmd)?;
…
if self.current_coalesce_window_remaining > 0 {
    self.current_coalesce_window_remaining =
        self.current_coalesce_window_remaining.saturating_sub(1);      // ← decrements same tick
}
if self.current_coalesce_window_remaining == 0 {
    self.flush_coalesce_buffer()?;
}
```

Trace with `coalesce_window_ticks = 10`:

| Tick | `current` at entry | Action | `current` after dec | Flush? |
|------|-------------------|--------|---------------------|--------|
| 1    | 0 (reset → 9)     | dispatch | 8                | no     |
| 2    | 8                 | dispatch | 7                | no     |
| 3    | 7                 | dispatch | 6                | no     |
| 4    | 6                 | dispatch | 5                | no     |
| 5    | 5                 | dispatch | 4                | no     |
| 6    | 4                 | dispatch | 3                | no     |
| 7    | 3                 | dispatch | 2                | no     |
| 8    | 2                 | dispatch | 1                | no     |
| 9    | 1                 | dispatch | 0                | yes    |
| 10   | 0 (reset → 9)     | dispatch | 8                | no     |

So 9 dispatches per flush window. The `ShardConfig::coalesce_window_ticks` doc (`config.rs:40-45`) says: "Number of ticks over which to coalesce journal events into a single batch commit." A user setting `coalesce_window_ticks: 10` expects 10 ticks of coalescing.

## Adversarial Check

A defender might argue "9 vs 10 is a minor perf difference, no correctness impact." Two rebuttals:

1. **Semantic contract violation.** Operators tune this knob based on durability latency budget. If they set `coalesce_window_ticks: 2` (a common low-latency setting), the trace shows flush-after-1, i.e. *no* coalescing at all. The default-1 case is also off: window=1 → reset to 0 → same-tick flush. So the lowest meaningful setting collapses to per-tick flush, defeating the feature.

2. **Combined with RS-001** the bug is *worse*: a smaller effective window means the corruption window is entered more often. Every flush under multi-run interleaving corrupts sequences.

## Suggested Fix

Set the counter to `window` on reset (not `window - 1`) and only decrement on subsequent ticks. Or move the decrement before the reset check so the reset wins ties:

```rust
// Decrement first (handles the "window expiring" case)
if self.current_coalesce_window_remaining > 0 {
    self.current_coalesce_window_remaining =
        self.current_coalesce_window_remaining.saturating_sub(1);
}
// Start a fresh window when the previous one expired
if self.current_coalesce_window_remaining == 0 {
    self.flush_coalesce_buffer()?;
    self.current_coalesce_window_remaining = self.coalesce_window_ticks;
    self.coalesce_buffer.clear();
}
let Some(cmd) = self.command_queue.pop() else { return Ok(true); };
self.dispatch_command(cmd)?;
```

This gives exactly `window` dispatches per flush.

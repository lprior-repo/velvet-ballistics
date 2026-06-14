P2-14b2 shard-tick-coalesce: Add coalesce_window_ticks: u32 to ShardConfig (tick-count coalescing, NOT wall-clock) and coalescing layer in Shard::tick

# Verification excerpts (read-before-write)

## crates/vb_runtime/src/shard/config.rs (156 lines)
- Line 27-38: `pub struct ShardConfig` has 5 fields: `command_queue_capacity: usize`, `trace_capacity: usize`, `step_budget_per_tick: u64`, `max_active_runs: usize`, `policy: vb_core::policy::RuntimePolicy`. NO `coalesce_window_us` or `coalesce_window_ticks` currently.
- Line 52-62: `Default for ShardConfig` returns `command_queue_capacity: 1024, trace_capacity: 4096, step_budget_per_tick: 1000, max_active_runs: 1024, policy: Strict`. NO coalescing default.

## crates/vb_runtime/src/shard/impl_parts/dispatch.rs (208 lines)
- Line 3-17: `pub fn tick(&mut self) -> RuntimeResult<bool>` — SYNCHRONOUS, pops ONE command from the queue, dispatches, returns. NO time-based accumulation. NO sleep. NO wall-clock dependency.
- The function increments nothing over wall time; it processes one command per call. This is the canonical sync tick architecture.

## crates/vb_runtime/src/shard/tick.rs — DOES NOT EXIST
- The rejected P2-14r cited `shard/tick.rs` — that file does not exist. The real dispatch is in `shard/impl_parts/dispatch.rs`.

# Scope (verified, no fabrication)

Add ONE new field to `ShardConfig`:
```rust
pub struct ShardConfig {
    // ... existing 5 fields ...
    pub coalesce_window_ticks: u32,  // default 1 (no coalescing)
}
```

Add coalescing logic to `Shard::tick` at `dispatch.rs:3-17`:
- Track `current_coalesce_window_remaining: u32` as a field on `Shard`.
- In `tick()`: if `coalesce_window_ticks > 1` and `current_coalesce_window_remaining > 0`, decrement and CONTINUE without dispatching (let more commands accumulate).
- When the window expires (counter reaches 0), dispatch ALL accumulated commands as a batch via the new `append_sequenced_batch` API (P2-14a).
- When `coalesce_window_ticks == 1` (default), each tick dispatches one command (current behavior — zero overhead).

The "window" is measured in tick counts, NOT wall-clock time. This matches the existing sync tick architecture and is deterministic for replay (master §68 invariant 4).

# Acceptance test

```rust
#[test]
fn shard_tick_with_window_1_dispatches_one_command_per_tick() {
    // Push 100 Submit commands. Set coalesce_window_ticks=1.
    // Call tick() 100 times.
    // Assert each call dispatches 1 command.
}

#[test]
fn shard_tick_with_window_10_dispatches_batch_after_10_ticks() {
    // Push 100 Submit commands. Set coalesce_window_ticks=10.
    // Call tick() 100 times.
    // Assert exactly 10 batch commits are issued.
}
```

# Anti-hallucination guards

- DO NOT cite `crates/vb_runtime/src/shard/tick.rs` — that file does not exist. The real file is `shard/impl_parts/dispatch.rs`.
- DO NOT use wall-clock time (SystemTime, Instant, Duration) for the window. Sync ticks have no time anchor; use tick count.
- DO NOT add `coalesce_window_us: u64` — the unit is `ticks`, not microseconds.

# Kani harness (skipped — coalescing is a counter; bounded by u32; no overflow risk in 100K ticks)

The window is bounded by `u32` (max 4 billion ticks). No arithmetic overflow risk in realistic workloads. Coverage comes from the unit test.

# Dependency

Depends on P2-14a (which adds `append_sequenced_batch` to `RuntimeJournal`). P2-14b2 alone is useless without the batched append.

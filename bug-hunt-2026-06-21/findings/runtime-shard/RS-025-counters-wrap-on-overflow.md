# RS-025: `ShardCounters::add_steps` wraps on `AtomicU64` overflow — counters silently go backwards

- **Severity**: Low
- **Category**: correctness / observability
- **Location**: `crates/vb_runtime/src/counters.rs:48-51`
- **Confidence**: confirmed

## Description

`ShardCounters::add_steps` calls `fetch_add(count, Ordering::Relaxed)`, which wraps silently on `u64` overflow. The LRU ring's counters (`LruRingCounters`, `lru_ring.rs:48-55`) explicitly document "Saturate at `u64::MAX`" and use `saturating_add`. The runtime's own step counter does not — it wraps, producing observations like `steps_executed: 5` after `u64::MAX + 6` cumulative steps. Operators correlating step counts across restarts see negative-looking regressions.

## Evidence

```rust
// counters.rs:48-51
/// Adds to the steps-executed counter.
pub fn add_steps(&self, count: u64) {
    self.steps_executed.fetch_add(count, Ordering::Relaxed);
}
```

```rust
// counters.rs:33-46 (similar pattern for inc_*)
pub fn inc_submitted(&self) {
    self.runs_submitted.fetch_add(1, Ordering::Relaxed);
}
pub fn inc_completed(&self) {
    self.runs_completed.fetch_add(1, Ordering::Relaxed);
}
pub fn inc_failed(&self) {
    self.runs_failed.fetch_add(1, Ordering::Relaxed);
}
```

`fetch_add` on `AtomicU64` is documented as wrapping on overflow (`std::sync::atomic::AtomicU64::fetch_add`).

Compare with the LRU ring's pattern (`lru_ring.rs:232-266`):

```rust
self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
```

The codebase has two different overflow policies for counters in the same crate.

## Adversarial Check

A defender might argue "u64 overflow at one step per nanosecond takes ~584 years." True for `steps_executed`, but:

1. `runs_submitted`, `runs_completed`, `runs_failed` are also wrap-on-overflow. For high-throughput runs (~100k/sec), the submitted counter wraps in ~58 million years — but for tests using arbitrary counter increments (some test suites multiply by 10⁹ to test overflow paths), the wrap is reachable.

2. The contract should be uniform. If the LRU ring uses saturating adds, the runtime counters should too. Mixed policies are a maintenance hazard: a future engineer reading `add_steps` will assume saturating because that is the project pattern, and write code that depends on monotonic counters.

3. `fetch_add` wraps; the result is a non-monotonic counter. The bug-hunt checklist explicitly calls out "Action completion watermark must be monotonic" — the runtime's step counter is a similar monotonicity contract.

## Suggested Fix

Use a `compare_exchange` loop with `saturating_add`:

```rust
pub fn add_steps(&self, count: u64) {
    let mut current = self.steps_executed.load(Ordering::Relaxed);
    loop {
        let next = current.saturating_add(count);
        match self.steps_executed.compare_exchange_weak(
            current, next, Ordering::Relaxed, Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(actual) => current = actual,
        }
    }
}
```

This is heavier than `fetch_add`, but the counter is not on the hottest path (it is incremented once per drive, not per step). Apply the same pattern to `inc_submitted`, `inc_completed`, `inc_failed`. Alternatively, document explicitly that these counters wrap and adjust consumers.

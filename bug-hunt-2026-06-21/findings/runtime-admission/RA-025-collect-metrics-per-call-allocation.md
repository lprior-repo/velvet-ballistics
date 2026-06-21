# RA-025: `collect_metrics` / `counters_snapshot` allocate fresh `Vec`/structs on every call

- **Severity**: Low
- **Category**: perf (allocator pressure on metrics hot path)
- **Location**: `crates/vb_runtime/src/runtime/runtime_metrics.rs:11-20` and `crates/vb_runtime/src/runtime/runtime_metrics.rs:81-96`
- **Confidence**: likely

## Description

`Runtime::collect_metrics` and `Runtime::counters_snapshot` both allocate owned data structures on every call: `collect_metrics` allocates `Vec::with_capacity(shard_count)` plus per-shard `ShardMetricsSnapshot` values (each carrying an inner counters struct), and `counters_snapshot` iterates shards calling `shard.counters().snapshot()` (which itself allocates per call).

## Evidence

`runtime_metrics.rs:11-20`:

```rust
pub(crate) fn collect_metrics(shards_ref: &[Shard], shard_count: usize) -> RuntimeMetricsSnapshot {
    let mut shards = Vec::with_capacity(shard_count);
    let mut totals = RuntimeMetricTotals::default();
    for (index, shard) in shards_ref.iter().enumerate() {
        let metrics = shard_metrics(index, shard);
        totals.add(&metrics);
        shards.push(metrics);
    }
    totals.into_snapshot(shards)
}
```

`runtime_metrics.rs:81-96`:

```rust
pub fn counters_snapshot(&self) -> CounterSnapshot {
    let mut total = CounterSnapshot { runs_submitted: 0, ... };
    for shard in &self.shards {
        let snap = shard.counters().snapshot();
        total.runs_submitted = total.runs_submitted.saturating_add(snap.runs_submitted);
        ...
    }
    total
}
```

Each call: `Vec::with_capacity(shard_count)` + N `shard_metrics` calls (each calling `shard.counters().snapshot()` which allocates internally) + N `ShardMetricsSnapshot` pushes + final move into `RuntimeMetricsSnapshot`. A metrics scraper polling at 10 Hz across 32 shards pays 320+ allocations per second just for `collect_metrics`, plus whatever `counters().snapshot()` allocates per shard.

`RuntimeMetricTotals::add` also uses `saturating_add` on every dimension (lines 32-44). For correct counters that never saturate, this is unnecessary defensive code — the totals are bounded by `shard_count × u64::MAX`, which can overflow but the saturation hides the real bug if it ever does.

## Adversarial Check

One could argue metrics scraping is not a hot path and the allocations are not in the run-execution hot loop. That is true for `collect_metrics`. But observability infrastructure typically wants zero-allocation metrics paths to avoid the observer effect — and `counters_snapshot` is also called by `list_active_runs` consumers indirectly via the per-shard status. Without a benchmark demonstrating the cost, this is "likely" rather than "confirmed" severity.

The `saturating_add` in `add` is harder to defend: if a counter ever saturates, the operator sees a frozen metric with no indication that saturation occurred. Standard practice is to either let it wrap (and rely on Prometheus-style rate calculations to handle the wrap), or to use `checked_add` and surface a separate "counter saturated" flag.

## Suggested Fix

For allocation pressure: add a `Runtime::collect_metrics_into(&self, sink: &mut RuntimeMetricsSnapshot)` variant that reuses an existing snapshot, and have callers in the metrics-scraping loop reuse a single snapshot. For the saturation: replace `saturating_add` with `checked_add` and surface a `counters_saturated: bool` flag on `RuntimeMetricsSnapshot` so the operator knows the totals are unreliable.

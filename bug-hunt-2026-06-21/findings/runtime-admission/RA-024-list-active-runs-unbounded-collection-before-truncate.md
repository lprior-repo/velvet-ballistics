# RA-024: `list_active_runs` allocates `Vec::with_capacity(usize::MAX)` when `limit = u32::MAX` and collects unbounded

- **Severity**: Low
- **Category**: perf (allocator pressure)
- **Location**: `crates/vb_runtime/src/runtime/runtime_control.rs:110-126`
- **Confidence**: likely

## Description

`list_active_runs` converts `limit: u32` to `max: usize` via `unwrap_or(usize::MAX)` and uses it as the per-shard cap for `collect_shard_summaries`. With `limit = u32::MAX` on a 64-bit host, `max = usize::MAX`, and `collect_shard_summaries` will iterate the entire `shard.runs` map for every shard, building an unbounded `summaries` Vec before truncating at the end.

## Evidence

```rust
pub fn list_active_runs(
    &self,
    limit: u32,
    workflow_filter: Option<vb_core::WorkflowDigest>,
) -> Vec<ActiveRunSummary> {
    let max = usize::try_from(limit).unwrap_or(usize::MAX);
    let mut summaries = Vec::new();
    for shard in &self.shards {
        collect_shard_summaries(shard, max, workflow_filter, &mut summaries);
        if summaries.len() >= max {
            break;
        }
    }
    summaries.sort_by_key(|summary| summary.run_id);
    summaries.truncate(max);
    summaries
}
```

`summaries` is allocated with no reserved capacity (`Vec::new()`) and grows via repeated `push` in `collect_one_summary` for every active run on every shard, until either all shards are scanned or `summaries.len() >= max`. With `max = usize::MAX`, every active run on every shard is copied into the Vec before the final `truncate`.

The `Vec::new()` without `with_capacity` also means the Vec reallocates log-many times during growth — a typical concern for hot inspection paths. Even with a bounded `limit`, the function never reserves capacity.

## Adversarial Check

One could argue inspection APIs are not hot paths and a one-shot allocation cost is acceptable. That argument holds for low `limit` values, but `list_active_runs(u32::MAX, None)` is a documented public API that walks the entire fleet — an operator running it on a 100 000-run fleet allocates 100 000 `ActiveRunSummary` values, sorts them, then truncates. The sort cost is `O(n log n)` on the unbounded Vec even when the caller only needed the first 10 results. The simpler fix is to bound the per-shard collection by `min(limit, shard.active_run_count())` and reserve capacity up front.

## Suggested Fix

```rust
pub fn list_active_runs(
    &self,
    limit: u32,
    workflow_filter: Option<vb_core::WorkflowDigest>,
) -> Vec<ActiveRunSummary> {
    let max = usize::try_from(limit).unwrap_or(usize::MAX);
    let mut summaries = Vec::with_capacity(self.shards.len().min(max));
    for shard in &self.shards {
        let remaining = max - summaries.len();
        collect_shard_summaries(shard, remaining, workflow_filter, &mut summaries);
        if summaries.len() >= max {
            break;
        }
    }
    summaries.sort_by_key(|summary| summary.run_id);
    summaries.truncate(max);
    summaries
}
```

The key change is passing `remaining` (not the original `max`) into `collect_shard_summaries` after the first shard fills up the budget, and reserving capacity based on shard count.

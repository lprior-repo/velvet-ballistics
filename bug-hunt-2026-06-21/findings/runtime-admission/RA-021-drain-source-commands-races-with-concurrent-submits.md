# RA-021: `drain_source_commands` measures `command_queue_len()` then pops in a `while drained < limit` loop, racing with concurrent producers

- **Severity**: Low
- **Category**: concurrency (TOCTOU)
- **Location**: `crates/vb_runtime/src/runtime/runtime_control.rs:264-279`
- **Confidence**: likely

## Description

`drain_source_commands` snapshots `shard.command_queue_len()` into `limit`, then pops up to `limit` commands in a `while drained < limit` loop. Because `command_queue.pop()` is `&self` (interior-mutable via the underlying `crossbeam_queue::ArrayQueue`), concurrent `Runtime::submit_*` callers on the same shard can push additional commands between the `len()` snapshot and the loop. Those new commands remain on the source shard after migration, which is correct for the "snapshot-then-migrate" semantics, but the function returns those stragglers as "source has work" via `source_has_work(source)`, which forces the caller to re-issue migration in a loop.

## Evidence

```rust
fn drain_source_commands(&self, source: usize) -> RuntimeResult<Vec<ShardCommand>> {
    let Some(shard) = self.shards.get(source) else {
        return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
    };
    let limit = shard.command_queue_len();
    let mut commands = Vec::with_capacity(limit);
    let mut drained = 0usize;
    while drained < limit {
        let Some(command) = shard.command_queue.pop() else {
            break;
        };
        commands.push(command);
        drained = drained.saturating_add(1);
    }
    Ok(commands)
}
```

`Shard` is documented as single-threaded (`config.rs:212`: "Single-threaded shard owning all mutable run state"), but `Runtime::submit_*` methods take `&self` and reach `shard.enqueue(...)` via the queue's interior mutability. Any `&self`-holding caller can race with `drain_source_commands` if the runtime is shared (e.g., behind an `Arc<Runtime>`). The admission_lock does NOT protect this drain path — it only covers preflight+enqueue of submits.

A second, subtler issue: `migrate_selected_shard` calls `drain_source_commands(&self, ...)` while taking `&mut self`. The signature is fine, but the function drops the `&mut` borrow before calling `enqueue_migrated_commands`, which means a concurrent submit could interleave between drain and enqueue — pushing to source AFTER drain completes but BEFORE source_has_work is checked, leading to a straggler that source_has_work correctly reports but the caller may not expect.

## Adversarial Check

One could argue that since `Shard` is documented as single-threaded, no concurrent submit can occur during drain — so the race window is purely theoretical. That argument only holds if the entire `Runtime` is single-threaded. But `Runtime::submit_*` methods all take `&self` (not `&mut self`), which is a clear API signal that the runtime is intended to be shared. The admission_lock pattern's entire existence (per `config.rs:258-266`) is to serialize concurrent submits — proving the codebase expects concurrent callers. So the race is real on the runtime-sharing deployment model. Even on a single-threaded executor, the loop's defensive `while drained < limit` with `saturating_add` is harder to read than `(0..limit).map_while(|_| shard.command_queue.pop()).collect()`.

## Suggested Fix

Functional-rust rewrite that removes the manual index arithmetic:

```rust
fn drain_source_commands(&self, source: usize) -> RuntimeResult<Vec<ShardCommand>> {
    let Some(shard) = self.shards.get(source) else {
        return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
    };
    let limit = shard.command_queue_len();
    Ok((0..limit).map_while(|_| shard.command_queue.pop()).collect())
}
```

For the race: hold the `admission_lock` for both source and target across drain+enqueue so concurrent submits cannot interleave. Document the locking discipline in the docstring.

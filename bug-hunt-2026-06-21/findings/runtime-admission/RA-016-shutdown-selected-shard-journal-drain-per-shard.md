# RA-016: `shutdown_selected_shard` double-drains the journal when invoked per shard

- **Severity**: Low
- **Category**: correctness (idempotency / consistency)
- **Location**: `crates/vb_runtime/src/runtime/runtime_control.rs:302-310`
- **Confidence**: confirmed

## Description

`shutdown_selected_shard` (the `ShardDirective::Shutdown` arm of `tick_shard`) calls `self.journal.drain_for_shutdown()` on every invocation, which is shared across all shards. `shutdown_graceful` calls it exactly once after all shards have been drained. A caller that shuts down N shards one at a time via `tick_shard(ShardDirective::Shutdown)` triggers `drain_for_shutdown` N times.

## Evidence

`runtime_control.rs:302-310`:

```rust
fn shutdown_selected_shard(&mut self, source: usize) -> RuntimeResult<bool> {
    let Some(shard) = self.shards.get_mut(source) else {
        return Err(RuntimeError::ShardNotFound { shard: u32::MAX });
    };
    shard.enqueue(ShardCommand::Shutdown)?;
    shard.drain_for_shutdown()?;
    self.journal.drain_for_shutdown()?;
    Ok(false)
}
```

Compare to `shutdown_graceful` (lines 218-227):

```rust
pub fn shutdown_graceful(&mut self) -> RuntimeResult<()> {
    for shard in &self.shards {
        shard.enqueue(ShardCommand::Shutdown)?;
    }
    for shard in &mut self.shards {
        shard.drain_for_shutdown()?;
    }
    self.journal.drain_for_shutdown()?;   // <-- called ONCE
    Ok(())
}
```

`self.journal` is a single `SharedRuntimeJournal` shared by every shard (see `Runtime::new_with_journal`, which clones the journal into each shard but stores the same `Arc` in `Runtime::journal`). `drain_for_shutdown` on a journal is conceptually a terminal flush. Calling it N times for N shards is at minimum redundant and at worst incorrect depending on the journal's drain idempotency.

## Adversarial Check

One could argue `drain_for_shutdown` should be idempotent and the redundant calls are harmless. That argument requires the journal implementation to make a stronger guarantee than `shutdown_graceful` relies on. The asymmetry between the two shutdown paths (`shutdown_graceful` drains once, `shutdown_selected_shard` drains per-call) itself is evidence the contract is not well-specified. If `drain_for_shutdown` flushes a coalesce buffer (which `Shard::coalesce_buffer` does — see `config.rs:257`), then the per-shard drain might be intended to flush per-shard coalesce state into the journal before the journal itself drains — but `shard.drain_for_shutdown()` already does that, so the additional `self.journal.drain_for_shutdown()` inside the per-shard loop is doing global work that belongs outside the loop.

## Suggested Fix

Remove `self.journal.drain_for_shutdown()?;` from `shutdown_selected_shard`. The single-shard directive should drain the shard's own queue and let the caller drive the global journal drain (or call `shutdown_graceful` if they want a global flush). Alternatively, document that `tick_shard(Shutdown)` is intended to fully quiesce the journal and make `drain_for_shutdown` provably idempotent.

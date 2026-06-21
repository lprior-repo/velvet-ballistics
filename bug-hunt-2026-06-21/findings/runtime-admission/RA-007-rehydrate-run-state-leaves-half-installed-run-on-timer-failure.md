# RA-007: `rehydrate_run_state` inserts run state then can fail in `insert_timer`, leaving the run half-installed

- **Severity**: Low
- **Category**: correctness (atomicity)
- **Location**: `crates/vb_runtime/src/runtime/runtime_recovery.rs:154-173` and `crates/vb_runtime/src/runtime/runtime_recovery.rs:206-222`
- **Confidence**: confirmed

## Description

`rehydrate_run_state` first installs the run into the shard via `insert_into_shard` (which writes `runtime_states`, `pending_workflows`, and `runs`) and only then calls `insert_timer` to register the pending timer. `insert_timer` can return `Err(RuntimeError::InvalidTimerFire)` if `next_pending_timer_generation` overflows `u64`, so a successful insertion is followed by an error path that does not unwind the insertion.

## Evidence

```rust
fn rehydrate_run_state(
    &mut self,
    run: RunId,
    frame: vb_core::frame::RunFrame,
    slot_count: u16,
    pending_timer: Option<PendingTimer>,
) -> crate::RuntimeResult<()> {
    let shard_idx = self.shard_index(run);
    {
        let shard = self.shards.get_mut(shard_idx).ok_or(...)?;
        Self::insert_into_shard(shard, run, frame, slot_count);
    }
    if let Some(timer) = pending_timer {
        Self::insert_timer(self, run, shard_idx, timer)?;
    }
    Ok(())
}
```

`insert_timer` (line 218) maps `next_pending_timer_generation` returning `None` to `RuntimeError::InvalidTimerFire`. The `None` arm triggers when the existing timer's generation is already at `u64::MAX` (`shard/impl_parts/timer_methods.rs:65-70`). After this error, the shard has the run installed (counted in `active_run_count`, present in `runs`, `runtime_states`, `pending_workflows`) but no pending timer — so the run is in `RuntimeState::Resumable` and visible to `list_active_runs`, yet cannot be advanced because nothing will wake it.

## Adversarial Check

One could argue generation overflow at `u64::MAX` is effectively unreachable — a run would need to have been cancelled and restarted `2^64` times. That is true in practice, but the error variant exists, the code path is reachable in principle, and the partial-install consequence is real: a subsequent `tick_all` will treat the run as resumable but the timer wheel will never fire for it. The error is also misclassified — `InvalidTimerFire` is the wrong variant for a generation overflow during recovery. The fix is small (insert the timer first, or roll back the insertion on failure), so the cost/benefit favors fixing it.

## Suggested Fix

Either (a) compute the new generation *before* `insert_into_shard` so failure aborts before any state mutation; or (b) wrap the insert + timer-register pair in a small helper that removes the run from all three maps on timer-registration failure. Also reclassify the error: `InvalidTimerFire` is for runtime-fired timer authority mismatches, not recovery-time generation overflow.

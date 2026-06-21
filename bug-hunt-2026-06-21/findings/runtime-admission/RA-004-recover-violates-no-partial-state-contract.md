# RA-004: `Runtime::recover` violates its "no partial state" docstring on mid-loop hydration failure

- **Severity**: Medium
- **Category**: correctness (atomicity / idempotency)
- **Location**: `crates/vb_runtime/src/runtime/runtime_recovery.rs:33-49` and `crates/vb_runtime/src/runtime/runtime_recovery.rs:53-78`
- **Confidence**: confirmed

## Description

The `recover` docstring claims "any error in hydration is propagated without partial state," but the implementation iterates hydrations sequentially and commits each one's shard state via `recover_one_run` *before* the next hydration is attempted. If hydration `N` fails, hydrations `0..N` have already inserted runtime states, pending workflows, and run state into the live shards, and those mutations are not rolled back.

## Evidence

```rust
pub fn recover(&mut self, journal: &SharedRuntimeJournal) -> crate::RuntimeResult<Vec<RunId>> {
    let hydrations = vb_storage::recovery::recover_all_incomplete_runs(...)
        .map_err(|_| crate::RuntimeError::InvalidRecoveryHydration)?;

    let mut recovered = Vec::with_capacity(hydrations.len());
    for hydration in hydrations {
        if let Some(run) = self.recover_one_run(journal, hydration)? {
            recovered.push(run);
        }
    }
    Ok(recovered)
}
```

`recover_one_run` calls `Self::rehydrate_run_state(self, run, frame, slot_count, pending_timer)?` which calls `Self::insert_into_shard(shard, run, ...)` (`runtime_recovery.rs:175-187`) that mutates `shard.runtime_states`, `shard.pending_workflows`, and `shard.runs`. None of these insertions are conditional on subsequent hydrations succeeding, and the function returns the underlying `?` error directly. There is no rollback path.

Additionally, `recover_one_run` writes the snapshot via `vb_storage::recovery::write_recovered_snapshot` at `runtime_recovery.rs:71` *before* any of `hydrate_frame`, `rehydrate_run_state`, or `insert_timer` run. If any of those later steps fails, the snapshot is already persisted, so a subsequent recovery attempt observes a snapshot for a run whose in-memory state was never installed — a non-idempotent persistence ordering.

## Adversarial Check

One could argue "partial state on failure is acceptable because the caller will tear down the runtime anyway." But the docstring explicitly promises atomicity and the `Runtime` struct is left in a usable but inconsistent state — a caller that catches the error and continues serving will observe some recovered runs and not others with no indication which. The `write_recovered_snapshot`-before-rehydrate ordering is independently problematic: it persists a checkpoint that attests to recovery that did not complete, defeating the "future recoveries can short-circuit" optimization comment at line 65-67 because the short-circuit path will skip replay for a run whose shard state is missing.

Note: this module is gated by `#[cfg(feature = "test-util")]`, which lowers the production blast radius but does not excuse the docstring/implementation mismatch — the API is publicly exported under that feature and is the only path to multi-process recovery.

## Suggested Fix

Either (a) make `recover` truly atomic by staging all hydration results in a local `Vec<(RunId, Frame, SlotCount, Option<PendingTimer>)>` first, and only after the full Vec is built, commit them all to the shards — so any failure aborts before any mutation; or (b) update the docstring to say "partial state may be present on failure" and document the recovery protocol callers must follow. Additionally, move the `write_recovered_snapshot` call to *after* `rehydrate_run_state` and `insert_timer` have succeeded so the snapshot attests to a completed hydration.

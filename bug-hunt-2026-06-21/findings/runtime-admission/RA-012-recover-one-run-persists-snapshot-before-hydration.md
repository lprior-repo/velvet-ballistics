# RA-012: `Runtime::recover` persists recovered snapshot before in-memory hydration completes

- **Severity**: Low
- **Category**: correctness (durability ordering)
- **Location**: `crates/vb_runtime/src/runtime/runtime_recovery.rs:65-78`
- **Confidence**: confirmed

## Description

`recover_one_run` calls `vb_storage::recovery::write_recovered_snapshot` against the durable journal *before* `hydrate_frame` and `rehydrate_run_state` have any chance to fail. A failure in either subsequent step leaves a snapshot on disk that attests to a recovery that did not complete, defeating the "future recoveries can short-circuit" optimization.

## Evidence

```rust
fn recover_one_run(
    &mut self,
    journal: &SharedRuntimeJournal,
    hydration: vb_storage::recovery::RecoveryHydration,
) -> crate::RuntimeResult<Option<RunId>> {
    let seed = match hydration {
        vb_storage::recovery::RecoveryHydration::FrameSeed(s) => s,
        _ => return Ok(None),
    };
    let run = seed.summary.run;
    let slot_count = seed.slot_count;
    let pc = seed.pc;
    if let Some(fjall_journal) = journal.storage_journal()
        && seed.unsupported.is_fully_supported()
    {
        vb_storage::recovery::write_recovered_snapshot(fjall_journal.as_ref(), &seed)
            .map_err(|_| crate::RuntimeError::InvalidRecoveryHydration)?;
    }
    let frame = Self::hydrate_frame(seed)?;             // can fail
    let pending_timer = Self::recover_timer_from_journal(journal, run, pc)?;  // can fail
    Self::rehydrate_run_state(self, run, frame, slot_count, pending_timer)?;  // can fail
    Ok(Some(run))
}
```

The snapshot is written at line 71. The three subsequent `?` propagations (lines 74-76) abort recovery without unwinding the snapshot. The snapshot itself encodes the seed, which is valid, but the *in-memory* shard state for that run is missing — so the next process restart will load the snapshot via `recover_snapshot_plus_tail` and assume the run is live, when in fact no shard has it installed.

## Adversarial Check

One could argue the snapshot is independent of the in-memory state — a future recovery would re-install the run from the snapshot, so persisting it early is just aggressive caching. That argument would hold if the snapshot write were idempotent and complete. It is not complete: the snapshot is derived from `seed`, which is the *input* to hydration, not the *output*. The output (a fully installed run with the correct pending timer) is what the snapshot is meant to short-circuit. Persisting the input snapshot before producing the output breaks the optimization's invariant: "if a snapshot exists, the run was successfully recovered in the previous process."

## Suggested Fix

Move the `write_recovered_snapshot` call to *after* `rehydrate_run_state` has succeeded, so the persisted snapshot attests to a completed recovery. This also removes the early persistence as a side effect of fixing RA-007.

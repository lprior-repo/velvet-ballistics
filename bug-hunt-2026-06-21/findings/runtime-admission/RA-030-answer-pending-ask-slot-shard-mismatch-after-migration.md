# RA-030: `answer_pending_ask_slot` accesses shard via `shard_index` without checking the run is asking on that shard

- **Severity**: Low
- **Category**: correctness (silent fallthrough)
- **Location**: `crates/vb_runtime/src/runtime/runtime_ask.rs:11-35`
- **Confidence**: likely

## Description

`answer_pending_ask_slot` derives the shard via `shard_index(run)`, then probes `shard.run_state_contains(run)` (via `shard_for_pending_ask`) and `shard.pending_timer_get(run)`. If the run was migrated to another shard (per RA-021) the run may live on a different shard than `shard_index(run)` reports, and the function returns `RunNotFound` even though the run still exists somewhere.

## Evidence

```rust
pub fn answer_pending_ask_slot(
    &mut self,
    run: RunId,
    answer_slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
    encoded_len: u32,
) -> RuntimeResult<()> {
    let shard_index = self.shard_index(run);
    let shard = shard_for_pending_ask(&mut self.shards, shard_index, run)?;
    let pending_timer = ask_timer_for_run(shard, run)?;
    let resume_step = ask_resume_step(shard, run, pending_timer, answer_slot)?;
    ...
}
```

`shard_index` is the deterministic `run.get() % shard_count` mapping. The function assumes the run lives at that index. Migration (`Runtime::tick_shard(ShardDirective::Migrate { target })`) can move commands and run state across shards, but `shard_index(run)` always returns the original mapping.

If a run was migrated away from its `shard_index` target, `shard_for_pending_ask` sees `!shard.run_state_contains(run)` and returns `Err(RuntimeError::RunNotFound)` even though the run is still live — just on a different shard.

## Adversarial Check

One could argue migration is not yet wired for live run state (the `migrate_selected_shard` helper moves only commands, not `runs` map entries — see RA-021 trace). That argument holds for the current code. But the docstring on `migrate_selected_shard` and `ShardDirective::Migrate` describes a migration path that is intended to relocate work, and `answer_pending_ask_slot`'s exclusive reliance on `shard_index(run)` will break the moment migration moves run state. The asymmetry between `shard_index` (constant per RunId) and `shard_for_pending_ask` (existence probe) is a latent bug waiting for the migration feature to land.

Also, even without migration: `answer_pending_ask_slot` takes `&mut self`, but every helper it calls (`shard_for_pending_ask`, `ask_timer_for_run`, `ask_resume_step`, `ask_answer`) takes `&Shard` or `&mut Shard` — the `&mut self` is only needed for the final `shard.handle_ask_answer(answer)`. The signature is wider than necessary.

## Suggested Fix

Either (a) scan all shards for the run when `shard_index(run)` lookup fails:

```rust
let shard = self.shards.iter_mut().find(|s| s.run_state_contains(run))
    .ok_or(RuntimeError::RunNotFound)?;
```

or (b) make migration update a `RunId -> shard_index` indirection map so the lookup is correct after migration. Option (a) is O(shard_count) but correct; option (b) is O(1) but requires invariant maintenance.

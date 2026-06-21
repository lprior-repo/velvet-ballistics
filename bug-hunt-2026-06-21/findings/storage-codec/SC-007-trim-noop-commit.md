# SC-007: `trim_events_for_run` commits an empty batch when `skip_noop_runs == false`

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_storage/src/trimming/logic.rs:58-109`
- **Confidence**: confirmed

## Description

When `policy.skip_noop_runs` is `false`, the early-return at line 92-99 is skipped, so a run with zero trimmable events still executes `batch.commit()?` at line 101. The commit writes an empty Fjall write batch, which still pays WAL and LSM-tree accounting cost without producing any durable state change.

## Evidence

```rust
// crates/vb_storage/src/trimming/logic.rs:90-108
        if deleted_count == 0 && policy.skip_noop_runs {
            return Ok(TrimmedRunResult {
                run,
                deleted_count: 0,
                cutoff_seq,
                status: TrimStatus::NoOp,
            });
        }

        batch.commit()?;                                  // <-- runs even when deleted_count == 0

        Ok(TrimmedRunResult {
            run,
            deleted_count,
            cutoff_seq,
            status: TrimStatus::Trimmed,                  // <-- reports Trimmed even when 0 deleted
        });
```

Two distinct issues compound: (1) the no-op commit costs an fsync-equivalent barrier per run; (2) the result reports `TrimStatus::Trimmed` even when nothing was trimmed, contradicting the `NoOp` semantics used elsewhere.

## Adversarial Check

`trim_all_eligible_runs` (line 116-130) calls this per run header. With `skip_noop_runs == false`, every run in the journal triggers a Fjall commit. For a journal with thousands of runs (most without trimmable events), this is thousands of empty commits per trim pass. Each commit takes the journal's `write_lock` and hits the LSM memtable, serializing against concurrent writers. The status misreport (`Trimmed` for zero deletes) also corrupts any operator dashboard that distinguishes NoOp from Trimmed.

## Suggested Fix

```rust
let status = if deleted_count == 0 {
    TrimStatus::NoOp
} else {
    batch.commit()?;
    TrimStatus::Trimmed
};
Ok(TrimmedRunResult { run, deleted_count, cutoff_seq, status })
```

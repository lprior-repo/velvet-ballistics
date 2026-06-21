# SR-004: `load_snapshot` conflates "snapshot missing" with "snapshot corrupt"

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/replay/recovery_ops.rs:93`
- **Confidence**: confirmed

## Description

`load_snapshot` maps both `Ok(None)` (snapshot does not exist for this run/seq)
and `Err(JournalError::PostcardDecodeFailed)` (snapshot exists but cannot be
decoded) to the same `RecoveryError::CorruptSnapshot`. Callers cannot
distinguish "fall back to full journal replay" from "the snapshot is damaged;
do not trust it". This is a real distinction: a missing snapshot is normal
operation, while a corrupt snapshot is a durability incident.

## Evidence

```rust
pub fn load_snapshot(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> crate::recovery::RecoveryResult<crate::recovery::RunSnapshot> {
    match journal.snapshot(run, seq) {
        Ok(Some(snapshot)) => Ok(snapshot),
        Ok(None) | Err(JournalError::PostcardDecodeFailed) => {
            Err(crate::recovery::RecoveryError::CorruptSnapshot { run, seq })
        }
        Err(other) => Err(crate::recovery::RecoveryError::Journal(other)),
    }
}
```

A caller that wants to do "load snapshot at seq N if it exists; otherwise
replay the journal from seq 0" cannot use `load_snapshot` for the lookup,
because both states collapse into `CorruptSnapshot`. The caller is forced to
either reimplement the lookup with `journal.snapshot(run, seq)` directly, or
to treat every `CorruptSnapshot` as a hard failure — which means a single
missing snapshot aborts the recovery batch.

## Adversarial Check

A counter-argument: the function is named `load_snapshot` and its docstring
promises to translate decode failures to `CorruptSnapshot`, so collapsing
`Ok(None)` is at least consistent with one reading of the contract. But
`Ok(None)` is not a decode failure — it is the journal saying "no row at
this key", which is a normal state for any run that has not yet been
snapshotted at that seq. Conflating it with corruption forces every caller
that wants graceful degradation to bypass the helper entirely.

## Suggested Fix

Either split the helper into `load_snapshot_optional` (returns
`Result<Option<RunSnapshot>, RecoveryError>`) or add a new
`RecoveryError::SnapshotNotFound { run, seq }` variant and map `Ok(None)`
to it. Keep `CorruptSnapshot` for genuine decode failures.

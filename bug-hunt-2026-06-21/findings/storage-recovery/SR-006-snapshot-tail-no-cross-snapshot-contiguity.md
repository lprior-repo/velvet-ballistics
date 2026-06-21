# SR-006: Snapshot+tail validation does not enforce cross-snapshot sequence contiguity

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/hydrate/validation.rs:84`
- **Confidence**: confirmed

## Description

`validate_tail_seq_after_snapshot` only checks that each tail event's seq is
strictly greater than `snapshot.seq` (`event.seq.get() > snapshot_seq.get()`).
It does not check that the first tail event is exactly `snapshot.seq + 1`,
nor does `validate_contiguous_sequences` (called inside
`replay_events_with_schedule_requirement`) bridge across the snapshot
boundary. A tail that starts at `snapshot.seq + 5` passes validation and
silently drops events `snapshot.seq + 1 .. snapshot.seq + 4`.

## Evidence

`validation.rs:84-96`:
```rust
pub(crate) fn validate_tail_seq_after_snapshot(
    event: TailEventMetadata,
    snapshot_seq: crate::EventSeq,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if event.seq.get() > snapshot_seq.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot { ... })
    }
}
```

The same weak check is also in `recover_snapshot_plus_tail`
(`recovery_ops.rs:117`):
```rust
for event in tail_events {
    if event.seq() <= snapshot_seq {
        return Err(...);
    }
}
```

Neither caller checks `tail_events[0].seq == snapshot.seq + 1`. The inner
`validate_contiguous_sequences` (core.rs:161) runs over `tail_events` only,
so it sees `[5, 6, 7, 8]` as contiguous and never notices the gap at seq 4
relative to `snapshot.seq = 3`.

Failure mode: any code path that constructs a `RunSnapshot + tail_events`
pair (manual recovery, snapshot rotation, snapshot corruption that loses the
intermediate events) will silently drop up to `tail[0].seq - snapshot.seq - 1`
events from the replayed state.

## Adversarial Check

A counter-argument: in normal operation the journal layer produces the tail
by iterating from `next_seq(snapshot.seq)` (see `events_for_run_bounded`),
so the gap cannot arise organically. But `hydrate_run_frame` and
`recover_snapshot_plus_tail` accept caller-supplied tail slices, and at least
one caller (`assert_snapshot_tail_matches_full_summary` in `tests.rs:1205`)
explicitly uses a `tail_after` helper that filters by seq. A future caller
that builds the tail from a different source (e.g. a snapshot export, a
replicated log segment) will silently drop events. The contract should be
enforced at the boundary, not at the journal helper.

## Suggested Fix

After the per-event strict-greater check, also assert:
```rust
if let Some(first) = tail_events.first() {
    let expected = crate::codec::next_seq(snapshot.seq)?;
    if first.seq() != expected {
        return Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not contiguous with snapshot seq {}",
                first.seq().get(),
                snapshot.seq.get()
            ),
        });
    }
}
```
This matches the contiguity rule that `validate_contiguous_sequences`
enforces within the tail and closes the cross-snapshot gap.

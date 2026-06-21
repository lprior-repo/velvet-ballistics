# SR-009: `sub_tail_parallel_in_flight` silently swallows counter underflow

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/event_replay/tail.rs:25`
- **Confidence**: confirmed

## Description

`sub_tail_parallel_in_flight` returns `Ok(())` whenever
`frame.parallel_in_flight() == 0` regardless of the event being processed.
This is inconsistent with the parallel-counter path in
`recovery/event_replay/parallel.rs` (`compute_parallel_in_flight`), which
returns `RecoveryError::ReplayDivergence` on the same underflow. The tail
path therefore tolerates journals that the parallel path rejects, masking
real corruption (e.g. a completion event without a matching schedule).

## Evidence

`tail.rs:25-35` — lenient:
```rust
fn sub_tail_parallel_in_flight(frame: &mut RunFrame, step: StepIdx) -> RecoveryResult<()> {
    if frame.parallel_in_flight() == 0 {
        return Ok(());                                                  // <-- silent swallow
    }
    frame
        .sub_parallel_in_flight(1)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "parallel_in_flight underflow".to_owned(),
        })
}
```

`parallel.rs:73-79` — strict:
```rust
JournalEvent::ActionCompletedEvent { action, step, .. } => {
    if tracker.is_resolved(*action, *step) {
        return Err(RecoveryError::NonIdempotentActionBlocked { ... });
    }
    tracker.mark_completed(*action, *step);
    frame
        .sub_parallel_in_flight(1)
        .map_err(|_| RecoveryError::ReplayDivergence {
            step: *step,
            detail: "parallel_in_flight underflow".to_owned(),         // <-- hard error
        })?;
}
```

So an `ActionCompletedEvent` that arrives with `parallel_in_flight == 0`
fails in `compute_parallel_in_flight` but succeeds in `apply_tail_events`.
`hydrate_run_frame_from_events` calls both (`hydrate/mod.rs:111-112`): the
second pass (`apply_parallel_peak`) reports the underflow, but the first
pass (`apply_tail_events`) has already silently written the post-completion
state to the frame. The result is a frame whose slot writes / step
transitions reflect the corrupt event, with a downstream divergence error
that does not name the actual culprit.

## Adversarial Check

The lenient behavior in the tail path is *defensible* for snapshot+tail
hydration: a completion in the tail may correspond to a schedule that
happened before the snapshot, so decrementing below the snapshot's baseline
should not fail. But that argument only applies when the schedule genuinely
preceded the snapshot, which the function cannot verify. The right behavior
is to record the underflow as a tracked signal (e.g. a "frame underflow at
seq N" diagnostic) rather than silently `Ok(())`. At minimum, the two paths
that compute the same counter should agree on the underflow policy. As
written, a snapshot-less events-only replay with a corrupt completion passes
the state-mutation pass and fails the parallel-peak pass, leaving the frame
in a half-applied state that callers cannot easily attribute to a specific
event.

## Suggested Fix

Either:

1. Make both paths strict and force callers that legitimately need
   snapshot+tail semantics to initialize `parallel_in_flight` from the
   snapshot before calling `apply_tail_events`.
2. Keep the tail path lenient but log a `tracing::warn!` with the step and
   current seq so operators can detect underflow events instead of having
   them silently absorbed.

Option (1) is preferable because it eliminates the behavioral asymmetry
that allows corrupt events to slip through the tail pass.

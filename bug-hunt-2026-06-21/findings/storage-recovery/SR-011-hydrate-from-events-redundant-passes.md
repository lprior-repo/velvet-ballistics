# SR-011: `hydrate_run_frame_from_events` makes redundant passes over the event list

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_storage/src/recovery/hydrate/mod.rs:111`
- **Confidence**: confirmed

## Description

`hydrate_run_frame_from_events` runs three independent iterations over the
event slice: `recover_runtime_frame_seed_from_events` (which itself folds
the accumulator over every event), `count_state_events` (which walks every
event with its own `ActionReplayTracker`), and `apply_parallel_peak` (which
calls `compute_parallel_in_flight` and walks every event a third time with a
third tracker). The tracker state computed in pass 2 is discarded before
pass 3 re-derives the same information.

## Evidence

```rust
pub fn hydrate_run_frame_from_events(
    events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame> {
    if !invariants::hydrate_events_preconditions(events) {
        return Err(RecoveryError::NoRecoveryData { run: run_id });
    }

    let seed = crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(events)?;  // pass 1
    ensure_nonzero_step_count(seed.step_count)?;
    let mut frame = build_frame_from_seed(&seed, run_id)?;
    apply::apply_seed_step_states(&mut frame, &seed.steps)?;
    apply::apply_seed_slots(&mut frame, &seed.slots)?;
    apply::apply_seed_pc(&mut frame, seed.pc)?;
    apply::increment_executed(&mut frame, run_id, count_state_events(events, run_id)?)?;            // pass 2
    apply::apply_parallel_peak(&mut frame, events)?;                                                // pass 3

    Ok(frame)
}
```

Each of the three passes invokes either `verified_action_envelope_digest`
(which calls `blake3::hash` over the action value) or
`verify_action_ticket_event`. For long runs (thousands of action events),
the Blake3 hashes and postcard decodes are repeated two or three times.

Compare with `hydrate_run_frame` (snapshot+tail path, hydrate/mod.rs:55): it
makes a single `apply_tail_events` pass that simultaneously populates slot
state, step state, action tracker, and the `executed` counter.

## Adversarial Check

One could argue that the three passes have *slightly* different
responsibilities (seed construction, executed counter, parallel peak) and
that fusing them would couple unrelated logic. But all three already
maintain their own `ActionReplayTracker` and decode the same envelope
payloads. The work is not just iteration overhead — it is repeated Blake3
hashing and postcard deserialization. For runs dominated by
`ActionCompletedEnvelope` events (which carry value bytes that get hashed
each time), the duplication is observable.

## Suggested Fix

Fuse the three passes into a single fold that:
1. Builds the seed accumulator.
2. Counts state-affecting events.
3. Tracks the parallel-in-flight peak.

Or, if fusion is too invasive, cache the verified envelope digests (keyed
by `(action, step)`) computed during the seed pass and reuse them in
`apply_parallel_peak`. At minimum, the parallel-peak pass should accept the
already-built tracker from the seed accumulator instead of re-deriving it.

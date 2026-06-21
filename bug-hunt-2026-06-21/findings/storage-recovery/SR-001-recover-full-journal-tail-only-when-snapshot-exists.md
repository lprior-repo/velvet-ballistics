# SR-001: `recover_full_journal` silently performs tail-only replay when a snapshot exists

- **Severity**: Critical
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/replay/recovery_ops.rs:41`
- **Confidence**: confirmed

## Description

`recover_full_journal` is documented as replaying "a full journal for a run
when no snapshot is available" and verifying durable `RunAdmission` evidence.
But the function loads its events via `journal.events_for_run(run)`, which in
`events_for_run_bounded` starts iteration at the sequence *after*
`latest_durable_snapshot_seq(run)`. The result is that when any snapshot
exists for the run, the function replays only the tail, silently skips
`RunAccepted`/`RunAdmission`, and almost always fails admission verification
with a confusing `PolicyDigestExpectationMissing` or `NoRecoveryData`.

## Evidence

```rust
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut crate::recovery::ActionReplayTracker,
    expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> crate::recovery::RecoveryResult<Vec<JournalEvent>> {
    let events = journal.events_for_run(run)?;        // <-- tail-only when snapshot exists
    if events.is_empty() {
        return Err(crate::recovery::RecoveryError::NoRecoveryData { run });
    }
    super::admission::verify_run_admission_evidence(&events, run, expected_policy_digests)?;
    ...
}
```

`journal/events_for_run_bounded` (replay.rs:71-84):
```rust
let (start_seq, first_event) = match self.latest_durable_snapshot_seq(run)? {
    Some(seq) => {
        let tail_start = crate::codec::next_seq(seq)?;
        (tail_start, tail_start)
    }
    None => (EventSeq::new(0), EventSeq::new(0)),
};
self.events_for_run_from(run, start_seq, first_event, limit)
```

`latest_durable_snapshot_seq` scans the `run_snapshot` keyspace and returns
the highest snapshot sequence. Once a snapshot has ever been written for the
run, every subsequent `recover_full_journal` invocation silently drops every
event at or before the snapshot seq — including the `RunAccepted` and
`RunAdmission` events that `verify_run_admission_evidence` is specifically
designed to find.

Callers in production code paths:
- `crates/vb_cli/src/replay.rs:32` — the `replay` CLI command.
- `crates/vb_storage/src/convenience.rs:37` — public `replay_journal` wrapper.

Neither caller checks for snapshot existence first; both trust
`recover_full_journal` to do what its name says.

## Adversarial Check

A plausible counter-argument is that `recover_full_journal` is only *intended*
to be called when no snapshot exists, and callers are expected to dispatch
between `recover_full_journal` and `recover_snapshot_plus_tail` based on
snapshot presence. But the function's documentation does not state that
precondition, the function does not assert it, and neither caller enforces
it. The failure mode is also uniquely bad: rather than erroring with "snapshot
exists, use snapshot+tail", the function silently truncates the event list
and then *fails admission verification*, producing an error message
(`PolicyDigestExpectationMissing` or `ReplayDivergence`) that is
indistinguishable from genuine journal corruption. An operator running
`velvet-ballistics replay <run>` after a snapshot will see a confusing
"admission evidence missing" error instead of either a correct full replay
or a clear "use snapshot path" message. The existing test
`assert_snapshot_tail_matches_full_summary` (tests.rs:1186) only exercises
`recover_full_journal` *before* a snapshot is written, so the bug is not
covered.

## Suggested Fix

Two options:

1. Make the function name honest. Add a new method
   `events_for_run_full(run)` that always iterates from `EventSeq::ZERO`, and
   have `recover_full_journal` use it. Keep `events_for_run` for the
   snapshot-optimized read path.
2. If tail-only is intentional after snapshot, have `recover_full_journal`
   start by checking `latest_durable_snapshot_seq(run)?` and return a typed
   error like `RecoveryError::SnapshotExists { run, seq }` directing the
   caller to `recover_snapshot_plus_tail`. Never silently do less than the
   name promises.

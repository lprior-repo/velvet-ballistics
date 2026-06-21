# SR-002: Public recovery APIs silently skip pre-snapshot events

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/recover.rs:200`
- **Confidence**: confirmed

## Description

`recover_runtime_summary`, `recover_runtime_summary_with_expected`,
`recover_runtime_frame_seed`, `recover_run_admission`, and
`recover_all_incomplete_runs` all load events via
`journal.events_for_run(run)`. That helper skips every event at or before
`latest_durable_snapshot_seq(run)`. When a snapshot exists, these functions
silently produce summaries / seeds / admission records built only from the
tail, missing the `RunAccepted`/`RunAdmission` events, most step states,
slot writes, and action schedules.

## Evidence

```rust
pub fn recover_runtime_summary(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryHydration> {
    let events = journal.events_for_run(run)?;          // <-- tail-only after snapshot
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    crate::recovery::replay::summary::summarize_recovery_events(&events)
}
```

And `recover_runtime_frame_seed` (recover.rs:247), `recover_run_admission`
(recover.rs:259), `recover_all_incomplete_runs` (recover.rs:273),
`recover_runtime_summary_with_expected` (recover.rs:211), and
`check_workflow_source_digest` (recover.rs:20) all use the same
`events_for_run` call.

`events_for_run_bounded` (journal/replay.rs:71):
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

Concretely:
- `recover_run_admission` returns `None` once a snapshot exists, because the
  `RunAdmission` event always precedes any snapshot.
- `recover_runtime_summary` returns a summary whose `workflow` is `None`
  (because `RunAccepted` is also before the snapshot), and whose counters
  under-count by the snapshot's seq.
- `recover_runtime_frame_seed` builds a seed whose `step_states` map is
  missing every step that was started before the snapshot, producing a frame
  that misrepresents the run's progress.
- `recover_all_incomplete_runs` filters runs via `extract_terminal(&events)`
  on the tail; runs whose terminal event was at or before the snapshot will
  be reported as "incomplete" and re-enqueued for recovery.

## Adversarial Check

One could argue these APIs are *intended* for the snapshot-less case, with
`hydrate_run_frame` covering the snapshot case. But (1) the docstrings do
not say so, (2) `recover_runtime_summary` is `pub` and is the documented
entry point for "summary-only recovery product for a run" with no
qualification, and (3) `recover_all_incomplete_runs` is explicitly designed
to scan *every* run header in the journal — including runs that were
snapshotted and then continued — and the function has no fallback to merge
snapshot state with tail state. The combination means a single
`put_snapshot` call corrupts the output of every one of these public
functions for that run.

## Suggested Fix

Two layers:

1. Each of these public functions should either explicitly reject runs with
   a snapshot (`latest_durable_snapshot_seq(run)?` is `Some`) with a typed
   error, or it should load `RunSnapshot` + tail events and merge them
   itself.
2. Introduce a `events_for_run_full(run)` reader that ignores snapshots and
   use it in any function whose contract requires the full event history
   (admission, summary, frame seed).

Until one of those lands, the public recovery API is unsafe for any run that
has ever been snapshotted.

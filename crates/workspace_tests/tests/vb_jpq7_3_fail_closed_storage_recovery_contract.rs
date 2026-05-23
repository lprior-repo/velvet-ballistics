#![forbid(unsafe_code)]

use vb_core::{RunId, StepIdx, WorkflowDigest};
use vb_storage::{
    EventReplayLimit, EventSeq, FjallJournal, JournalError, JournalEvent, RunSnapshot,
};

const JOURNAL_REPLAY_SOURCE: &str = include_str!("../../vb_storage/src/journal/replay.rs");
const JOURNAL_CORE_SOURCE: &str = include_str!("../../vb_storage/src/journal/core.rs");
const JOURNAL_APPEND_SOURCE: &str = include_str!("../../vb_storage/src/journal/append.rs");
const HYDRATE_SUPPORT_SOURCE: &str =
    include_str!("../../vb_storage/src/recovery/hydrate_support.rs");

fn step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn empty_snapshot(run: RunId, seq: u64) -> RunSnapshot {
    RunSnapshot {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0xA5; 32]),
        slots: Vec::new(),
        taint: Vec::new(),
    }
}

fn journal_at_temp_path() -> Result<(tempfile::TempDir, FjallJournal), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|err| err.to_string())?;
    Ok((temp, journal))
}

#[test]
fn given_explicit_replay_limit_when_more_events_exist_then_too_many_events_and_code_are_returned()
-> Result<(), String> {
    // Given: a run has two contiguous durable events but the caller allows only one.
    let (_temp, journal) = journal_at_temp_path()?;
    let run = RunId::new(73_001);
    journal
        .append_journaled(&step_started(run, 0, 0))
        .map_err(|err| err.to_string())?;
    journal
        .append_journaled(&step_started(run, 1, 1))
        .map_err(|err| err.to_string())?;
    let Some(limit) = EventReplayLimit::new(1) else {
        return Err("non-zero replay limit must be accepted".to_owned());
    };

    // When: bounded replay reaches the second event.
    let result = journal.events_for_run_bounded(run, limit);

    // Then: replay fails closed with the exact stable error payload and diagnostic code.
    match result {
        Err(JournalError::TooManyEvents {
            run: observed_run,
            limit: observed_limit,
            observed,
        }) => {
            assert_eq!(observed_run, run);
            assert_eq!(observed_limit, 1);
            assert_eq!(observed, 2);
            assert_eq!(
                JournalError::TooManyEvents {
                    run,
                    limit: observed_limit,
                    observed,
                }
                .diagnostic_code(),
                JournalError::TOO_MANY_EVENTS_CODE
            );
            assert_eq!(JournalError::TOO_MANY_EVENTS_CODE.code(), 0x401E);
        }
        other => return Err(format!("expected TooManyEvents, got {other:?}")),
    }
    Ok(())
}

#[test]
fn given_first_tail_event_is_missing_when_replaying_run_then_sequence_gap_points_after_snapshot()
-> Result<(), String> {
    // Given: a durable snapshot claims seq=2, but the first retained tail event starts at seq=4.
    let (_temp, journal) = journal_at_temp_path()?;
    let run = RunId::new(73_002);
    journal
        .put_snapshot(&empty_snapshot(run, 2))
        .map_err(|err| err.to_string())?;
    journal
        .append_journaled(&step_started(run, 4, 4))
        .map_err(|err| err.to_string())?;

    // When: replay starts strictly after the latest durable snapshot boundary.
    let result = journal.events_for_run(run);

    // Then: the missing first tail event is not laundered into a successful tail replay.
    match result {
        Err(JournalError::SequenceGap { expected, actual }) => {
            assert_eq!(expected, EventSeq::new(3));
            assert_eq!(actual, EventSeq::new(4));
            assert_eq!(
                JournalError::SequenceGap { expected, actual }.diagnostic_code(),
                JournalError::SEQUENCE_GAP_CODE
            );
            assert_eq!(JournalError::SEQUENCE_GAP_CODE.code(), 0x4009);
        }
        other => {
            return Err(format!(
                "expected SequenceGap after snapshot boundary, got {other:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn given_close_after_unpersisted_append_when_reopened_then_event_is_observable()
-> Result<(), String> {
    // Given: an event is appended through the non-strict path.
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let path = temp.path().to_path_buf();
    let run = RunId::new(73_003);
    {
        let mut journal = FjallJournal::open(&path, None).map_err(|err| err.to_string())?;
        journal
            .append_journaled(&step_started(run, 0, 0))
            .map_err(|err| err.to_string())?;

        // When: the caller explicitly observes the durability barrier result.
        let close_result = journal.close();

        // Then: a healthy store reports the exact success value instead of hiding the outcome.
        assert_eq!(close_result.map_err(|err| err.to_string())?, ());
    }

    // Then: reopening the store observes the event flushed by close().
    let journal = FjallJournal::open(&path, None).map_err(|err| err.to_string())?;
    let replayed = journal.events_for_run(run).map_err(|err| err.to_string())?;
    assert_eq!(replayed, vec![step_started(run, 0, 0)]);
    Ok(())
}

#[test]
fn given_zero_replay_limit_when_constructed_then_limit_is_rejected_before_replay()
-> Result<(), String> {
    // Given/When/Then: zero is not a valid fail-closed replay bound.
    assert_eq!(EventReplayLimit::new(0), None);
    let Some(limit) = EventReplayLimit::new(1) else {
        return Err("limit of one must be constructible".to_owned());
    };
    assert_eq!(limit.max_events(), 1);
    Ok(())
}

#[test]
fn given_snapshot_index_read_fails_when_events_for_run_starts_then_error_is_not_erased()
-> Result<(), String> {
    // Given: latest durable snapshot lookup is recovery-critical state.
    let erases_trim_error = JOURNAL_REPLAY_SOURCE
        .contains(".latest_durable_snapshot_seq(run)\n            .ok()")
        || JOURNAL_REPLAY_SOURCE
            .contains(".ok()\n            .flatten()\n            .unwrap_or(EventSeq::new(0))");
    let defaults_after_lookup = JOURNAL_REPLAY_SOURCE.contains("unwrap_or(EventSeq::new(0))");

    // When: events_for_run chooses its replay start sequence.
    let propagates_snapshot_lookup =
        JOURNAL_REPLAY_SOURCE.contains("latest_durable_snapshot_seq(run)?");

    // Then: a storage/index error cannot be silently treated as no snapshot.
    assert_eq!(erases_trim_error, false);
    assert_eq!(defaults_after_lookup, false);
    assert_eq!(propagates_snapshot_lookup, true);
    Ok(())
}

#[test]
fn given_tail_slot_write_when_recovery_reads_existing_taint_then_read_failure_is_typed_error()
-> Result<(), String> {
    // Given: recovery must preserve taint and fail closed if the frame cannot read it.
    let defaults_failed_read_to_clean =
        HYDRATE_SUPPORT_SOURCE.contains("frame.read_taint(*slot).unwrap_or(vb_core::Taint::Clean)");
    let uses_typed_read_taint_error = HYDRATE_SUPPORT_SOURCE.contains("frame.read_taint(*slot)")
        && HYDRATE_SUPPORT_SOURCE.contains("RecoveryError::SlotTaintReadFailed")
        && HYDRATE_SUPPORT_SOURCE.contains("Err(_) =>")
        && HYDRATE_SUPPORT_SOURCE.contains("return Err(RecoveryError::SlotTaintReadFailed")
        && HYDRATE_SUPPORT_SOURCE.contains("read_taint");

    // When: the hydration support source is scanned for the slot write recovery path.
    // Then: corrupt dimensions or out-of-range slots must not downgrade to Clean taint.
    assert_eq!(defaults_failed_read_to_clean, false);
    assert_eq!(uses_typed_read_taint_error, true);
    Ok(())
}

#[test]
fn given_run_event_replay_api_when_public_contract_is_scanned_then_unbounded_vec_api_is_not_the_only_path()
-> Result<(), String> {
    // Given: run replay can be large and must have an explicit bound or streaming contract.
    let default_replay_delegates_to_bound = JOURNAL_REPLAY_SOURCE
        .contains("self.events_for_run_bounded(run, EventReplayLimit::DEFAULT)");
    let exposes_explicit_bound_or_stream = JOURNAL_REPLAY_SOURCE.contains("events_for_run_bounded")
        || JOURNAL_REPLAY_SOURCE.contains("events_for_run_stream")
        || JOURNAL_REPLAY_SOURCE.contains("ReplayLimit")
        || JOURNAL_REPLAY_SOURCE.contains("EventReplayLimit");

    // When: callers ask for a run's events.
    // Then: the API surface must make memory bounds explicit instead of forcing all events into Vec.
    assert_eq!(default_replay_delegates_to_bound, true);
    assert_eq!(exposes_explicit_bound_or_stream, true);
    Ok(())
}

#[test]
fn given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result()
-> Result<(), String> {
    // Given: an explicit persist API already exists for callers that can observe durability errors.
    let explicit_persist_result_api = JOURNAL_APPEND_SOURCE
        .contains("pub fn persist_strict(&self) -> Result<(), JournalError>")
        && JOURNAL_APPEND_SOURCE.contains("self.database.persist(fjall::PersistMode::SyncAll)?;")
        && JOURNAL_APPEND_SOURCE.contains("Ok(())");

    // When: implicit shutdown/drop behavior is scanned.
    let drop_discards_persist_error = JOURNAL_CORE_SOURCE.contains("impl Drop for FjallJournal")
        && (JOURNAL_CORE_SOURCE.contains("let _ = e;")
            || JOURNAL_CORE_SOURCE.contains("let _ = self.database.persist"));

    // Then: any implicit durability boundary must not hide a SyncAll failure.
    assert_eq!(explicit_persist_result_api, true);
    assert_eq!(drop_discards_persist_error, false);
    Ok(())
}

#[test]
fn given_snapshot_after_many_old_events_when_replaying_then_pre_snapshot_work_does_not_exhaust_limit()
-> Result<(), String> {
    // Given: many old events exist before the durable snapshot boundary.
    let (_temp, journal) = journal_at_temp_path()?;
    let run = RunId::new(73_004);
    for seq in 0..101_u64 {
        journal
            .append_journaled(&step_started(run, seq, 0))
            .map_err(|err| err.to_string())?;
    }
    journal
        .put_snapshot(&empty_snapshot(run, 99))
        .map_err(|err| err.to_string())?;
    let Some(limit) = EventReplayLimit::new(1) else {
        return Err("limit of one must be constructible".to_owned());
    };

    // When: replay starts after the snapshot boundary with a one-event collection limit.
    let replayed = journal
        .events_for_run_bounded(run, limit)
        .map_err(|err| err.to_string())?;

    // Then: pre-snapshot entries do not consume scan/collection budget; only seq 100 is returned.
    assert_eq!(replayed, vec![step_started(run, 100, 0)]);
    Ok(())
}

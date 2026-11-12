#![forbid(unsafe_code)]

use vb_core::{ListId, RunId, SlotIdx, SlotValue, StepIdx, WorkflowDigest};
use vb_runtime::primitives::collect::CollectPaginationState;
use vb_storage::recovery::{RecoveryError, hydrate_run_frame, hydrate_run_frame_from_events};
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

fn slot_written(
    run: RunId,
    seq: u64,
    slot: SlotIdx,
    value: SlotValue,
) -> Result<JournalEvent, String> {
    let payload = postcard::to_allocvec(&value).map_err(|err| err.to_string())?;
    Ok(JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(seq),
        slot,
        value: Some(payload),
        extra: None,
        attempt: 1,
    })
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

fn corrupt_slot_taint_envelope() -> Vec<u8> {
    let mut bytes = vb_storage::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    bytes.extend_from_slice(&[255, 255, 255]);
    bytes
}

fn collect_frame_extra(run: RunId, slot: SlotIdx) -> Result<Vec<u8>, String> {
    let state = CollectPaginationState {
        run_id: run,
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(2),
        cursor: 1,
        page_size: 1,
        item_count: 2,
        limit: 2,
        time_limit_ms: None,
        start_millis: 0,
    };
    postcard::to_allocvec(&state).map_err(|err| err.to_string())
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
fn given_public_hydration_tail_slot_cannot_be_dimensioned_when_recovery_runs_then_clean_taint_is_not_defaulted()
-> Result<(), String> {
    // Given: public snapshot+tail hydration receives a tail slot write at the largest slot index.
    let run = RunId::new(73_008);
    let snapshot = empty_snapshot(run, 0);
    let tail = vec![slot_written(
        run,
        1,
        SlotIdx::new(u16::MAX),
        SlotValue::I64(7),
    )?];

    // When: hydration derives dimensions before applying the slot write.
    let result = hydrate_run_frame(&snapshot, &tail, run);

    // Then: the public recovery path fails closed instead of creating an implicit Clean slot.
    match result {
        Err(RecoveryError::FrameDimensionOverflow { run: observed }) => {
            assert_eq!(observed, run);
        }
        other => {
            return Err(format!(
                "expected FrameDimensionOverflow before Clean taint default, got {other:?}"
            ));
        }
    }

    // And: the lower-level tail write helper keeps the fail-closed taint lattice wired.
    let defaults_failed_read_to_clean =
        HYDRATE_SUPPORT_SOURCE.contains("frame.read_taint(*slot).unwrap_or(vb_core::Taint::Clean)");
    let reads_slot_taint = HYDRATE_SUPPORT_SOURCE.contains("frame.read_taint(*slot)")
        || HYDRATE_SUPPORT_SOURCE.contains("frame.read_taint(slot)");
    let uses_typed_read_taint_error = reads_slot_taint
        && HYDRATE_SUPPORT_SOURCE.contains("resolve_slot_taint_read")
        && HYDRATE_SUPPORT_SOURCE.contains("SlotTaintResolution::FailClosed")
        && HYDRATE_SUPPORT_SOURCE.contains("RecoveryError::SlotTaintReadFailed");

    assert_eq!(defaults_failed_read_to_clean, false);
    assert_eq!(uses_typed_read_taint_error, true);
    Ok(())
}

#[test]
fn given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed()
-> Result<(), String> {
    // Given: a full-journal slot write has a valid value that legacy fallback would call Clean,
    // but its persisted taint sidecar bytes are corrupt.
    let run = RunId::new(73_009);
    let slot = SlotIdx::new(0);
    let value = postcard::to_allocvec(&SlotValue::Bool(false)).map_err(|err| err.to_string())?;
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xA5; 32]),
        },
        step_started(run, 1, 0),
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot,
            value: Some(value),
            extra: Some(corrupt_slot_taint_envelope()),
            attempt: 1,
        },
    ];

    // When: full-journal hydration decodes the runtime frame seed.
    let result = hydrate_run_frame_from_events(&events, run);

    // Then: corrupt taint metadata is not erased into legacy Clean/default taint.
    match result {
        Err(RecoveryError::CorruptSlotTaint { slot: observed }) => assert_eq!(observed, slot),
        other => return Err(format!("expected CorruptSlotTaint, got {other:?}")),
    }
    Ok(())
}

#[test]
fn given_legacy_collect_frame_extra_when_hydrating_full_journal_then_extra_is_not_corrupt_and_taint_fails_closed()
-> Result<(), String> {
    // Given: legacy runtime records used SlotWrittenEvent.extra for collect pagination state.
    let run = RunId::new(73_010);
    let slot = SlotIdx::new(0);
    let value = postcard::to_allocvec(&SlotValue::Bool(false)).map_err(|err| err.to_string())?;
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xA5; 32]),
        },
        step_started(run, 1, 0),
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot,
            value: Some(value),
            extra: Some(collect_frame_extra(run, slot)?),
            attempt: 1,
        },
    ];

    // When: full-journal hydration sees legacy frame extra bytes.
    // The storage layer now fails closed at the frame-seed boundary:
    // a frame seed alone never carries the full RunState, so the
    // hydration returns `UnsupportedFrameSeed`. The legacy frame
    // extra is no longer misclassified as corrupt taint, but the
    // frame cannot be built without the missing full-state
    // components (workflow, store, action attempts, admission,
    // collect states, action contracts, action ABI digests).
    let result = hydrate_run_frame_from_events(&events, run);

    // Then: the storage boundary rejects the hydration with the
    // typed `UnsupportedFrameSeed` error, not corrupt taint.
    match result {
        Err(RecoveryError::UnsupportedFrameSeed { run: found, .. }) => {
            assert_eq!(found, run, "rejected run must match input run");
        }
        other => return Err(format!("expected UnsupportedFrameSeed, got {other:?}")),
    }
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

#![allow(clippy::panic, clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_runtime::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
use vb_storage::recovery::{
    RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryFrameSeed,
    RecoveryRuntimeSummary, UnsupportedRecoveryState, recover_runtime_frame_seed_from_events,
};
use vb_storage::{
    EventSeq, FjallJournal, JournalEvent, JournalWriterFlushReport, JournalWriterQueue,
    StorageLimits,
};

fn encoded(value: SlotValue) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(&value)
}

fn complete_slot_events(
    run: RunId,
    step: StepIdx,
    slot: SlotIdx,
    value: SlotValue,
) -> Result<Vec<JournalEvent>, postcard::Error> {
    Ok(vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([17; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot,
            value: Some(encoded(value)?),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step,
            output: slot,
        },
    ])
}

fn no_output_events(run: RunId, step: StepIdx) -> Vec<JournalEvent> {
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([23; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step,
            output: SlotIdx::ZERO,
        },
    ]
}

fn seed_for(events: &[JournalEvent]) -> Result<RecoveryFrameSeed, String> {
    recover_runtime_frame_seed_from_events(events).map_err(|error| error.to_string())
}

fn unsupported_for(events: &[JournalEvent]) -> Result<UnsupportedRecoveryState, String> {
    seed_for(events).map(|seed| seed.unsupported)
}

fn slots_for(events: &[JournalEvent]) -> Result<Vec<RecoveredSlotEntry>, String> {
    seed_for(events).map(|seed| seed.slots)
}

fn slot_count_for(events: &[JournalEvent]) -> Result<u16, String> {
    seed_for(events).map(|seed| seed.slot_count)
}

fn hydration_label(seed: RecoveryFrameSeed) -> Result<(), String> {
    DurableFrameRecoveryBoundary::from_seed(seed)
        .hydrate_run_frame()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn supported_seed_with_slot(taint: Taint) -> RecoveryFrameSeed {
    RecoveryFrameSeed {
        summary: RecoveryRuntimeSummary {
            run: RunId::new(42),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(3),
            workflow: Some(WorkflowDigest::from_bytes([31; 32])),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 1,
            terminal: None,
        },
        first_step: StepIdx::ZERO,
        step_count: 4,
        slot_count: 4,
        pc: StepIdx::new(2),
        steps: vec![RecoveredStepEntry {
            step: StepIdx::new(2),
            state: RecoveredStepState::Succeeded,
        }],
        slots: vec![RecoveredSlotEntry {
            slot: SlotIdx::new(3),
            value: SlotValue::I64(99),
            taint,
        }],
        unsupported: UnsupportedRecoveryState::SUPPORTED,
    }
}

macro_rules! red_complete_slot_contract_test {
    ($name:ident, $value:expr, $taint:expr) => {
        #[test]
        fn $name() -> Result<(), postcard::Error> {
            let run = RunId::new(42);
            let events = complete_slot_events(run, StepIdx::new(2), SlotIdx::new(3), $value)?;
            let expected = vec![RecoveredSlotEntry {
                slot: SlotIdx::new(3),
                value: $value,
                taint: $taint,
            }];

            assert_eq!(slots_for(&events), Ok(expected));
            assert_eq!(
                unsupported_for(&events),
                Ok(UnsupportedRecoveryState::SUPPORTED)
            );
            Ok(())
        }
    };
}

red_complete_slot_contract_test!(
    event_only_recovery_returns_secret_i64_when_durable_taint_is_secret,
    SlotValue::I64(99),
    Taint::Secret
);
red_complete_slot_contract_test!(
    event_only_recovery_returns_derived_bool_when_durable_taint_is_derived,
    SlotValue::Bool(true),
    Taint::DerivedFromSecret
);
red_complete_slot_contract_test!(
    action_completion_records_exact_secret_taint_when_action_writes_output,
    SlotValue::I64(7),
    Taint::Secret
);
red_complete_slot_contract_test!(
    ask_answer_records_exact_clean_taint_when_answer_writes_output,
    SlotValue::Bool(false),
    Taint::Clean
);
red_complete_slot_contract_test!(
    runtime_to_storage_mapping_preserves_taint_for_slot_write,
    SlotValue::Null,
    Taint::DerivedFromSecret
);

#[test]
fn event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid() {
    let events = must_postcard(complete_slot_events(
        RunId::new(43),
        StepIdx::new(1),
        SlotIdx::new(2),
        SlotValue::I64(12),
    ));

    assert_eq!(
        unsupported_for(&events),
        Ok(UnsupportedRecoveryState::SUPPORTED)
    );
}

#[test]
fn deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete() {
    let events = must_postcard(complete_slot_events(
        RunId::new(44),
        StepIdx::new(2),
        SlotIdx::new(3),
        SlotValue::I64(99),
    ));
    let hydrated = seed_for(&events).and_then(hydration_label);

    assert_eq!(hydrated, Ok(()));
}

#[test]
fn recovery_does_not_default_missing_durable_taint_to_clean() {
    let events = must_postcard(complete_slot_events(
        RunId::new(45),
        StepIdx::new(2),
        SlotIdx::new(3),
        SlotValue::I64(99),
    ));
    let expected = vec![RecoveredSlotEntry {
        slot: SlotIdx::new(3),
        value: SlotValue::I64(99),
        taint: Taint::Secret,
    }];

    assert_eq!(slots_for(&events), Ok(expected));
}

fn must_postcard<T>(result: Result<T, postcard::Error>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("postcard serialization failed: {error}"),
    }
}

#[test]
fn no_output_step_does_not_fabricate_slot_zero_dimension() {
    let events = no_output_events(RunId::new(46), StepIdx::new(2));

    assert_eq!(slot_count_for(&events), Ok(0));
}

#[test]
fn no_output_step_summary_reports_zero_slots_written() {
    let events = no_output_events(RunId::new(47), StepIdx::new(2));
    let summary_slot_dimension = slot_count_for(&events).map(u64::from);

    assert_eq!(summary_slot_dimension, Ok(0));
}

#[test]
fn no_output_step_recovery_has_no_recovered_slot_entries() {
    let events = no_output_events(RunId::new(48), StepIdx::new(2));

    assert_eq!(slot_count_for(&events), Ok(0));
}

#[test]
fn corrupt_slot_value_blocks_both_values_and_taint() {
    let run = RunId::new(49);
    let events = vec![JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(0),
        slot: SlotIdx::new(1),
        value: Some(vec![255, 0, 19]),
        extra: None,
        attempt: 1,
    }];
    let expected = UnsupportedRecoveryState {
        slot_values: true,
        slot_taint: false,
        action_payloads: false,
    };

    assert_eq!(unsupported_for(&events), Ok(expected));
}

#[test]
fn missing_slot_value_blocks_both_values_and_taint() {
    let run = RunId::new(50);
    let events = vec![JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(0),
        slot: SlotIdx::new(1),
        value: None,
        extra: None,
        attempt: 1,
    }];
    let expected = UnsupportedRecoveryState {
        slot_values: true,
        slot_taint: false,
        action_payloads: false,
    };

    assert_eq!(unsupported_for(&events), Ok(expected));
}

#[test]
fn supported_seed_hydrates_exact_secret_taint() {
    let hydrated = hydration_label(supported_seed_with_slot(Taint::Secret));

    assert_eq!(hydrated, Ok(()));
}

#[test]
fn supported_seed_hydrates_exact_derived_taint() {
    let hydrated = hydration_label(supported_seed_with_slot(Taint::DerivedFromSecret));

    assert_eq!(hydrated, Ok(()));
}

#[test]
fn drain_report_contract_requires_three_drained_and_three_written() {
    let temp_dir = tempfile::tempdir().expect("tempdir must succeed");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("journal open must succeed");
    let queue =
        JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("queue create must succeed");
    let run = RunId::new(9999);

    queue
        .enqueue_journaled(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        })
        .expect("enqueue 0 must succeed");
    queue
        .enqueue_journaled(JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        })
        .expect("enqueue 1 must succeed");
    queue
        .enqueue_journaled(JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            output: SlotIdx::ZERO,
        })
        .expect("enqueue 2 must succeed");

    let report = queue.drain_all(&journal).expect("drain_all must succeed");

    assert_eq!(
        report,
        JournalWriterFlushReport {
            drained: 3,
            written: 3,
            pending_after: 0,
        }
    );
}

proptest! {
    #[test]
    fn proptest_event_only_slot_recovery_preserves_secret_taint(value in -128_i64..128_i64) {
        let events = complete_slot_events(
            RunId::new(60),
            StepIdx::new(1),
            SlotIdx::new(1),
            SlotValue::I64(value),
        ).map_err(|error| TestCaseError::fail(error.to_string()))?;
        let expected = vec![RecoveredSlotEntry {
            slot: SlotIdx::new(1),
            value: SlotValue::I64(value),
            taint: Taint::Secret,
        }];

        prop_assert_eq!(slots_for(&events), Ok(expected));
    }

    #[test]
    fn proptest_no_output_success_never_creates_slot_zero(step in 0_u16..16_u16) {
        let events = no_output_events(RunId::new(61), StepIdx::new(step));

        prop_assert_eq!(slot_count_for(&events), Ok(0));
    }

    #[test]
    fn proptest_valid_slot_events_are_fully_hydrateable(slot in 0_u16..16_u16, value in 0_i64..1024_i64) {
        let events = complete_slot_events(
            RunId::new(62),
            StepIdx::new(1),
            SlotIdx::new(slot),
            SlotValue::I64(value),
        ).map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert_eq!(unsupported_for(&events), Ok(UnsupportedRecoveryState::SUPPORTED));
    }
}

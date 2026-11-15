use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{
    RecoveredStepEntry, RecoveredStepState, RecoveryError,
    recover_raw_runtime_frame_seed_from_events, recover_runtime_frame_seed_from_events,
};
use vb_storage::{EventSeq, JournalEvent};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn events(output: SlotIdx) -> Vec<JournalEvent> {
    let run = RunId::new(460);
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: WorkflowDigest::from_bytes([46; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            output,
        },
    ]
}

#[test]
fn event_only_seed_preserves_zero_marker_and_accounts_for_output_slots() -> TestResult {
    for (output, count) in [(0, 0), (7, 8), (65534, 65535)] {
        let evidence = events(SlotIdx::new(output));
        let raw = recover_raw_runtime_frame_seed_from_events(&evidence)?;
        assert_eq!(raw.slot_count, count);
        assert_eq!(raw.step_count, 1);
        assert_eq!(raw.slots, vec![]);
        assert_eq!(
            raw.steps,
            vec![RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            }]
        );
        let typed = recover_runtime_frame_seed_from_events(&evidence)?;
        assert_eq!(typed.slot_count, count);
        assert_eq!(*typed, raw);
        assert!(!typed.is_resumable());
    }
    let evidence = events(SlotIdx::new(7));
    for prefix in [&evidence[..1], &evidence[..2]] {
        assert_eq!(
            recover_raw_runtime_frame_seed_from_events(prefix)?.slot_count,
            0
        );
        assert_eq!(
            recover_runtime_frame_seed_from_events(prefix)?.slot_count,
            0
        );
    }
    Ok(())
}

#[test]
fn event_only_seed_rejects_unrepresentable_output_dimension() {
    let evidence = events(SlotIdx::MAX);
    assert_eq!(
        recover_raw_runtime_frame_seed_from_events(&evidence),
        Err(RecoveryError::FrameDimensionOverflow {
            run: RunId::new(460)
        })
    );
    assert_eq!(
        recover_runtime_frame_seed_from_events(&evidence),
        Err(RecoveryError::FrameDimensionOverflow {
            run: RunId::new(460)
        })
    );
}

#[test]
fn later_success_does_not_shrink_an_observed_output_dimension() -> TestResult {
    for (first, second) in [(0, 7), (7, 0)] {
        let mut evidence = events(SlotIdx::new(first));
        evidence.push(JournalEvent::StepSucceeded {
            run: RunId::new(460),
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            output: SlotIdx::new(second),
        });
        let raw = recover_raw_runtime_frame_seed_from_events(&evidence)?;
        assert_eq!(raw.slot_count, 8);
        assert_eq!(raw.step_count, 2);
        let typed = recover_runtime_frame_seed_from_events(&evidence)?;
        assert_eq!(*typed, raw);
        assert!(!typed.is_resumable());
    }
    Ok(())
}

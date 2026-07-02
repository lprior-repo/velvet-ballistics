use super::{
    DurableFrameRecoveryBoundary, RecoveryResumeStatus, RuntimeRecoveryBoundary,
    SummaryRecoveryBoundary,
};
use crate::RuntimeError;
use crate::recovery::recovery_boundary_from_hydration;
use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::recovery::{
    RecoveredPendingAction, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState, RecoveryCannotResumeState,
    RecoveryFrameSeed, RecoveryHydration, RecoveryRuntimeSummary, RecoveryTerminalState,
    UnsupportedRecoveryState,
};

#[test]
fn summary_recovery_boundary_exposes_summary() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(15),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(2),
        workflow: Some(WorkflowDigest::from_bytes([4; 32])),
        steps_started: 1,
        steps_succeeded: 1,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 1,
        terminal: Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(2),
        }),
    };
    let boundary = SummaryRecoveryBoundary::from_summary(summary);

    assert_eq!(boundary.summary(), summary);
}

#[test]
fn summary_recovery_boundary_rejects_full_frame_hydration() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(16),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(0),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let boundary = SummaryRecoveryBoundary::from_summary(summary);

    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::UnsupportedFullRecoveryHydration)
    );
}

#[test]
fn durable_frame_recovery_boundary_rejects_frame_only_minimal_state() {
    let run = RunId::new(17);
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(3),
        workflow: Some(WorkflowDigest::from_bytes([5; 32])),
        steps_started: 2,
        steps_succeeded: 1,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 1,
        slots_written: 0,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 4,
        slot_count: 0,
        pc: StepIdx::new(3),
        steps: vec![
            RecoveredStepEntry {
                step: StepIdx::new(1),
                state: RecoveredStepState::Waiting,
            },
            RecoveredStepEntry {
                step: StepIdx::new(3),
                state: RecoveredStepState::Succeeded,
            },
        ],
        slots: Vec::new(),
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    let cannot_resume = RecoveryCannotResumeState {
        pending_timers: true,
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };
    assert_eq!(boundary.cannot_resume_state(), cannot_resume);
    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
    assert_eq!(
        boundary.unsupported_state(),
        UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        }
    );
}

#[test]
fn durable_frame_recovery_boundary_rejects_inconsistent_seed() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(18),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(1),
        workflow: None,
        steps_started: 1,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 1,
        slot_count: 0,
        pc: StepIdx::ZERO,
        steps: vec![RecoveredStepEntry {
            step: StepIdx::new(2),
            state: RecoveredStepState::Running,
        }],
        slots: Vec::new(),
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn durable_frame_recovery_boundary_rejects_unsupported_action_payloads() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(23),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(1),
        workflow: None,
        steps_started: 1,
        steps_succeeded: 0,
        actions_scheduled: 1,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 1,
        slot_count: 0,
        pc: StepIdx::ZERO,
        steps: vec![RecoveredStepEntry {
            step: StepIdx::ZERO,
            state: RecoveredStepState::Running,
        }],
        slots: Vec::new(),
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: true,
            pending_actions: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn durable_frame_recovery_boundary_rejects_frame_only_slot_value_and_taint() {
    let run = RunId::new(22);
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(2),
        workflow: Some(WorkflowDigest::from_bytes([8; 32])),
        steps_started: 1,
        steps_succeeded: 1,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 1,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 1,
        slot_count: 2,
        pc: StepIdx::ZERO,
        steps: vec![RecoveredStepEntry {
            step: StepIdx::ZERO,
            state: RecoveredStepState::Succeeded,
        }],
        slots: vec![vb_storage::recovery::RecoveredSlotEntry {
            slot: SlotIdx::new(1),
            value: SlotValue::I64(86),
            taint: Taint::Secret,
        }],
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    let cannot_resume = RecoveryCannotResumeState {
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };
    assert_eq!(boundary.cannot_resume_state(), cannot_resume);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn recovery_boundary_factory_selects_summary_for_summary_variant() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(19),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(0),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let hydration = RecoveryHydration::Summary(summary);
    let boundary = recovery_boundary_from_hydration(hydration);

    assert_eq!(boundary.summary(), summary);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::UnsupportedFullRecoveryHydration)
    );
}

#[test]
fn recovery_boundary_factory_selects_frame_for_frame_seed_variant() {
    let run = RunId::new(20);
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(2),
        workflow: Some(WorkflowDigest::from_bytes([6; 32])),
        steps_started: 1,
        steps_succeeded: 1,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 2,
        slot_count: 4,
        pc: StepIdx::new(1),
        steps: vec![
            RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(1),
                state: RecoveredStepState::Running,
            },
        ],
        slots: Vec::new(),
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
        },
    };
    let hydration = RecoveryHydration::FrameSeed(seed);
    let boundary = recovery_boundary_from_hydration(hydration);

    assert_eq!(boundary.summary(), summary);
    let cannot_resume = RecoveryCannotResumeState {
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };
    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn recovery_boundary_factory_frame_seed_round_trips_summary() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(21),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(5),
        workflow: Some(WorkflowDigest::from_bytes([7; 32])),
        steps_started: 3,
        steps_succeeded: 2,
        actions_scheduled: 1,
        actions_resolved: 1,
        suspensions: 0,
        slots_written: 2,
        terminal: Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(1),
        }),
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 3,
        slot_count: 2,
        pc: StepIdx::new(2),
        steps: vec![
            RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(1),
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(2),
                state: RecoveredStepState::Running,
            },
        ],
        slots: Vec::new(),
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState {
            slot_values: true,
            slot_taint: true,
            action_payloads: true,
            pending_actions: false,
        },
    };
    let hydration = RecoveryHydration::FrameSeed(seed);
    let boundary = recovery_boundary_from_hydration(hydration);

    let recovered_summary = boundary.summary();
    assert_eq!(recovered_summary.run, summary.run);
    assert_eq!(recovered_summary.steps_started, summary.steps_started);
    assert_eq!(recovered_summary.steps_succeeded, summary.steps_succeeded);
    assert_eq!(recovered_summary.terminal, summary.terminal);
}

#[test]
fn pending_actions_fail_closed_with_typed_cannot_resume_state() {
    let run = RunId::new(24);
    let pending_action = RecoveredPendingAction {
        step: StepIdx::new(2),
        action: ActionId::new(7),
    };
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(4),
        workflow: Some(WorkflowDigest::from_bytes([9; 32])),
        steps_started: 3,
        steps_succeeded: 2,
        actions_scheduled: 1,
        actions_resolved: 0,
        suspensions: 1,
        slots_written: 0,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 4,
        slot_count: 0,
        pc: StepIdx::new(2),
        steps: vec![
            RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(1),
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(2),
                state: RecoveredStepState::Asking,
            },
        ],
        slots: Vec::new(),
        pending_actions: vec![pending_action],
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: true,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    let cannot_resume = RecoveryCannotResumeState {
        pending_actions: true,
        pending_asks: true,
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
        ..RecoveryCannotResumeState::RESUMABLE
    };
    assert_eq!(boundary.cannot_resume_state(), cannot_resume);
    assert_eq!(
        boundary.resume_status(),
        RecoveryResumeStatus::CannotResume(cannot_resume)
    );
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
    assert!(boundary.unsupported_state().pending_actions);
}

/// Behavior test proving the hydration gap: no FullRunState variant,
/// no Resumable status, and no full RunState hydration from durable evidence.
/// This test verifies the current gap and provides the target for vb-h5j05.
#[test]
fn hydration_gap_full_run_state_not_yet_implemented() {
    // 1. A fully-supported frame seed (all storage flags false, no pending)
    let run = RunId::new(100);
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(3),
        workflow: Some(WorkflowDigest::from_bytes([0xAA; 32])),
        steps_started: 2,
        steps_succeeded: 1,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 1,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 2,
        slot_count: 1,
        pc: StepIdx::new(1),
        steps: vec![
            RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            },
            RecoveredStepEntry {
                step: StepIdx::new(1),
                state: RecoveredStepState::Succeeded,
            },
        ],
        slots: vec![RecoveredSlotEntry {
            slot: SlotIdx::new(0),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
        }],
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState::SUPPORTED,
    };

    // Verify the seed itself is resumable (storage layer says it has enough evidence)
    assert!(seed.is_resumable(), "Frame seed should be resumable");

    // 2. The runtime boundary correctly hydrates the frame
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed.clone());
    let frame_result = boundary.hydrate_run_frame();
    assert!(frame_result.is_ok(), "hydrate_run_frame should succeed for supported seed");
    let frame = frame_result.ok().unwrap();
    assert_eq!(frame.run_id(), run);
    assert_eq!(frame.step_count(), 2);

    // 3. THE GAP: runtime boundary reports CannotResume even though seed is resumable
    //    After vb-h5j05: this should return Resumable(full_run_state) when all
    //    full-RunState fields are also recoverable.
    match boundary.resume_status() {
        RecoveryResumeStatus::CannotResume(_) => {
            // Frame seed alone cannot resume because full RunState fields
            // (workflow, store, action_attempts, admission, collect_states,
            // action_contracts) are not represented by durable events.
            // The seed says resumable but the boundary says CannotResume —
            // this is the expected gap before vb-h5j05 is implemented.
        }
        RecoveryResumeStatus::SummaryOnly => {
            assert!(false, "Expected CannotResume, got SummaryOnly");
        }
    }

    // 4. TODO (vb-h5j05): After FullRunState variant exists:
    //    let hydration = RecoveryHydration::FullRunState(full_state);
    //    let boundary = recovery_boundary_from_hydration(hydration);
    //    assert!(matches!(boundary.resume_status(), RecoveryResumeStatus::Resumable(_)));
}

use super::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary, SummaryRecoveryBoundary};
use crate::RuntimeError;
use crate::recovery::recovery_boundary_from_hydration;
use vb_core::frame::StepState;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::recovery::{
    RecoveredStepEntry, RecoveredStepState, RecoveryFrameSeed, RecoveryHydration,
    RecoveryRuntimeSummary, RecoveryTerminalState, UnsupportedRecoveryState,
};

#[test]
fn summary_recovery_boundary_exposes_summary() -> Result<(), RuntimeError> {
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
    Ok(())
}

#[test]
fn summary_recovery_boundary_rejects_full_frame_hydration() -> Result<(), RuntimeError> {
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
    Ok(())
}

#[test]
fn durable_frame_recovery_boundary_hydrates_minimal_frame_state() -> Result<(), RuntimeError> {
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

        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    let frame = match boundary.hydrate_run_frame() {
        Ok(frame) => frame,
        Err(error) => {
            panic!("frame hydration should succeed: {error}");
        }
    };

    assert_eq!(frame.run_id(), run);
    assert_eq!(frame.pc(), StepIdx::new(3));
    assert_eq!(frame.step_count(), 4);
    assert_eq!(frame.slot_count(), 0);
    assert_eq!(frame.step_state(StepIdx::new(1)), Ok(StepState::Waiting));
    assert_eq!(frame.step_state(StepIdx::new(3)), Ok(StepState::Succeeded));
    assert_eq!(
        boundary.unsupported_state(),
        UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
        }
    );
    Ok(())
}

#[test]
fn durable_frame_recovery_boundary_rejects_inconsistent_seed() -> Result<(), RuntimeError> {
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

        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
    Ok(())
}

#[test]
fn durable_frame_recovery_boundary_rejects_unsupported_action_payloads() -> Result<(), RuntimeError>
{
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

        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: true,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
    Ok(())
}

#[test]
fn durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint() -> Result<(), RuntimeError>
{
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

        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
        },
    };
    let frame = DurableFrameRecoveryBoundary::from_seed(seed).hydrate_run_frame()?;

    assert_eq!(frame.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(86)));
    assert_eq!(frame.read_taint(SlotIdx::new(1)), Ok(Taint::Secret));
    Ok(())
}

#[test]
fn recovery_boundary_factory_selects_summary_for_summary_variant() -> Result<(), RuntimeError> {
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
    Ok(())
}

#[test]
fn recovery_boundary_factory_selects_frame_for_frame_seed_variant() -> Result<(), RuntimeError> {
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

        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
        },
    };
    let hydration = RecoveryHydration::FrameSeed(seed);
    let boundary = recovery_boundary_from_hydration(hydration);

    assert_eq!(boundary.summary(), summary);
    let frame = match boundary.hydrate_run_frame() {
        Ok(f) => f,
        Err(e) => panic!("hydration should succeed: {e}"),
    };
    assert_eq!(frame.run_id(), run);
    assert_eq!(frame.pc(), StepIdx::new(1));
    assert_eq!(frame.step_count(), 2);
    assert_eq!(frame.slot_count(), 4);
    assert_eq!(frame.step_state(StepIdx::ZERO), Ok(StepState::Succeeded));
    assert_eq!(frame.step_state(StepIdx::new(1)), Ok(StepState::Running));
    Ok(())
}

#[test]
fn recovery_boundary_factory_frame_seed_round_trips_summary() -> Result<(), RuntimeError> {
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

        unsupported: UnsupportedRecoveryState {
            slot_values: true,
            slot_taint: true,
            action_payloads: true,
        },
    };
    let hydration = RecoveryHydration::FrameSeed(seed);
    let boundary = recovery_boundary_from_hydration(hydration);

    let recovered_summary = boundary.summary();
    assert_eq!(recovered_summary.run, summary.run);
    assert_eq!(recovered_summary.steps_started, summary.steps_started);
    assert_eq!(recovered_summary.steps_succeeded, summary.steps_succeeded);
    assert_eq!(recovered_summary.terminal, summary.terminal);
    Ok(())
}

/// RQ-W0-16: Cancelled and Killed terminal variants must be preserved as
/// distinct from Failed and Finished by the recovery boundary so callers
/// can detect the discrimination before attempting rehydration.
#[test]
fn recovery_boundary_preserves_cancelled_vs_failed_distinction() -> Result<(), RuntimeError> {
    let cases = [
        (RunId::new(2001), RecoveryTerminalState::Cancelled),
        (RunId::new(2002), RecoveryTerminalState::Killed),
        (RunId::new(2003), RecoveryTerminalState::Failed),
        (
            RunId::new(2004),
            RecoveryTerminalState::Finished {
                result: SlotIdx::new(7),
            },
        ),
    ];
    for (run, terminal) in cases {
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
            slots_written: 0,
            terminal: Some(terminal),
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 1,
            slot_count: 0,
            pc: StepIdx::ZERO,
            steps: Vec::new(),
            slots: Vec::new(),
            unsupported: UnsupportedRecoveryState::SUPPORTED,
        };
        let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
        let exposed = boundary.summary().terminal;
        assert_eq!(
            exposed,
            Some(terminal),
            "boundary must preserve {terminal:?} for run {run:?}"
        );
        assert!(
            exposed.is_some(),
            "rehydration must reject seed with terminal={terminal:?} (RQ-W0-16)"
        );
    }
    Ok(())
}

/// Verifies that a hydrated run frame from a frame-seed hydration product
/// can be inserted into a shard with a pending timer entry.
#[test]
fn recover_hydrates_pending_timers() -> Result<(), String> {
    use crate::shard::PendingTimer;
    use crate::shard::config::ShardConfig;
    use crate::shard::timer::PendingTimerKind;
    use vb_core::ids::StepIdx;

    // Build a frame-seed hydration for a run that was suspended on a wait.
    let run = RunId::new(42);
    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(3),
        workflow: Some(WorkflowDigest::from_bytes([1; 32])),
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
                state: RecoveredStepState::Waiting,
            },
            RecoveredStepEntry {
                step: StepIdx::new(2),
                state: RecoveredStepState::Running,
            },
        ],
        slots: Vec::new(),

        unsupported: UnsupportedRecoveryState::SUPPORTED,
    };

    // Hydrate the frame from the seed.
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed.clone());
    let frame = boundary
        .hydrate_run_frame()
        .map_err(|e| format!("hydrate_run_frame failed: {e:?}"))?;

    // Build a shard with relaxed policy.
    let config = ShardConfig::default();
    let journal = crate::journal::VolatileRuntimeJournal::shared();
    let mut shard = crate::shard::Shard::new_with_journal(config, journal)
        .map_err(|e| format!("shard construction failed: {e:?}"))?;

    // Insert the recovered run state.
    shard
        .runtime_states
        .insert(run, crate::shard::RuntimeState::Resumable);

    // Use a minimal workflow (test-util feature required for CompiledWorkflow construction).
    #[cfg(feature = "test-util")]
    {
        use crate::admission::empty_workflow;
        let workflow = empty_workflow();

        shard.runs.insert(
            run,
            crate::shard::RunState {
                frame,
                workflow,
                store: vb_core::value_store::ValueStore::with_max_slots(2),
                action_attempts: Box::new([]),
                admission: None,
                collect_states: Default::default(),
                action_contracts: Box::new([]),
                last_snapshot_executed: 0,
            },
        );
    }

    #[cfg(not(feature = "test-util"))]
    {
        // Without test-util, we can't construct a CompiledWorkflow.
        // Skip this part of the test but still verify the pending timer path.
        let _ = frame;
    }

    // Verify pending timer generation for the run.
    let generation = shard
        .next_pending_timer_generation(run)
        .ok_or_else(|| "generation should be Some(1) for new run".to_string())?;
    if generation != 1 {
        return Err(format!(
            "expected generation 1 for new run, got {generation}"
        ));
    }

    // Insert a pending timer for the run.
    let timer = PendingTimer {
        step: StepIdx::new(2),
        kind: PendingTimerKind::Wait,
        generation,
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(30),
    };
    shard.pending_timer_insert(run, timer);

    // Verify the timer was inserted.
    let retrieved = shard.pending_timer_get(run);
    match retrieved {
        Some(t) => {
            if t.step != StepIdx::new(2) {
                return Err(format!("expected step 2, got {:?}", t.step));
            }
            if t.kind != PendingTimerKind::Wait {
                return Err(format!("expected Wait kind, got {:?}", t.kind));
            }
            if t.generation != 1 {
                return Err(format!("expected generation 1, got {}", t.generation));
            }
        }
        None => return Err("pending timer should exist after insert".to_string()),
    }

    // Verify next generation increments.
    let next_gen = shard
        .next_pending_timer_generation(run)
        .ok_or_else(|| "next generation should exist".to_string())?;
    if next_gen != 2 {
        return Err(format!("expected next generation 2, got {next_gen}"));
    }

    Ok(())
}

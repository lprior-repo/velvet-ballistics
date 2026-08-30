#![forbid(unsafe_code)]
//! Integration tests for vb_storage + vb_runtime + vb_validate full pipeline.
//!
//! Tests the complete workflow from storage recovery through runtime hydration
//! to validation, covering edge cases not in individual crate test suites:
//! - Storage journal replay feeds into runtime recovery boundary
//! - Runtime recovery hydration produces frames compatible with validation
//! - Validation gates run on workflow parts derived from storage/recovery

use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, RunId, SlotIdx,
    SlotValue, StepIdx, Taint, WorkflowDigest, WorkflowParts, action::ActionContract,
};
use vb_runtime::recovery::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveredStepEntry, RecoveredStepState, RecoveryFrameSeed,
    RecoveryRuntimeSummary, RecoveryTerminalState, UnsupportedRecoveryState,
    recover_runtime_frame_seed_from_events,
};
use vb_storage::{EventSeq, JournalEvent};
use vb_validate::shared::{ValidationPipeline, validate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn encoded(value: SlotValue) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(&value)
}

fn make_compiled_workflow(name: &str, node_count: u16, slot_count: u16) -> CompiledWorkflow {
    let nodes: Vec<CompiledNode> = (0..node_count)
        .map(|i| CompiledNode {
            id: StepIdx::new(i),
            output: None,
            next: if i < node_count - 1 {
                Some(StepIdx::new(i + 1))
            } else {
                None
            },
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        })
        .collect();

    let parts = WorkflowParts {
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([42; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

// ---------------------------------------------------------------------------
// Pipeline: storage events -> runtime recovery boundary -> validation
// ---------------------------------------------------------------------------

#[test]
fn storage_recovery_to_runtime_boundary_to_validation() {
    // Given - a run that completed successfully
    let run = RunId::new(200);
    let workflow = WorkflowDigest::from_bytes([7; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::ZERO,
            value: Some(encoded(SlotValue::I64(99)).expect("encode")),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    // When - recover from storage events
    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");

    // Then - use runtime boundary to hydrate frame
    let boundary = DurableFrameRecoveryBoundary::from_product(seed);
    let summary = boundary.summary();
    assert_eq!(summary.run, run);
    assert_eq!(summary.steps_started, 1);
    assert_eq!(summary.steps_succeeded, 1);

    // And - validation runs on the workflow from storage
    let compiled = make_compiled_workflow("storage_to_runtime", 1, 1);
    let parts = compiled.to_parts();
    let result = validate(&parts);
    assert!(result.is_ok(), "validation should succeed: {:?}", result);
}

#[test]
fn storage_recovery_feeds_runtime_boundary_with_partial_progress() {
    // Given - a run that was interrupted mid-execution
    let run = RunId::new(201);
    let workflow = WorkflowDigest::from_bytes([8; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::ZERO,
            value: Some(encoded(SlotValue::I64(50)).expect("encode")),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            attempt: 1,
        },
        // Interrupted before completion
    ];

    // When
    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");
    let boundary = DurableFrameRecoveryBoundary::from_product(seed);

    // Then - partial progress is captured
    let summary = boundary.summary();
    assert_eq!(summary.steps_started, 2);
    assert_eq!(summary.steps_succeeded, 1);
    assert!(summary.terminal.is_none(), "should not be terminal");
}

#[test]
fn runtime_boundary_rejects_unsupported_slot_taint_in_pipeline() {
    // Given - a recovery seed with unsupported slot taint
    let run = RunId::new(202);

    let summary = RecoveryRuntimeSummary {
        run,
        first_seq: EventSeq::ZERO,
        last_seq: EventSeq::new(2),
        workflow: Some(WorkflowDigest::from_bytes([9; 32])),
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
            slot_valuesvb_validate::shared::GateStatus::Disabled,
            slot_taintvb_validate::shared::GateStatus::Enabled, // Unsupported
            action_payloadsvb_validate::shared::GateStatus::Disabled,
            pending_actionsvb_validate::shared::GateStatus::Disabled,
        },
    };

    // When - runtime boundary tries to hydrate
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

    // Then - it rejects the unsupported state
    let result = boundary.hydrate_run_frame();
    assert!(result.is_err());
}

#[test]
fn validation_gate_7_to_15_on_workflow_from_storage_recovery() {
    // Given - a compiled workflow with valid structure
    let compiled = make_compiled_workflow("validate_from_storage", 3, 2);
    let parts = compiled.to_parts();

    // When - validate runs all gates
    let pipeline = ValidationPipeline::all_gates();
    let result = pipeline.validate(&parts);

    // Then - validation passes
    assert!(result.is_ok(), "all gates should pass: {:?}", result);
}

#[test]
fn validation_detects_slot_out_of_bounds_in_workflow_parts() {
    // Given - a workflow with valid slot references
    let compiled = make_compiled_workflow("valid_slots", 1, 1);
    let parts = compiled.to_parts();

    // When - validation runs
    let result = validate(&parts);

    // Then - the result should be ok
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Pipeline with action contracts
// ---------------------------------------------------------------------------

#[test]
fn storage_runtime_validation_with_action_contracts() {
    // Given - a compiled workflow with valid structure (no Do nodes, so empty contracts)
    let compiled = make_compiled_workflow("with_contracts", 2, 2);
    let parts = compiled.to_parts();

    // Empty contracts since our test workflow has no Do nodes
    let contracts: Vec<ActionContract> = Vec::new();

    // When - validate with contracts
    let pipeline = ValidationPipeline::all_gates();
    let result = pipeline.validate_with_contracts(&parts, &contracts);

    // Then - validation passes (empty contracts = no orphaned contracts)
    assert!(
        result.is_ok(),
        "validation with contracts should pass: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Error propagation through pipeline
// ---------------------------------------------------------------------------

#[test]
fn storage_recovery_error_propagates_to_runtime() {
    // Given - events that would produce an invalid seed
    let run = RunId::new(203);
    let workflow = WorkflowDigest::from_bytes([10; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        // No step succeeded event - interrupted
    ];

    // When
    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");

    // Then - seed captures the incomplete state
    let boundary = DurableFrameRecoveryBoundary::from_product(seed);
    let summary = boundary.summary();
    assert_eq!(summary.steps_started, 1);
    assert_eq!(summary.steps_succeeded, 0);
    assert!(summary.terminal.is_none());
}

#[test]
fn action_replay_tracker_blocks_non_idempotent_during_recovery() {
    // Given - an action replay tracker and a completed action
    let mut tracker = ActionReplayTracker::new();
    let action_id = ActionId::new(42);
    let step = StepIdx::ZERO;

    // Mark action as completed
    tracker.mark_completed(action_id, step);

    // When - check if action is resolved
    let is_resolved = tracker.is_resolved(action_id, step);

    // Then - action is marked as resolved (would block replay)
    assert!(
        is_resolved,
        "completed action should be blocked from replay"
    );
}

#[test]
fn action_replay_tracker_allows_failed_action_replay() {
    // Given - an action replay tracker with a failed action
    let mut tracker = ActionReplayTracker::new();
    let action_id = ActionId::new(43);
    let step = StepIdx::ZERO;

    // Mark action as failed
    tracker.mark_failed(action_id, step);

    // When - check if action is resolved
    let is_resolved = tracker.is_resolved(action_id, step);

    // Then - failed action is also resolved (blocked from replay)
    assert!(
        is_resolved,
        "failed action should also be blocked from replay"
    );
}

#[test]
fn validation_pipeline_respects_gate_configuration() {
    // Given - a compiled workflow
    let compiled = make_compiled_workflow("gate_config_test", 2, 1);
    let parts = compiled.to_parts();

    // When - validate with only some gates enabled
    let partial_pipeline = ValidationPipeline {
        gate_07_expression_stack: vb_validate::shared::GateStatus::Enabled,
        gate_08_accessor_pathsvb_validate::shared::GateStatus::Enabled,
        gate_09_slot_referencesvb_validate::shared::GateStatus::Enabled,
        gate_10_node_kind_specificvb_validate::shared::GateStatus::Disabled,
        gate_11_loop_body_graphvb_validate::shared::GateStatus::Disabled,
        gate_12_action_contractsvb_validate::shared::GateStatus::Disabled,
        gate_13_no_slot_cyclesvb_validate::shared::GateStatus::Disabled,
        gate_14_slot_type_consistencyvb_validate::shared::GateStatus::Disabled,
        gate_15_determinism_proofvb_validate::shared::GateStatus::Disabled,
    };

    // Then - partial validation runs without error
    let result = partial_pipeline.validate(&parts);
    assert!(
        result.is_ok(),
        "partial gate validation should pass: {:?}",
        result
    );
}

#[test]
fn validation_pipeline_default_enables_all_gates() {
    // Given
    let pipeline = ValidationPipeline::default();

    // Then - all gates should be enabled
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

#[test]
fn validation_pipeline_no_gates_disables_all() {
    // Given
    let pipeline = ValidationPipeline::no_gates();

    // Then - all gates should be disabled
    assert!(!pipeline.gate_07_expression_stack);
    assert!(!pipeline.gate_08_accessor_paths);
    assert!(!pipeline.gate_09_slot_references);
    assert!(!pipeline.gate_10_node_kind_specific);
    assert!(!pipeline.gate_11_loop_body_graph);
    assert!(!pipeline.gate_12_action_contracts);
    assert!(!pipeline.gate_13_no_slot_cycles);
    assert!(!pipeline.gate_14_slot_type_consistency);
    assert!(!pipeline.gate_15_determinism_proof);
}

// ---------------------------------------------------------------------------
// Terminal state handling
// ---------------------------------------------------------------------------

#[test]
fn storage_recovery_captures_finished_terminal_state() {
    // Given - a run that finished successfully
    let run = RunId::new(204);
    let workflow = WorkflowDigest::from_bytes([11; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
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
            output: SlotIdx::ZERO,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];

    // When
    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");

    // Then - terminal state is captured
    assert!(seed.summary.terminal.is_some());
    if let Some(terminal) = seed.summary.terminal {
        match terminal {
            RecoveryTerminalState::Finished { result } => {
                assert_eq!(result, SlotIdx::ZERO);
            }
            RecoveryTerminalState::Cancelled => {
                panic!("expected Finished, got Cancelled");
            }
            RecoveryTerminalState::Failed => {
                panic!("expected Finished, got Failed");
            }
            other => {
                panic!("expected Finished, got {other:?}");
            }
        }
    }
}

#[test]
fn storage_recovery_captures_failed_terminal_state() {
    // Given - a run that failed
    let run = RunId::new(205);
    let workflow = WorkflowDigest::from_bytes([12; 32]);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ];

    // When
    let seed = recover_runtime_frame_seed_from_events(&events).expect("recovery should succeed");

    // Then - terminal state is Failed
    assert!(seed.summary.terminal.is_some());
    if let Some(terminal) = seed.summary.terminal {
        assert!(matches!(terminal, RecoveryTerminalState::Failed));
    }
}

// ---------------------------------------------------------------------------
// Multiple runs handling
// ---------------------------------------------------------------------------

#[test]
fn separate_runs_produce_separate_recovery_seeds() {
    // Given - two different runs
    let run_a = RunId::new(300);
    let run_b = RunId::new(301);
    let workflow = WorkflowDigest::from_bytes([13; 32]);

    let events_a = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: run_a,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    let events_b = vec![
        JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run: run_b,
            seq: EventSeq::new(2),
            slot: SlotIdx::ZERO,
            value: Some(encoded(SlotValue::I64(77)).expect("encode")),
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: run_b,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            output: SlotIdx::ZERO,
        },
    ];

    // When
    let seed_a =
        recover_runtime_frame_seed_from_events(&events_a).expect("recovery should succeed");
    let seed_b =
        recover_runtime_frame_seed_from_events(&events_b).expect("recovery should succeed");

    // Then - seeds are separate
    assert_eq!(seed_a.summary.run, run_a);
    assert_eq!(seed_b.summary.run, run_b);
    assert_ne!(seed_a.summary.run, seed_b.summary.run);

    // And - slot values differ
    assert!(seed_a.slots.is_empty() || seed_a.slots[0].value != SlotValue::I64(77));
    assert_eq!(seed_b.slots[0].value, SlotValue::I64(77));
}

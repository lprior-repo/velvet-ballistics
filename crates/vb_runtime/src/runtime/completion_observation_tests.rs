use super::*;
use CompletionValueObservation::{Available, Restricted, Unavailable};
use OwnedCompletionScalar as Scalar;
use vb_core::frame::RunFrame;
use vb_core::ids::{BlobId, ConstIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ErrorRouting, NodeContinuation, NodeOutput,
    NodeRouting, ResourceContract, WorkflowParts,
};

fn state(
    run: RunId,
    value: Option<SlotValue>,
    taint: Taint,
) -> Result<RunState, vb_core::WorkflowError> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("completion_observation"),
        digest: vb_core::WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([
            CompiledNode {
                id: StepIdx::ZERO,
                output: NodeOutput::Slot(SlotIdx::ZERO),
                routing: NodeRouting {
                    next: NodeContinuation::Step(StepIdx::new(1)),
                    error: ErrorRouting::None,
                },
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: NodeOutput::Discard,
                routing: NodeRouting {
                    next: NodeContinuation::End,
                    error: ErrorRouting::None,
                },
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(99)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
    })?;
    let mut frame = RunFrame::new(run, StepIdx::new(1), 2, 1)?;
    if let Some(value) = value {
        frame.write_slot_with_taint(SlotIdx::ZERO, value, taint)?;
    }
    Ok(RunState {
        registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
        frame,
        run_attempt: NonZeroU16::MIN,
        workflow,
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: Box::from([0, 0]),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    })
}

#[test]
fn capture_discloses_only_clean_owned_scalars() -> Result<(), vb_core::WorkflowError> {
    let cases = [
        (SlotValue::Null, Taint::Clean, Available(Scalar::Null)),
        (
            SlotValue::Bool(true),
            Taint::Clean,
            Available(Scalar::Bool(true)),
        ),
        (SlotValue::I64(99), Taint::Clean, Available(Scalar::I64(99))),
        (
            SlotValue::F64(FiniteF64::new(1.5)?),
            Taint::Clean,
            Available(Scalar::F64(FiniteF64::new(1.5)?)),
        ),
        (SlotValue::I64(99), Taint::DerivedFromSecret, Restricted),
        (SlotValue::I64(99), Taint::Secret, Restricted),
        (SlotValue::I64(99), Taint::Random, Restricted),
        (SlotValue::I64(99), Taint::TimeDependent, Restricted),
        (
            SlotValue::Symbol(SymbolId::new(3)),
            Taint::Clean,
            Unavailable,
        ),
        (SlotValue::List(ListId::new(3)), Taint::Clean, Unavailable),
        (
            SlotValue::Object(ObjectId::new(3)),
            Taint::Clean,
            Unavailable,
        ),
        (SlotValue::Blob(BlobId::new(3)), Taint::Clean, Unavailable),
    ];
    for (value, taint, expected) in cases {
        let mut capture = CompletionCapture {
            enabled: true,
            ..Default::default()
        };
        capture.capture_terminal(RunId::new(7), &state(RunId::new(7), Some(value), taint)?);
        assert_eq!(
            capture.slot,
            CompletionObservationSlot {
                first: Some(CompletionObservation {
                    run: RunId::new(7),
                    attempt: NonZeroU16::MIN,
                    result: expected,
                }),
                conflict: false,
            },
        );
    }
    let uninitialized = state(RunId::new(7), None, Taint::Clean)?;
    let mut capture = CompletionCapture {
        enabled: true,
        ..Default::default()
    };
    capture.capture_terminal(RunId::new(7), &uninitialized);
    assert_eq!(
        capture.slot.first.map(|receipt| receipt.result),
        Some(Unavailable),
    );
    let mut non_finish = state(RunId::new(7), Some(SlotValue::I64(99)), Taint::Clean)?;
    non_finish.frame.set_pc(StepIdx::ZERO)?;
    capture.reset();
    capture.enabled = true;
    capture.capture_terminal(RunId::new(7), &non_finish);
    assert_eq!(
        capture.slot.first.map(|receipt| receipt.result),
        Some(Unavailable),
    );
    Ok(())
}

#[test]
fn capture_preserves_first_and_conflicts_even_for_same_identity() -> Result<(), String> {
    let mut first_state = state(RunId::new(7), Some(SlotValue::I64(99)), Taint::Clean)
        .map_err(|error| format!("fixture failed: {error:?}"))?;
    let mut capture = CompletionCapture::default();
    capture.capture_terminal(RunId::new(7), &first_state);
    assert!(!capture.enabled);
    assert_eq!(capture.slot, CompletionObservationSlot::default());
    capture.enabled = true;
    capture.capture_terminal(RunId::new(7), &first_state);
    let first = Some(CompletionObservation {
        run: RunId::new(7),
        attempt: NonZeroU16::MIN,
        result: Available(Scalar::I64(99)),
    });
    assert_eq!(
        capture.slot,
        CompletionObservationSlot {
            first,
            conflict: false,
        },
    );
    first_state
        .frame
        .write_slot(SlotIdx::ZERO, SlotValue::I64(42))
        .map_err(|error| format!("write failed: {error:?}"))?;
    capture.capture_terminal(RunId::new(7), &first_state);
    assert_eq!(
        capture.slot,
        CompletionObservationSlot {
            first,
            conflict: true,
        },
    );
    let mut second_state = state(RunId::new(8), Some(SlotValue::I64(42)), Taint::Clean)
        .map_err(|error| format!("second fixture failed: {error:?}"))?;
    second_state.run_attempt = NonZeroU16::new(2).ok_or("invalid literal attempt")?;
    capture.capture_terminal(RunId::new(8), &second_state);
    assert_eq!(
        capture.slot,
        CompletionObservationSlot {
            first,
            conflict: true,
        },
    );
    capture.reset();
    assert!(!capture.enabled);
    assert_eq!(capture.slot, CompletionObservationSlot::default());
    capture.enabled = true;
    capture.capture_terminal(RunId::new(8), &second_state);
    assert_eq!(
        capture.slot.first,
        Some(CompletionObservation {
            run: RunId::new(8),
            attempt: NonZeroU16::new(2).ok_or("invalid literal attempt")?,
            result: Available(Scalar::I64(42)),
        }),
    );
    Ok(())
}

#[test]
fn observed_shard_resets_after_real_resume_error_and_finish() -> Result<(), String> {
    use crate::shard::{Shard, ShardCommand, ShardConfig};
    let mut shard = Shard::new(ShardConfig {
        policy: vb_core::RuntimePolicy::Relaxed,
        max_active_runs: 1,
        ..ShardConfig::default()
    });
    let enqueue_result = shard.enqueue(ShardCommand::Resume { run: RunId::new(7) });
    enqueue_result.map_err(|error| format!("resume enqueue failed: {error:?}"))?;
    assert_eq!(
        shard.tick_observed(),
        (
            Err(crate::RuntimeError::RunNotFound),
            CompletionObservationSlot::default(),
        ),
    );
    assert!(!shard.completion_capture.enabled);
    assert_eq!(
        shard.completion_capture.slot,
        CompletionObservationSlot::default(),
    );
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.completion_capture.enabled);
    assert_eq!(
        shard.completion_capture.slot,
        CompletionObservationSlot::default(),
    );
    let fixture = state(RunId::new(7), None, Taint::Clean);
    let workflow = fixture
        .map_err(|error| format!("fixture failed: {error:?}"))?
        .workflow;
    let enqueue_result = shard.enqueue(ShardCommand::Submit {
        run: RunId::new(7),
        workflow: workflow.clone(),
        caps: vb_core::CapabilitySet::empty(),
    });
    enqueue_result.map_err(|error| format!("observed submit enqueue failed: {error:?}"))?;
    assert_eq!(
        shard.tick_observed(),
        (
            Ok(true),
            CompletionObservationSlot {
                first: Some(CompletionObservation {
                    run: RunId::new(7),
                    attempt: NonZeroU16::MIN,
                    result: Available(Scalar::I64(99)),
                }),
                conflict: false,
            },
        ),
    );
    assert!(!shard.completion_capture.enabled);
    assert_eq!(
        shard.completion_capture.slot,
        CompletionObservationSlot::default(),
    );
    let enqueue_result = shard.enqueue(ShardCommand::Submit {
        run: RunId::new(8),
        workflow,
        caps: vb_core::CapabilitySet::empty(),
    });
    enqueue_result.map_err(|error| format!("ordinary submit enqueue failed: {error:?}"))?;
    assert_eq!(shard.tick(), Ok(true));
    assert!(!shard.completion_capture.enabled);
    assert_eq!(
        shard.completion_capture.slot,
        CompletionObservationSlot::default(),
    );
    Ok(())
}

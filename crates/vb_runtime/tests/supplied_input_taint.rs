#![forbid(unsafe_code)]
//! Live supplied-input seeding; no durability or owned-arena qualification.

use std::error::Error;
use std::num::NonZeroUsize;
use vb_core::frame::RunFrame;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts, node,
};
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_runtime::shard::helpers::seed_input_slots;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn workflow() -> TestResult<CompiledWorkflow> {
    let mut secret = CompiledInputConstraint::required();
    secret.secret = true;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: "supplied-input-taint".into(),
        digest: WorkflowDigest::from_bytes([0x7a; 32]),
        nodes: Box::new([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(1),
                    timeout_slot: None,
                },
                Some(SlotIdx::new(4)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(4),
                },
                Some(SlotIdx::new(4)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(4),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        // Declarations stay canonical; supplied values deliberately use another order.
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::ZERO,
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::required(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(3),
                kind: InputSlotKind::I64,
                constraint: secret,
            },
        ]),
    };
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn runtime() -> TestResult<Runtime> {
    let shards = NonZeroUsize::new(1).ok_or("nonzero shard count")?;
    let config = ShardConfig::new(4, 16, 16, 1, RuntimePolicy::Relaxed)?;
    Ok(Runtime::new_for_tests_and_benchmarks_only(shards, config))
}

#[test]
fn supplied_taints_follow_matching_declarations_in_retained_runtime_frame() -> TestResult {
    let mut runtime = runtime()?;
    let run = RunId::new(8101);
    runtime.submit_compiled_with_inputs(
        run,
        workflow()?,
        Box::new([
            (SlotIdx::new(3), SlotValue::I64(99)),
            // Undeclared in-bounds inputs retain their existing clean behavior.
            (SlotIdx::new(1), SlotValue::I64(7)),
            (SlotIdx::ZERO, SlotValue::I64(11)),
        ]),
    )?;
    assert!(runtime.tick_all()?);
    assert!(
        runtime
            .lookup_pending_ask_ticket(run, StepIdx::ZERO)
            .is_some()
    );
    let frame = &runtime.inspect_run_state(run)?.frame;
    assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(11));
    assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Clean);
    assert_eq!(frame.read_slot(SlotIdx::new(1))?, &SlotValue::I64(7));
    assert_eq!(frame.read_taint(SlotIdx::new(1))?, Taint::Clean);
    assert_eq!(frame.read_slot(SlotIdx::new(3))?, &SlotValue::I64(99));
    assert_eq!(frame.read_taint(SlotIdx::new(3))?, Taint::Secret);
    Ok(())
}

#[test]
fn wrong_supplied_kind_keeps_exact_runtime_refusal() -> TestResult {
    let mut runtime = runtime()?;
    let run = RunId::new(8102);
    runtime.submit_compiled_with_inputs(
        run,
        workflow()?,
        Box::new([
            (SlotIdx::ZERO, SlotValue::I64(11)),
            (SlotIdx::new(1), SlotValue::I64(7)),
            (SlotIdx::new(3), SlotValue::Bool(true)),
        ]),
    )?;
    assert_eq!(
        runtime.tick_all(),
        Err(RuntimeError::RuntimeInputSlotKindMismatch {
            slot: SlotIdx::new(3),
            expected: InputSlotKind::I64,
            actual: InputSlotKind::Bool,
        })
    );
    assert!(matches!(
        runtime.inspect_run_state(run),
        Err(RuntimeError::RunNotFound)
    ));
    Ok(())
}

#[test]
fn undeclared_out_of_range_input_keeps_runtime_refusal() -> TestResult {
    let mut runtime = runtime()?;
    let run = RunId::new(8103);
    runtime.submit_compiled_with_inputs(
        run,
        workflow()?,
        Box::new([
            (SlotIdx::ZERO, SlotValue::I64(11)),
            (SlotIdx::new(1), SlotValue::I64(7)),
            (SlotIdx::new(3), SlotValue::I64(99)),
            (SlotIdx::new(5), SlotValue::Bool(true)),
        ]),
    )?;
    assert_eq!(
        runtime.tick_all(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
    assert!(matches!(
        runtime.inspect_run_state(run),
        Err(RuntimeError::RunNotFound)
    ));
    Ok(())
}

#[test]
fn later_kind_refusal_preserves_earlier_supplied_write() -> TestResult {
    let workflow = workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(8104),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )?;
    assert_eq!(
        seed_input_slots(
            &mut frame,
            &[
                (SlotIdx::ZERO, SlotValue::I64(11)),
                (SlotIdx::new(3), SlotValue::Bool(true)),
            ],
            &workflow,
        ),
        Err(RuntimeError::RuntimeInputSlotKindMismatch {
            slot: SlotIdx::new(3),
            expected: InputSlotKind::I64,
            actual: InputSlotKind::Bool,
        })
    );
    assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(11));
    assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Clean);
    assert!(matches!(
        frame.read_slot(SlotIdx::new(3)),
        Err(vb_core::errors::EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(3)
    ));
    Ok(())
}

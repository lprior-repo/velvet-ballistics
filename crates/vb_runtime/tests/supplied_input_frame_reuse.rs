#![forbid(unsafe_code)]
//! Live frame-pool reuse after input refusal; no durability qualification.

use std::error::Error;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts, node,
};
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_runtime::journal::VolatileRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn workflow() -> TestResult<CompiledWorkflow> {
    let mut secret = CompiledInputConstraint::required();
    secret.secret = true;
    let mut optional_secret = secret;
    optional_secret.optional = true;
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        name: "supplied-input-frame-reuse".into(),
        digest: WorkflowDigest::from_bytes([0x7b; 32]),
        nodes: Box::new([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(1),
                    timeout_slot: None,
                },
                Some(SlotIdx::new(5)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(5),
                },
                Some(SlotIdx::new(5)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(5),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 6,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::ZERO,
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::required(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
                constraint: secret,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(3),
                kind: InputSlotKind::I64,
                constraint: optional_secret,
            },
        ]),
    })?)
}

fn assert_refusal_preserves_capacity(
    invalid: (SlotIdx, SlotValue),
    expected: RuntimeError,
    reuse_same_run: bool,
) -> TestResult {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let config = ShardConfig::new(4, 32, 16, 1, RuntimePolicy::Relaxed)?;
    let shards = NonZeroUsize::new(1).ok_or("nonzero shard count")?;
    let mut runtime = Runtime::new(shards, config, journal.clone());
    let workflow = workflow()?;
    let rejected_run = RunId::new(8201);
    runtime.submit_compiled_with_inputs(
        rejected_run,
        workflow.clone(),
        Box::new([
            (SlotIdx::new(3), SlotValue::I64(99)),
            (SlotIdx::new(4), SlotValue::I64(222)),
            (SlotIdx::new(1), SlotValue::I64(7)),
            invalid,
        ]),
    )?;
    assert_eq!(runtime.tick_all(), Err(expected));
    assert!(matches!(
        runtime.inspect_run_state(rejected_run),
        Err(RuntimeError::RunNotFound)
    ));
    assert!(
        runtime
            .lookup_pending_ask_ticket(rejected_run, StepIdx::ZERO)
            .is_none()
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    assert!(journal.snapshot()?.is_empty());
    // This fresh runtime has not reached submission tracing on this error path.
    assert!(runtime.drain_trace().is_empty());

    let run = if reuse_same_run {
        rejected_run
    } else {
        RunId::new(8202)
    };
    runtime.submit_compiled_with_inputs(
        run,
        workflow,
        Box::new([
            (SlotIdx::new(2), SlotValue::I64(199)),
            (SlotIdx::new(1), SlotValue::I64(7)),
            (SlotIdx::ZERO, SlotValue::I64(11)),
        ]),
    )?;
    assert!(runtime.tick_all()?);
    let ticket = runtime
        .lookup_pending_ask_ticket(run, StepIdx::ZERO)
        .ok_or("valid submission must reach the untimed Ask")?;
    assert_eq!(ticket.run(), run);
    assert_eq!(ticket.ask_step(), StepIdx::ZERO);
    let frame = &runtime.inspect_run_state(run)?.frame;
    assert_eq!(frame.run_id(), run);
    assert_eq!(
        frame.slots_snapshot(),
        vec![
            Some(SlotValue::I64(11)),
            Some(SlotValue::I64(7)),
            Some(SlotValue::I64(199)),
            None,
            None,
            None,
        ]
    );
    assert_eq!(
        frame.taint_snapshot(),
        vec![
            Taint::Clean,
            Taint::Clean,
            Taint::Secret,
            Taint::Clean,
            Taint::Clean,
            Taint::Clean,
        ]
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn kind_refusal_reclaims_the_only_frame_for_the_same_run() -> TestResult {
    assert_refusal_preserves_capacity(
        (SlotIdx::ZERO, SlotValue::Bool(true)),
        RuntimeError::RuntimeInputSlotKindMismatch {
            slot: SlotIdx::ZERO,
            expected: InputSlotKind::I64,
            actual: InputSlotKind::Bool,
        },
        true,
    )
}

#[test]
fn out_of_range_refusal_reclaims_the_only_frame_for_another_run() -> TestResult {
    assert_refusal_preserves_capacity(
        (SlotIdx::new(6), SlotValue::Bool(true)),
        RuntimeError::InvalidRecoveryHydration,
        false,
    )
}

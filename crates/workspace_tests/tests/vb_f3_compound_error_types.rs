//! Error-only readers use the actual failed compound's slot prestate.
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, ExprOp, ExprProgram, InputSlotKind,
    ResourceContract, WorkflowError, node, node_with_handler,
};
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, ConstIdx, ConstValue, ExprIdx, RunId, RuntimePolicy,
    SlotIdx, SlotValue, StepIdx, WorkflowDigest, WorkflowParts,
};
use vb_runtime::{
    journal::{RuntimeJournalEvent, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "vb_f3_compound_error_types/counter_boundary.rs"]
mod counter_boundary;
#[path = "vb_f3_compound_error_types/late_writes.rs"]
mod late_writes;
#[path = "vb_f3_compound_error_types/siblings.rs"]
mod siblings;

fn fixture(numeric: bool, external: bool) -> Result<WorkflowParts, WorkflowError> {
    let items = SlotIdx::new(0);
    let item = SlotIdx::new(1);
    let cursor = SlotIdx::new(2);
    let result = SlotIdx::new(3);
    let recovered = SlotIdx::new(4);
    let start = if external { 1 } else { 2 };
    let body = StepIdx::new(start + 1);
    let control = StepIdx::new(start + 2);
    let done = StepIdx::new(start + 3);
    let normal_finish = StepIdx::new(start + 4);
    let handler = StepIdx::new(start + 5);
    let handler_finish = StepIdx::new(start + 6);
    let mut nodes = vec![node(
        StepIdx::ZERO,
        K::SetConst {
            value: ConstIdx::new(0),
        },
        Some(cursor),
        Some(StepIdx::new(1)),
    )];
    if !external {
        nodes.push(node(
            StepIdx::new(1),
            K::BuildList {
                items: Box::from([cursor, cursor]),
            },
            Some(items),
            Some(StepIdx::new(start)),
        ));
    }
    nodes.extend([
        node_with_handler(
            StepIdx::new(start),
            K::ForEachStart {
                input: items,
                item_slot: item,
                limit: 1.into(),
                body,
                done,
            },
            Some(cursor),
            None,
            handler,
            None,
        ),
        node(body, K::Nop, None, Some(control)),
        node(
            control,
            K::ForEachNext {
                iterator_slot: cursor,
                body,
                done,
            },
            Some(item),
            None,
        ),
        node(
            done,
            K::ForEachJoin { output: cursor },
            Some(result),
            Some(normal_finish),
        ),
        node(normal_finish, K::Finish { result }, None, None),
        node(
            handler,
            if numeric {
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                }
            } else {
                K::Copy { source: cursor }
            },
            Some(recovered),
            Some(handler_finish),
        ),
        node(handler_finish, K::Finish { result: recovered }, None, None),
    ]);
    let mut constraint = CompiledInputConstraint::required();
    constraint.max = Some(2);
    constraint.element = Some(InputSlotKind::I64);
    Ok(WorkflowParts {
        name: "compound-error-types".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([ExprProgram::try_from_ops(Box::from([
            ExprOp::LoadSlot(cursor),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ]))?]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(42), ConstValue::I64(0)]),
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: if external {
            Box::from([CompiledInputSlot {
                slot: items,
                kind: InputSlotKind::List,
                constraint,
            }])
        } else {
            Box::new([])
        },
    })
}

#[test]
fn public_constructor_accepts_numeric_handler_with_preinitialized_cursor() -> TestResult {
    let parts = fixture(true, false)?;
    let construction = CompiledWorkflow::try_from_parts(parts.clone());
    assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
    vb_validate::shared::validate(&parts)?;
    Ok(())
}

#[test]
fn public_constructor_accepts_declared_input_with_preinitialized_cursor() -> TestResult {
    let parts = fixture(true, true)?;
    let construction = CompiledWorkflow::try_from_parts(parts.clone());
    assert_eq!(construction.as_ref().map(|_| ()), Ok(()));
    vb_validate::shared::validate(&parts)?;
    Ok(())
}

#[test]
fn public_constructor_still_enforces_active_cursor_ownership() -> TestResult {
    let mut parts = fixture(false, true)?;
    parts.nodes.get_mut(2).ok_or("body")?.kind = K::Copy {
        source: SlotIdx::ZERO,
    };
    parts.nodes.get_mut(2).ok_or("body")?.output = Some(SlotIdx::new(2)).into();
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).as_ref().map(|_| ()),
        Err(&WorkflowError::LoopCursorForeignWrite {
            owner: StepIdx::new(1),
            writer: StepIdx::new(2),
            slot: SlotIdx::new(2),
        })
    );
    Ok(())
}

fn runtime_recovers(numeric: bool) -> TestResult {
    execute_recovery(fixture(numeric, false)?, 2, SlotValue::I64(42))
}

fn execute_recovery(parts: WorkflowParts, failed: u16, expected: SlotValue) -> TestResult {
    execute_recovery_with_cursor_writes(parts, failed, expected, 1)
}

fn execute_recovery_with_cursor_writes(
    parts: WorkflowParts,
    failed: u16,
    expected: SlotValue,
    cursor_writes: usize,
) -> TestResult {
    let last = parts.nodes.last().ok_or("finish")?;
    let K::Finish { result: recovered } = last.kind else {
        return Err("final Finish".into());
    };
    let plan = CompiledWorkflow::try_from_parts(parts.clone())?;
    vb_validate::shared::validate(&parts)?;
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            ..ShardConfig::default()
        },
        journal.clone(),
    );
    let run = RunId::new(42);
    runtime.submit_direct(run, plan)?;
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    let events = journal.snapshot()?;
    assert!(events.contains(&RuntimeJournalEvent::StepFailed {
        run,
        step: StepIdx::new(failed),
        attempt: 1
    }));
    assert!(events.contains(&RuntimeJournalEvent::SlotWritten {
        run,
        slot: recovered,
        value: postcard::to_allocvec(&expected)?,
        taint: vb_core::Taint::Clean,
        extra: None,
    }));
    assert_eq!(events.iter().filter(|event| matches!(event, RuntimeJournalEvent::SlotWritten { slot, .. } if *slot == recovered)).count(), 1);
    assert_eq!(events.iter().filter(|event| matches!(event, RuntimeJournalEvent::SlotWritten { slot, .. } if *slot == SlotIdx::new(2))).count(), cursor_writes);
    assert_eq!(events.iter().filter(|event| matches!(event, RuntimeJournalEvent::StepFailed { step, .. } if *step == StepIdx::new(failed))).count(), 1);
    assert!(!events.iter().any(|event| matches!(event, RuntimeJournalEvent::StepSucceeded { step, .. } if *step == StepIdx::new(failed))));
    Ok(())
}

#[test]
fn public_runtime_copy_control_preserves_42_on_failed_start() -> TestResult {
    runtime_recovers(false)
}

#[test]
fn public_runtime_numeric_handler_preserves_42_on_failed_start() -> TestResult {
    runtime_recovers(true)
}

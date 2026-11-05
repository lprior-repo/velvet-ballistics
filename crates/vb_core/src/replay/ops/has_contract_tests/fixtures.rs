use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::replay::{ReplayError, ReplayExprStack};
use crate::value::{SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, node,
};

pub(super) type TestResult = Result<(), CoreError>;

pub(super) fn checked<T>(result: Result<T, ReplayError>) -> Result<T, CoreError> {
    result.map_err(|_| CoreError::InternalInvariantViolation {
        reason: "replay Has test setup or expected-success operation failed",
    })
}

pub(super) fn unsupported() -> ReplayError {
    ReplayError::Internal {
        reason: "unsupported expression op for replay",
    }
}

pub(super) fn stack(capacity: u8, values: &[SlotValue]) -> Result<ReplayExprStack, CoreError> {
    let mut stack = checked(ReplayExprStack::new(capacity))?;
    // Populate and pop through the real API, retaining non-null inactive cells.
    for _ in 0..capacity {
        checked(stack.push(SlotValue::I64(-91)))?;
    }
    for _ in 0..capacity {
        checked(stack.pop())?;
    }
    for value in values {
        checked(stack.push(*value))?;
    }
    Ok(stack)
}

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Eval,
    Choose,
}

pub(super) fn plan(mode: Mode) -> Result<CompiledWorkflow, CoreError> {
    let (kind, output, next) = match mode {
        Mode::Eval => (
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
            Some(SlotIdx::new(2)),
            Some(StepIdx::new(2)),
        ),
        Mode::Choose => (
            CompiledNodeKind::Choose {
                branches: vec![ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(2),
                }]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(3)),
            },
            None,
            None,
        ),
    };
    let expression = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Has,
        ]
        .into_boxed_slice(),
    )?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "replay_has_contract".into(),
        digest: WorkflowDigest::from_bytes([23; 32]),
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(StepIdx::new(1), kind, output, next),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(3)),
            ),
            node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "replay Has test plan",
    })
}

pub(super) fn frame(
    left: SlotValue,
    right: SlotValue,
    taint: Taint,
) -> Result<RunFrame, CoreError> {
    let mut frame = RunFrame::new(RunId::new(71), StepIdx::new(1), 4, 3)?;
    frame.write_slot_with_taint(SlotIdx::new(0), left, Taint::Clean)?;
    frame.write_slot_with_taint(SlotIdx::new(1), right, taint)?;
    frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(99), Taint::Clean)?;
    Ok(frame)
}

pub(super) fn object_fixture() -> Result<(ValueStore, SlotValue, SymbolId, SymbolId), CoreError> {
    let mut store = ValueStore::new();
    let present = store.insert_symbol("present")?;
    let missing = store.insert_symbol("missing")?;
    let object = store.insert_object(
        vec![ObjectField::with_taint(
            present,
            SlotValue::Bool(false),
            Taint::Secret,
        )]
        .into_boxed_slice(),
    )?;
    Ok((store, SlotValue::Object(object), present, missing))
}

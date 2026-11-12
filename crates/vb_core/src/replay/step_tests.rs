use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, ListId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::replay::{ReplayError, SuspensionKind, step::ReplayAction};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind, ExprOp,
    ExprProgram, InputSlotKind, ResourceContract, SlotBranch, WorkflowParts,
    check_expr_stack_bound,
};

use super::replay_step;

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(make_parts(nodes, constants, expressions))
        .map_err(|_| CoreError::InvalidCompiledWorkflow {
            reason: "test workflow validation failed",
        })
}

fn make_parts(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
        name: "test_step".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn make_rejected_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    expected: crate::workflow::SlotFlowFailure,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    let parts = make_parts(nodes, constants, expressions);
    if !matches!(
        crate::workflow::CompiledWorkflow::try_from_parts(parts.clone()),
        Err(crate::WorkflowError::SlotFlow(crate::workflow::SlotFlowError { reason, .. }))
            if reason == expected
    ) {
        return Err(CoreError::InvalidCompiledWorkflow {
            reason: "invalid fixture must fail compiled slot validation",
        });
    }
    Ok(crate::workflow::CompiledWorkflow::from_parts_unchecked(
        parts,
    ))
}

fn make_plan_with_symbols(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    symbols_count: u32,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
        name: "test_step".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "test workflow validation failed",
    })
}

fn make_expr_program(ops: Vec<ExprOp>) -> Result<ExprProgram, CoreError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
    ExprProgram::try_from_parts(ops.into(), max_stack)
}

fn replay_err_to_core(e: ReplayError) -> CoreError {
    match e {
        ReplayError::StepNotFound { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::SlotNotAvailable { slot } => CoreError::SlotOutOfBounds { slot },
        ReplayError::ExpressionEvalFailed { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::NonDeterministicStep { step, .. } => CoreError::InvalidProgramCounter { step },
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation {
            reason: reason.into(),
        },
    }
}

include!("step_tests/nop_set_const_copy.rs");
include!("step_tests/eval_expr_build.rs");
include!("step_tests/finish_jump_suspend_choose.rs");
include!("step_tests/multi_field_and_failures.rs");

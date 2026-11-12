#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{
    AccessorIdx, ConstIdx, ExprIdx, ListId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::replay::{ReplayError, ReplayExprStack, eval_expr_for_replay};
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    ResourceContract, WorkflowParts, check_expr_stack_bound, node,
};

use super::{eval_replay_op, pop_i64_pair, pop_pair};

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "test_ops".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: accessors.into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 1,
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

fn make_frame(slot_count: u16) -> Result<RunFrame, CoreError> {
    RunFrame::new(RunId::new(0), StepIdx::new(0), 2, slot_count)
}

fn make_minimal_plan_with_constants(
    constants: Vec<ConstValue>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        constants,
        vec![],
        vec![],
    )
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

// ---- ReplayExprStack lifecycle and stack helpers ----
include!("tests_stack.rs");

// ---- LoadSlot / LoadConst / LoadAccessor and expr_for_replay ----
include!("tests_expr.rs");
include!("has_parity_replay_tests.rs");

// ---- Mixed I64/F64 arithmetic and ce003 taint join tests ----
include!("tests_ce003.rs");

include!("has_error_replay_tests.rs");
include!("has_bytecode_boundary_tests.rs");
include!("has_bytecode_boundary_hot_tests.rs");
include!("has_bytecode_boundary_replay_tests.rs");
include!("contains_replay_parity_tests.rs");
include!("contains_replay_workflow_tests.rs");

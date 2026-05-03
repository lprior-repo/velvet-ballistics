//! Tests for the replay module.

use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::ConstValue;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, ResourceContract, WorkflowParts,
    check_expr_stack_bound,
};

use super::{ReplayEngine, ReplayError};

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "test_replay".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 0,
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
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation { reason },
    }
}

#[test]
fn replay_linear_setconst_finish() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(42)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    let result = engine
        .replay_up_to(StepIdx::new(1), &mut store)
        .map_err(replay_err_to_core)?;
    if result != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "expected step 1",
        });
    }
    Ok(())
}

#[test]
fn replay_stops_at_action() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: crate::ids::ActionId::new(0),
                    input: SlotIdx::new(0),
                },
                output: None,
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(10)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    match engine.replay_up_to(StepIdx::new(2), &mut store) {
        Err(ReplayError::NonDeterministicStep { step, kind }) => {
            if step != StepIdx::new(1) || kind != "Do" {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "expected Do at step 1",
                });
            }
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected NonDeterministicStep for Do",
        }),
    }
}

#[test]
fn replay_stops_at_ask() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
                output: None,
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(5)],
        vec![],
    )?;

    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);
    match engine.replay_up_to(StepIdx::new(2), &mut store) {
        Err(ReplayError::NonDeterministicStep { step, kind }) => {
            if step != StepIdx::new(1) || kind != "Ask" {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "expected Ask at step 1",
                });
            }
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected NonDeterministicStep for Ask",
        }),
    }
}

#[test]
fn replay_reconstructs_slots() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(100), ConstValue::I64(200)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    super::step::replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be 100",
        });
    }

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    super::step::replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 should be 100",
        });
    }

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(200) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be 200",
        });
    }

    Ok(())
}

#[test]
fn replay_expression_eval() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                output: Some(SlotIdx::new(2)),
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(30), ConstValue::I64(12)],
        vec![expr],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    super::step::replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    super::step::replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    super::step::replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be 42",
        });
    }

    Ok(())
}

#[test]
fn replay_step_not_found() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let engine = ReplayEngine::new(&plan);

    match engine.replay_up_to(StepIdx::new(99), &mut store) {
        Err(ReplayError::StepNotFound { step }) => {
            if step != StepIdx::new(99) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "expected step 99",
                });
            }
            Ok(())
        }
        Err(other) => Err(replay_err_to_core(other)),
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "expected StepNotFound",
        }),
    }
}

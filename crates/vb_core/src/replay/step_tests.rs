#![allow(
    clippy::panic_in_result_fn,
    clippy::panic,
    clippy::expect_used,
    clippy::ok_expect,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::redundant_guards
)]
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
    CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, ResourceContract, SlotBranch,
    WorkflowParts, check_expr_stack_bound,
};

use super::replay_step;

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
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
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "test workflow validation failed",
    })
}

fn make_plan_with_symbols(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    symbols_count: u32,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
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
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation { reason },
    }
}

// ---- Nop step ----

#[test]
fn replay_nop_advances_to_next() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
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
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Nop should return Continue(1)",
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1",
        });
    }
    if run.executed() != 1 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 1",
        });
    }
    Ok(())
}

#[test]
fn replay_nop_missing_next_returns_error() -> Result<(), CoreError> {
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

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "Nop node missing next step"
            })
        ),
        "Nop without next must fail"
    );
    Ok(())
}

// ---- SetConst step ----

#[test]
fn replay_set_const_writes_slot() -> Result<(), CoreError> {
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

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be I64(42) after SetConst",
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_bool() -> Result<(), CoreError> {
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
        vec![ConstValue::Bool(true)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be Bool(true)",
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_null() -> Result<(), CoreError> {
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
        vec![ConstValue::Null],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::Null {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be Null",
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_missing_output_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                output: None,
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
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "SetConst node missing output slot"
            })
        ),
        "SetConst without output must fail"
    );
    Ok(())
}

// ---- Copy step ----

#[test]
fn replay_copy_transfers_value_and_taint() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(100)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 should be I64(100)",
        });
    }
    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 taint should be Secret",
        });
    }
    Ok(())
}

#[test]
fn replay_copy_clean_source_has_clean_taint() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::Clean {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 taint should be Clean",
        });
    }
    Ok(())
}

#[test]
fn replay_copy_uninitialized_source_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(3),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        matches!(result, Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(3)),
        "Copy from uninitialized slot must fail with SlotNotAvailable"
    );
    Ok(())
}

#[test]
fn replay_copy_missing_output_returns_error() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let result = replay_step(node1, &mut run, &mut store, &plan);
    assert!(
        matches!(
            result,
            Err(ReplayError::Internal {
                reason: "Copy node missing output slot"
            })
        ),
        "Copy without output must fail"
    );
    Ok(())
}

// ---- EvalExpr step ----

#[test]
fn replay_eval_expr_computes_result() -> Result<(), CoreError> {
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
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be I64(42)",
        });
    }
    Ok(())
}

// ---- BuildObject step ----

#[test]
fn replay_build_object_creates_handle() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
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
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(42)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            let field = obj.first().ok_or(CoreError::InternalInvariantViolation {
                reason: "object should have a field",
            })?;
            if field.key != field_sym || field.value != SlotValue::I64(42) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field mismatch",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 1 should be Object",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_empty_fields() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![].into(),
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
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            if !obj.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildObject should create empty object",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be Object",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_uninitialized_field_returns_error() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(5))].into(),
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
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "BuildObject with uninitialized field must fail"
    );
    Ok(())
}

#[test]
fn replay_build_object_propagates_taint() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
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
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(1)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::DerivedFromSecret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::DerivedFromSecret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject output taint should be DerivedFromSecret",
        });
    }
    Ok(())
}

// ---- BuildList step ----

#[test]
fn replay_build_list_creates_handle() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into(),
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
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if list.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list should have 2 items",
                });
            }
            if list[0] != SlotValue::I64(10) || list[1] != SlotValue::I64(20) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list items mismatch",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be List",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_empty_items() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![].into(),
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
        vec![],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if !list.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildList should create empty list",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be List",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_uninitialized_item_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(5)].into(),
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
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "BuildList with uninitialized item must fail"
    );
    Ok(())
}

#[test]
fn replay_build_list_propagates_taint() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into(),
                },
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(7)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildList output taint should be Secret",
        });
    }
    Ok(())
}

// ---- Finish step ----

#[test]
fn replay_finish_returns_finished_action() -> Result<(), CoreError> {
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
        vec![ConstValue::I64(99)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Finished => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Finish should return Finished",
            });
        }
    }
    if run.executed() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 2",
        });
    }
    Ok(())
}

#[test]
fn replay_finish_uninitialized_result_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(5),
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let result = replay_step(node, &mut run, &mut store, &plan);
    assert!(
        result.is_err(),
        "Finish with uninitialized result must fail"
    );
    Ok(())
}

// ---- Jump step ----

#[test]
fn replay_jump_advances_pc_to_target() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
                output: None,
                next: None,
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
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Jump should return Continue(1)",
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1",
        });
    }
    Ok(())
}

// ---- Suspend steps ----

#[test]
fn replay_do_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::ActionPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Do should return Suspended(0, Do)",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_ask_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::AskPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Ask should return Suspended(0, Ask)",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_until_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitUntil => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitUntil should return Suspended(0, WaitUntil)",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_event_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![CompiledNode {
            id: StepIdx::new(0),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
            output: None,
            next: None,
        }],
        vec![],
        vec![],
    )?;

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();

    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitEvent => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitEvent should return Suspended(0, WaitEvent)",
            });
        }
    }
    Ok(())
}

// ---- ChooseSlot step ----

#[test]
fn replay_choose_slot_true_branch_taken() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into(),
                    otherwise: Some(StepIdx::new(3)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::Bool(true)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(2) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ChooseSlot true should go to step 2",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_choose_slot_false_falls_to_otherwise() -> Result<(), CoreError> {
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
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into(),
                    otherwise: Some(StepIdx::new(3)),
                },
                output: None,
                next: None,
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
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::Bool(false)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(3) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ChooseSlot false should go to otherwise (step 3)",
            });
        }
    }
    Ok(())
}

// ---- Multi-step counter ----

#[test]
fn replay_multi_step_executed_counter() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: Some(StepIdx::new(3)),
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                output: None,
                next: None,
            },
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node3 = plan
        .node(StepIdx::new(3))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 3 missing",
        })?;
    replay_step(node3, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.executed() != 4 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed counter should be 4",
        });
    }
    Ok(())
}

// ---- BuildObject multiple fields ----

#[test]
fn replay_build_object_multiple_fields_preserves_order() -> Result<(), CoreError> {
    let sym_a = SymbolId::new(0);
    let sym_b = SymbolId::new(1);
    let plan = make_plan_with_symbols(
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
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(sym_a, SlotIdx::new(0)), (sym_b, SlotIdx::new(1))].into(),
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
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        vec![],
        2,
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    let mut store = ValueStore::new();

    let node0 = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing",
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::Object(id) => {
            let fields = store.object(id)?;
            if fields.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "object should have 2 fields",
                });
            }
            if fields[0].key != sym_a
                || fields[0].value != SlotValue::I64(10)
                || fields[1].key != sym_b
                || fields[1].value != SlotValue::I64(20)
            {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field order or values wrong",
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be Object",
            });
        }
    }
    Ok(())
}

#[test]
fn replay_collect_next_reports_missing_source_list() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
        ],
        vec![],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let current_page = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "current page insert failed",
        })?;
    let mut run = RunFrame::new(
        RunId::new(91),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(current_page))?;
    let mut states = super::ReplayCollectStates::new();
    states.upsert(
        SlotIdx::new(0),
        super::ReplayCollectState {
            source: ListId::new(99),
            current_page,
            cursor: 1,
            page_size: 1,
            item_count: 1,
            taint: Taint::Clean,
        },
    );
    let node = plan
        .node(StepIdx::new(0))
        .ok_or(CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        })?;

    match super::replay_step_with_collect(node, &mut run, &mut store, &plan, &mut states) {
        Err(ReplayError::Internal { reason })
            if reason == "collect source list missing during replay" =>
        {
            Ok(())
        }
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "CollectNext should report missing source list",
        }),
    }
}

#[test]
fn replay_copy_reports_source_slot_failures() -> Result<(), CoreError> {
    let node = CompiledNode {
        id: StepIdx::new(0),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(7),
        },
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
    };
    let mut run = RunFrame::new(RunId::new(92), StepIdx::new(0), 2, 1)?;
    match super::replay_copy(&mut run, &node, SlotIdx::new(7)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Copy should report out-of-bounds source slot",
            });
        }
    }
    match super::replay_copy(&mut run, &node, SlotIdx::new(0)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Copy should report uninitialized source slot",
        }),
    }
}

#[test]
fn replay_build_object_reports_field_slot_failures() -> Result<(), CoreError> {
    let node = CompiledNode {
        id: StepIdx::new(0),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildObject {
            fields: vec![].into(),
        },
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
    };
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(93), StepIdx::new(0), 2, 1)?;
    let fields = [(SymbolId::new(0), SlotIdx::new(7))];
    match super::replay_build_object(&mut run, &mut store, &node, &fields) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BuildObject should report out-of-bounds field slot",
            });
        }
    }
    let fields = [(SymbolId::new(0), SlotIdx::new(0))];
    match super::replay_build_object(&mut run, &mut store, &node, &fields) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject should report uninitialized field slot",
        }),
    }
}

#[test]
fn replay_build_list_reports_item_slot_failures() -> Result<(), CoreError> {
    let node = CompiledNode {
        id: StepIdx::new(0),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildList {
            items: vec![].into(),
        },
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
    };
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(94), StepIdx::new(0), 2, 1)?;
    match super::replay_build_list(&mut run, &mut store, &node, &[SlotIdx::new(7)]) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BuildList should report out-of-bounds item slot",
            });
        }
    }
    match super::replay_build_list(&mut run, &mut store, &node, &[SlotIdx::new(0)]) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "BuildList should report uninitialized item slot",
        }),
    }
}

#[test]
fn replay_finish_reports_result_slot_failures() -> Result<(), CoreError> {
    let mut run = RunFrame::new(RunId::new(95), StepIdx::new(0), 1, 1)?;
    match super::replay_finish(&mut run, SlotIdx::new(7)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Finish should report out-of-bounds result slot",
            });
        }
    }
    match super::replay_finish(&mut run, SlotIdx::new(0)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Finish should report uninitialized result slot",
        }),
    }
}

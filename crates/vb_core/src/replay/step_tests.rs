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

// ---- Nop step ----

#[test]
fn replay_nop_advances_to_next() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Nop should return Continue(1)".into(),
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1".into(),
        });
    }
    if run.executed() != 1 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 1".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_nop_missing_next_returns_error() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )],
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
            reason: "node 0 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be I64(42) after SetConst".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_bool() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be Bool(true)".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_null() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(0))? != SlotValue::Null {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 0 should be Null".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_set_const_missing_output_returns_error() -> Result<(), CoreError> {
    let plan = make_rejected_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(1)],
        vec![],
        crate::workflow::SlotFlowFailure::InvalidReference,
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
            reason: "node 0 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(1))? != SlotValue::I64(100) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 should be I64(100)".into(),
        });
    }
    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 taint should be Secret".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_copy_clean_source_has_clean_taint() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::Clean {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 1 taint should be Clean".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_copy_uninitialized_source_returns_error() -> Result<(), CoreError> {
    let plan = make_rejected_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        crate::workflow::SlotFlowFailure::UninitializedRead,
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
            reason: "node 0 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing".into(),
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if *run.read_slot(SlotIdx::new(2))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "slot 2 should be I64(42)".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(1))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            let field = obj.first().ok_or(CoreError::InternalInvariantViolation {
                reason: "object should have a field".into(),
            })?;
            if field.key != field_sym || field.value != SlotValue::I64(42) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field mismatch".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 1 should be Object".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_empty_fields() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildObject {
                    fields: vec![].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::Object(id) => {
            let obj = store.object(id)?;
            if !obj.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildObject should create empty object".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be Object".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_object_uninitialized_field_returns_error() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let plan = make_rejected_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(5))].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        crate::workflow::SlotFlowFailure::UninitializedRead,
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
            reason: "node 0 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::BuildObject {
                    fields: vec![(field_sym, SlotIdx::new(0))].into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::DerivedFromSecret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::DerivedFromSecret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject output taint should be DerivedFromSecret".into(),
        });
    }
    Ok(())
}

// ---- BuildList step ----

#[test]
fn replay_build_list_creates_handle() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(1)].into(),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing".into(),
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if list.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list should have 2 items".into(),
                });
            }
            if list[0] != SlotValue::I64(10) || list[1] != SlotValue::I64(20) {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "list items mismatch".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be List".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_empty_items() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildList {
                    items: vec![].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(0))? {
        SlotValue::List(id) => {
            let list = store.list(id)?;
            if !list.is_empty() {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "empty BuildList should create empty list".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 0 should be List".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_build_list_uninitialized_item_returns_error() -> Result<(), CoreError> {
    let plan = make_rejected_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(5)].into(),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        crate::workflow::SlotFlowFailure::UninitializedRead,
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
            reason: "node 0 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
    run.write_taint(SlotIdx::new(0), Taint::Secret)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.read_taint(SlotIdx::new(1))? != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BuildList output taint should be Secret".into(),
        });
    }
    Ok(())
}

// ---- Finish step ----

#[test]
fn replay_finish_returns_finished_action() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Finished => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Finish should return Finished".into(),
            });
        }
    }
    if run.executed() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed should be 2".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_finish_uninitialized_result_returns_error() -> Result<(), CoreError> {
    let plan = make_rejected_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(5),
            },
            None,
            None,
        )],
        vec![],
        vec![],
        crate::workflow::SlotFlowFailure::UninitializedRead,
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
            reason: "node 0 missing".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(1) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Jump should return Continue(1)".into(),
            });
        }
    }
    if run.pc() != StepIdx::new(1) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "PC should be at step 1".into(),
        });
    }
    Ok(())
}

#[test]
fn ce_004_replay_jump_out_of_bounds_yields_typed_step_not_found() -> Result<(), CoreError> {
    // CE-004 E2E regression: a Jump node whose target is outside the run
    // frame's step array must reach the caller as `ReplayError::StepNotFound`,
    // not collapse to the generic `Internal` variant.
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(0)],
        vec![],
    )?;

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(0))?;
    let _store = ValueStore::new();

    // Drive `replay_jump` with an out-of-bounds target (99) directly to
    // exercise the `set_pc` failure path without relying on workflow
    // validation rejecting the plan.
    let result = super::replay_jump(&mut run, StepIdx::new(99));
    match result {
        Ok(_) => Err(CoreError::InternalInvariantViolation {
            reason: "replay_jump accepted out-of-bounds target".into(),
        }),
        Err(ReplayError::StepNotFound { step }) if step == StepIdx::new(99) => Ok(()),
        Err(_) => Err(CoreError::InternalInvariantViolation {
            reason: "CE-004: out-of-bounds Jump must surface as typed StepNotFound".into(),
        }),
    }
}

// ---- Suspend steps ----

#[test]
fn replay_do_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            None,
            None,
        )],
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
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::ActionPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Do should return Suspended(0, Do)".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_ask_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None.into(),
            },
            None,
            None,
        )],
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
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::AskPending => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Ask should return Suspended(0, Ask)".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_until_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            None,
            None,
        )],
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
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitUntil => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitUntil should return Suspended(0, WaitUntil)".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_wait_event_suspends() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None.into(),
            },
            None,
            None,
        )],
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
            reason: "node 0 missing".into(),
        })?;
    let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Suspended { step, kind }
            if step == StepIdx::new(0) && kind == SuspensionKind::WaitEvent => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "WaitEvent should return Suspended(0, WaitEvent)".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into(),
                    otherwise: Some(StepIdx::new(3)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(2) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ChooseSlot true should go to step 2".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_choose_slot_false_falls_to_otherwise() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into(),
                    otherwise: Some(StepIdx::new(3)).into(),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    let action = replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match action {
        ReplayAction::Continue(next) if next == StepIdx::new(3) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "ChooseSlot false should go to otherwise (step 3)".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing".into(),
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node3 = plan
        .node(StepIdx::new(3))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 3 missing".into(),
        })?;
    replay_step(node3, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    if run.executed() != 4 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "executed counter should be 4".into(),
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
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::BuildObject {
                    fields: vec![(sym_a, SlotIdx::new(0)), (sym_b, SlotIdx::new(1))].into(),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
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
            reason: "node 0 missing".into(),
        })?;
    replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node1 = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing".into(),
        })?;
    replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    let node2 = plan
        .node(StepIdx::new(2))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "node 2 missing".into(),
        })?;
    replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

    match *run.read_slot(SlotIdx::new(2))? {
        SlotValue::Object(id) => {
            let fields = store.object(id)?;
            if fields.len() != 2 {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "object should have 2 fields".into(),
                });
            }
            if fields[0].key != sym_a
                || fields[0].value != SlotValue::I64(10)
                || fields[1].key != sym_b
                || fields[1].value != SlotValue::I64(20)
            {
                return Err(CoreError::InternalInvariantViolation {
                    reason: "field order or values wrong".into(),
                });
            }
        }
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "slot 2 should be Object".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn replay_collect_next_reports_missing_source_list() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                None,
                None,
            ),
            crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
            crate::workflow::node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        ],
        vec![],
        vec![],
    )?;
    let mut store = ValueStore::new();
    let current_page = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "current page insert failed".into(),
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
            reason: "CollectNext should report missing source list".into(),
        }),
    }
}

#[test]
fn replay_copy_reports_source_slot_failures() -> Result<(), CoreError> {
    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(7),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let mut run = RunFrame::new(RunId::new(92), StepIdx::new(0), 2, 1)?;
    match super::replay_copy(&mut run, &node, SlotIdx::new(7)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "Copy should report out-of-bounds source slot".into(),
            });
        }
    }
    match super::replay_copy(&mut run, &node, SlotIdx::new(0)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Copy should report uninitialized source slot".into(),
        }),
    }
}

#[test]
fn replay_build_object_reports_field_slot_failures() -> Result<(), CoreError> {
    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildObject {
            fields: vec![].into(),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(93), StepIdx::new(0), 2, 1)?;
    let fields = [(SymbolId::new(0), SlotIdx::new(7))];
    match super::replay_build_object(&mut run, &mut store, &node, &fields) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BuildObject should report out-of-bounds field slot".into(),
            });
        }
    }
    let fields = [(SymbolId::new(0), SlotIdx::new(0))];
    match super::replay_build_object(&mut run, &mut store, &node, &fields) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "BuildObject should report uninitialized field slot".into(),
        }),
    }
}

#[test]
fn replay_build_list_reports_item_slot_failures() -> Result<(), CoreError> {
    let node = crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildList {
            items: vec![].into(),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    );
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(94), StepIdx::new(0), 2, 1)?;
    match super::replay_build_list(&mut run, &mut store, &node, &[SlotIdx::new(7)]) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(7) => {}
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BuildList should report out-of-bounds item slot".into(),
            });
        }
    }
    match super::replay_build_list(&mut run, &mut store, &node, &[SlotIdx::new(0)]) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "BuildList should report uninitialized item slot".into(),
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
                reason: "Finish should report out-of-bounds result slot".into(),
            });
        }
    }
    match super::replay_finish(&mut run, SlotIdx::new(0)) {
        Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(0) => Ok(()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "Finish should report uninitialized result slot".into(),
        }),
    }
}

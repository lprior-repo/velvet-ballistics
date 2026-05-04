//! Replay step execution.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use super::{ReplayError, eval_expr_for_replay, slot_to_replay_err};

/// Internal action returned by `replay_step`.
pub enum ReplayAction {
    /// Continue to the next step.
    Continue(StepIdx),
    /// The run finished.
    Finished,
    /// The run is suspended on a non-deterministic node.
    Suspended { step: StepIdx, kind: &'static str },
}

/// Replays a single deterministic step.
///
/// For deterministic node kinds (SetConst, Copy, EvalExpr, BuildObject, BuildList,
/// Finish, Nop), executes the same logic as the engine's `step_once`.
/// For non-deterministic (Do/Action, Ask, WaitUntil, WaitEvent), returns a
/// suspension signal.
pub fn replay_step(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
) -> Result<ReplayAction, ReplayError> {
    match &node.kind {
        CompiledNodeKind::Nop => replay_nop(node, run),
        CompiledNodeKind::SetConst { value } => replay_set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => replay_copy(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => replay_eval_expr(plan, run, store, node, *expr),
        CompiledNodeKind::BuildObject { fields } => replay_build_object(run, store, node, fields),
        CompiledNodeKind::BuildList { items } => replay_build_list(run, store, node, items),
        CompiledNodeKind::Finish { result } => replay_finish(run, *result),
        CompiledNodeKind::Jump { target } => replay_jump(run, *target),
        CompiledNodeKind::Do { .. } => replay_suspend(node, "Do"),
        CompiledNodeKind::Ask { .. } => replay_suspend(node, "Ask"),
        CompiledNodeKind::WaitUntil { .. } => replay_suspend(node, "WaitUntil"),
        CompiledNodeKind::WaitEvent { .. } => replay_suspend(node, "WaitEvent"),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => super::choose::replay_choose_slot(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => super::choose::replay_choose_expr(plan, run, store, branches, *otherwise),
        _ => Err(ReplayError::Internal {
            reason: "unsupported node kind for replay",
        }),
    }
}

fn replay_nop(node: &CompiledNode, run: &mut RunFrame) -> Result<ReplayAction, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "Nop node missing next step",
    })?;
    run.set_pc(next).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(next))
}

fn replay_finish(run: &mut RunFrame, result: SlotIdx) -> Result<ReplayAction, ReplayError> {
    let _value = *run.read_slot(result).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading finish result slot",
        },
    })?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Finished)
}

fn replay_jump(run: &mut RunFrame, target: StepIdx) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

fn replay_suspend(node: &CompiledNode, kind: &'static str) -> Result<ReplayAction, ReplayError> {
    Ok(ReplayAction::Suspended {
        step: node.id,
        kind,
    })
}

fn replay_set_const(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &CompiledNode,
    value: ConstIdx,
) -> Result<ReplayAction, ReplayError> {
    let constant = plan.constant(value).copied().ok_or(ReplayError::Internal {
        reason: "constant out of bounds",
    })?;
    let slot_value = constant
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "SetConst node missing output slot",
    })?;
    run.write_slot(output, slot_value)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_copy(
    run: &mut RunFrame,
    node: &CompiledNode,
    source: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(source).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading copy source slot",
        },
    })?;
    let taint = run.read_taint(source).map_err(slot_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "Copy node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_eval_expr(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    expr: ExprIdx,
) -> Result<ReplayAction, ReplayError> {
    let (value, taint) = eval_expr_for_replay(plan, run, store, expr)
        .map_err(|_| ReplayError::ExpressionEvalFailed { step: node.id })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "EvalExpr node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_build_object(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<ReplayAction, ReplayError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields.get(index).ok_or(ReplayError::Internal {
            reason: "build_object field index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_object field slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        entries.push(ObjectField { key: *key, value, taint: slot_taint });
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_object field index overflow",
        })?;
    }
    let handle = store
        .insert_object(entries.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "insert_object failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildObject node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::Object(handle), accumulated_taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn replay_build_list(
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    items: &[SlotIdx],
) -> Result<ReplayAction, ReplayError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| ReplayError::Internal {
            reason: "allocation failed",
        })?;
    let mut accumulated_taint = Taint::Clean;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items.get(index).ok_or(ReplayError::Internal {
            reason: "build_list item index checked by loop bound",
        })?;
        let value = *run.read_slot(*slot).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
            _ => ReplayError::Internal {
                reason: "unexpected error reading build_list item slot",
            },
        })?;
        let slot_taint = run.read_taint(*slot).map_err(slot_to_replay_err)?;
        accumulated_taint = join_taint(accumulated_taint, slot_taint);
        values.push(value);
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "build_list item index overflow",
        })?;
    }
    let handle =
        store
            .insert_list(values.into_boxed_slice())
            .map_err(|_| ReplayError::Internal {
                reason: "insert_list failed",
            })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "BuildList node missing output slot",
    })?;
    run.write_slot_with_taint(output, SlotValue::List(handle), accumulated_taint)
        .map_err(slot_to_replay_err)?;
    let next = advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}

fn advance_to_next(run: &mut RunFrame, node: &CompiledNode) -> Result<StepIdx, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "node missing next step",
    })?;
    run.set_pc(next).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use crate::errors::CoreError;
    use crate::frame::RunFrame;
    use crate::ids::{
        ActionId, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
    };
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::replay::{ReplayError, step::ReplayAction};
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
            ReplayError::NonDeterministicStep { step, .. } => {
                CoreError::InvalidProgramCounter { step }
            }
            ReplayError::Internal { reason } => {
                CoreError::InternalInvariantViolation { reason }
            }
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        let result = replay_step(node, &mut run, &mut store, &plan);
        assert!(
            matches!(result, Err(ReplayError::Internal { reason: "Nop node missing next step" })),
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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
            vec![CompiledNode {
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
            }],
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        run.write_taint(SlotIdx::new(0), Taint::Secret)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        let result = replay_step(node, &mut run, &mut store, &plan);
        assert!(
            result.is_err(),
            "Copy from uninitialized slot must fail"
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
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

        for idx in 0u16..2 {
            let node = plan.node(StepIdx::new(idx)).ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
            replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        }

        let node2 = plan.node(StepIdx::new(2)).ok_or(CoreError::InternalInvariantViolation {
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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
            vec![CompiledNode {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        run.write_taint(SlotIdx::new(0), Taint::DerivedFromSecret)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
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

        for idx in 0u16..2 {
            let node = plan.node(StepIdx::new(idx)).ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
            replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        }

        let node2 = plan.node(StepIdx::new(2)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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
            vec![CompiledNode {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        run.write_taint(SlotIdx::new(0), Taint::Secret)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
        let action =
            replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        match action {
            ReplayAction::Suspended { step, kind }
                if step == StepIdx::new(0) && kind == "Do" => {}
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        match action {
            ReplayAction::Suspended { step, kind }
                if step == StepIdx::new(0) && kind == "Ask" => {}
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        match action {
            ReplayAction::Suspended { step, kind }
                if step == StepIdx::new(0) && kind == "WaitUntil" => {}
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

        let node = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        let action = replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        match action {
            ReplayAction::Suspended { step, kind }
                if step == StepIdx::new(0) && kind == "WaitEvent" => {}
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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
        let action =
            replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

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

        let node0 = plan.node(StepIdx::new(0)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 0 missing",
        })?;
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

        let node1 = plan.node(StepIdx::new(1)).ok_or(CoreError::InternalInvariantViolation {
            reason: "node 1 missing",
        })?;
        let action =
            replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;

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

        for idx in 0u16..4 {
            let node = plan.node(StepIdx::new(idx)).ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
            replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        }

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

        for idx in 0u16..2 {
            let node = plan.node(StepIdx::new(idx)).ok_or(CoreError::InternalInvariantViolation {
                reason: "node missing",
            })?;
            replay_step(node, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
        }

        let node2 = plan.node(StepIdx::new(2)).ok_or(CoreError::InternalInvariantViolation {
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
}

//! Deterministic replay engine for reconstructing slot state from journal evidence.
//!
//! Given a compiled workflow and step evidence, re-executes deterministic steps
//! to reconstruct slot state. Non-deterministic nodes (Action, Ask) cause
//! suspension with the blocking step index.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp};

/// Failures that can occur during deterministic replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The target step does not exist in the compiled workflow.
    StepNotFound {
        /// Requested step index.
        step: StepIdx,
    },
    /// Replay encountered a non-deterministic node that cannot be replayed.
    NonDeterministicStep {
        /// Step index of the blocking node.
        step: StepIdx,
        /// Human-readable node kind name.
        kind: &'static str,
    },
    /// A required slot was not populated before being read.
    SlotNotAvailable {
        /// Slot that was missing.
        slot: SlotIdx,
    },
    /// Expression evaluation failed during replay.
    ExpressionEvalFailed {
        /// Step where evaluation failed.
        step: StepIdx,
    },
    /// An internal error occurred during replay.
    Internal {
        /// Description of the internal failure.
        reason: &'static str,
    },
}

/// Deterministic replay engine.
///
/// Holds a reference to a compiled workflow and re-executes deterministic nodes
/// in order from the entry step to a target step, reconstructing slot state in
/// the provided `ValueStore` and `RunFrame`.
pub struct ReplayEngine<'a> {
    plan: &'a CompiledWorkflow,
}

impl<'a> ReplayEngine<'a> {
    /// Creates a new replay engine for the given compiled workflow.
    pub fn new(plan: &'a CompiledWorkflow) -> Self {
        Self { plan }
    }

    /// Replays deterministic steps from the entry point up to `target_step`.
    ///
    /// Returns `Ok(target_step)` if the target was reached.
    /// Returns `Ok(suspension_point)` if a non-deterministic node blocked progress
    /// before the target was reached.
    pub fn replay_up_to(
        &self,
        target_step: StepIdx,
        store: &mut ValueStore,
    ) -> Result<StepIdx, ReplayError> {
        if self.plan.node(target_step).is_none() {
            return Err(ReplayError::StepNotFound { step: target_step });
        }

        let entry = self.plan.entry();
        let step_count = self.plan.node_count();
        let slot_count = self.plan.slot_count();

        let mut run =
            RunFrame::new(RunId::new(0), entry, step_count, slot_count).map_err(|_| {
                ReplayError::Internal {
                    reason: "failed to create run frame",
                }
            })?;

        let mut current = entry;
        loop {
            if current == target_step {
                return Ok(current);
            }

            let node = match self.plan.node(current) {
                Some(n) => n,
                None => return Err(ReplayError::StepNotFound { step: current }),
            };

            match replay_step(node, &mut run, store, self.plan) {
                Ok(ReplayAction::Continue(next)) => {
                    current = next;
                }
                Ok(ReplayAction::Finished) => {
                    return Ok(current);
                }
                Ok(ReplayAction::Suspended { step, kind }) => {
                    return Err(ReplayError::NonDeterministicStep { step, kind });
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Internal action returned by `replay_step`.
enum ReplayAction {
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
fn replay_step(
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
        } => replay_choose_slot(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => replay_choose_expr(plan, run, store, branches, *otherwise),
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
        entries.push(ObjectField { key: *key, value });
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

fn replay_choose_slot(
    run: &mut RunFrame,
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index checked by loop bound",
        })?;
        let value = run.read_slot(branch.condition).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
            _ => ReplayError::Internal {
                reason: "unexpected error reading choose_slot condition",
            },
        })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_slot condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_slot no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

fn replay_choose_expr(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index checked by loop bound",
        })?;
        let (value, _taint) = eval_expr_for_replay(plan, run, store, branch.condition)
            .map_err(|_| ReplayError::ExpressionEvalFailed { step: run.pc() })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_expr condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_expr no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
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

fn slot_to_replay_err(e: EngineError) -> ReplayError {
    match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected engine error during replay",
        },
    }
}

// ---------------------------------------------------------------------------
// Minimal expression evaluator for replay
// ---------------------------------------------------------------------------

struct ReplayExprStack {
    values: [SlotValue; crate::limits::MAX_EXPRESSION_STACK_USIZE],
    len: u8,
    capacity: u8,
}

impl ReplayExprStack {
    fn new(capacity: u8) -> Result<Self, ReplayError> {
        if usize::from(capacity) <= crate::limits::MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; crate::limits::MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
            })
        } else {
            Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
        }
    }

    fn push(&mut self, value: SlotValue) -> Result<(), ReplayError> {
        if self.len >= self.capacity {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        let index = usize::from(self.len);
        *self
            .values
            .get_mut(index)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })? = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        Ok(())
    }

    fn pop(&mut self) -> Result<SlotValue, ReplayError> {
        if self.len == 0 {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        self.len = self
            .len
            .checked_sub(1)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        self.values
            .get(usize::from(self.len))
            .copied()
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
    }
}

fn eval_expr_for_replay(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), ReplayError> {
    let program = plan.expression(expr).ok_or(ReplayError::Internal {
        reason: "expression out of bounds",
    })?;
    let mut stack = ReplayExprStack::new(program.max_stack)?;
    let mut taint_accum = Taint::Clean;
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = program
            .ops
            .get(index)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "expression op index checked by loop bound",
            })?;
        eval_replay_op(plan, run, store, op, &mut stack, &mut taint_accum)?;
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "expression op index overflow",
        })?;
    }
    if stack.len != 1 {
        return Err(ReplayError::ExpressionEvalFailed { step: run.pc() });
    }
    let value = stack.pop()?;
    Ok((value, taint_accum))
}

fn eval_replay_op(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    op: ExprOp,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, slot, stack, taint_accum),
        ExprOp::LoadConst(constant) => eval_load_const(plan, constant, stack),
        ExprOp::LoadAccessor(accessor) => {
            eval_load_accessor(plan, run, store, accessor, stack, taint_accum)
        }
        ExprOp::Eq => eval_eq(stack),
        ExprOp::NotEq => eval_not_eq(stack),
        ExprOp::And => eval_and(stack),
        ExprOp::Or => eval_or(stack),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_add(stack),
        ExprOp::Sub => eval_sub(stack),
        ExprOp::Mul => eval_mul(stack),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_gt(stack),
        ExprOp::Gte => eval_gte(stack),
        ExprOp::Lt => eval_lt(stack),
        ExprOp::Lte => eval_lte(stack),
        _ => Err(ReplayError::Internal {
            reason: "unsupported expression op for replay",
        }),
    }
}

fn eval_load_slot(
    run: &RunFrame,
    slot: SlotIdx,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    let value = *run.read_slot(slot).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
        _ => ReplayError::Internal {
            reason: "unexpected error reading expression load slot",
        },
    })?;
    let slot_taint = run.read_taint(slot).map_err(|_| ReplayError::Internal {
        reason: "read_taint failed",
    })?;
    *taint_accum = join_taint(*taint_accum, slot_taint);
    stack.push(value)
}

fn eval_load_const(
    plan: &CompiledWorkflow,
    constant: ConstIdx,
    stack: &mut ReplayExprStack,
) -> Result<(), ReplayError> {
    let value = plan
        .constant(constant)
        .ok_or(ReplayError::Internal {
            reason: "constant out of bounds",
        })?
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    stack.push(value)
}

fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    let accessor_program = plan.accessor(accessor).ok_or(ReplayError::Internal {
        reason: "accessor out of bounds",
    })?;
    let root_taint = run
        .read_taint(accessor_program.root)
        .map_err(|_| ReplayError::Internal {
            reason: "read_taint failed for accessor root",
        })?;
    let value = eval_accessor_for_replay(run, store, accessor_program)?;
    *taint_accum = join_taint(*taint_accum, root_taint);
    stack.push(value)
}

fn eval_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left == right))
}

fn eval_not_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left != right))
}

fn eval_and(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_bool = expect_bool_replay(left)?;
    let right_bool = expect_bool_replay(right)?;
    stack.push(SlotValue::Bool(left_bool && right_bool))
}

fn eval_or(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_bool = expect_bool_replay(left)?;
    let right_bool = expect_bool_replay(right)?;
    stack.push(SlotValue::Bool(left_bool || right_bool))
}

fn eval_not(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let b = expect_bool_replay(value)?;
    stack.push(SlotValue::Bool(!b))
}

fn eval_add(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_add(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_sub(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_sub(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_mul(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_mul(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_div(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_div(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_gt(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left > right))
}

fn eval_gte(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left >= right))
}

fn eval_lt(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left < right))
}

fn eval_lte(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left <= right))
}

fn eval_accessor_for_replay(
    run: &RunFrame,
    store: &mut ValueStore,
    program: &crate::workflow::AccessorProgram,
) -> Result<SlotValue, ReplayError> {
    let mut current = *run.read_slot(program.root).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading accessor root",
        },
    })?;
    if program.path.is_empty() {
        return Ok(current);
    }
    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program
            .path
            .get(index)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "accessor path index checked by loop bound",
            })?;
        current = match (current, segment) {
            (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => store
                .object_field(object, field)
                .map_err(|_| ReplayError::Internal {
                    reason: "object field not found during replay accessor",
                })?,
            (SlotValue::List(list), crate::workflow::PathSegment::Index(idx)) => store
                .list_item(list, idx)
                .map_err(|_| ReplayError::Internal {
                    reason: "list index out of bounds during replay accessor",
                })?,
            (_, _) => {
                return Err(ReplayError::Internal {
                    reason: "unsupported accessor traversal during replay",
                });
            }
        };
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "accessor path index overflow",
        })?;
    }
    Ok(current)
}

fn pop_pair(stack: &mut ReplayExprStack) -> Result<(SlotValue, SlotValue), ReplayError> {
    let right = stack.pop()?;
    let left = stack.pop()?;
    Ok((left, right))
}

fn pop_i64_pair(stack: &mut ReplayExprStack) -> Result<(i64, i64), ReplayError> {
    let right = stack.pop()?;
    let left = stack.pop()?;
    Ok((expect_i64_replay(left)?, expect_i64_replay(right)?))
}

fn expect_bool_replay(value: SlotValue) -> Result<bool, ReplayError> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        _ => Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        }),
    }
}

fn expect_i64_replay(value: SlotValue) -> Result<i64, ReplayError> {
    match value {
        SlotValue::I64(v) => Ok(v),
        _ => Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::CoreError;
    use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::value::ConstValue;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, ResourceContract, WorkflowParts,
        check_expr_stack_bound,
    };

    fn make_plan(
        nodes: Vec<CompiledNode>,
        constants: Vec<ConstValue>,
        expressions: Vec<ExprProgram>,
    ) -> Result<CompiledWorkflow, CoreError> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
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
            ReplayError::NonDeterministicStep { step, .. } => {
                CoreError::InvalidProgramCounter { step }
            }
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
        replay_step(node0, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
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
        replay_step(node1, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
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
        replay_step(node2, &mut run, &mut store, &plan).map_err(replay_err_to_core)?;
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
}

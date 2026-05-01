//! Synchronous in-memory state-machine loop.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId};
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::SlotValue;
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    AccessorProgram, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, PathSegment,
    SlotBranch, WorkflowError, WorkflowParts,
};

/// Bounded number of steps a caller may execute in one engine slice.
pub struct StepBudget {
    remaining: u64,
}

impl StepBudget {
    /// Largest bounded execution slice representable by the runtime.
    pub const MAX: Self = Self {
        remaining: u64::MAX,
    };

    /// Creates a budget. Zero is valid and executes no transitions.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self { remaining: value }
    }

    /// Attempts to consume one transition from the budget.
    pub fn try_take(&mut self) -> Result<bool, EngineError> {
        if self.remaining == 0 {
            Ok(false)
        } else {
            self.remaining = self.remaining.saturating_sub(1);
            Ok(true)
        }
    }

    /// Remaining transitions.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }
}

/// Outcome of one or more engine transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineSignal {
    /// The run made progress and can continue immediately.
    Continue,
    /// The run finished with a result value.
    Finished(SlotValue),
    /// The caller's execution slice ended before completion.
    StepBudgetExhausted,
    /// The run suspended on an action.
    AwaitingAction,
    /// The run suspended on wait.
    AwaitingWait,
    /// The run suspended on ask.
    AwaitingAsk,
}

/// Creates a run frame for a compiled workflow.
pub fn new_run_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, EngineError> {
    RunFrame::new(
        run_id,
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
}

/// Executes one compiled node.
pub fn step_once(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    let pc = run.pc();
    let node = plan
        .node(pc)
        .ok_or(EngineError::InvalidProgramCounter { step: pc })?;
    run.mark_running(pc)?;
    let signal = match execute_node(plan, run, node, store) {
        Ok(signal) => signal,
        Err(error) => {
            run.mark_failed(pc)?;
            return Err(error);
        }
    };
    mark_step_after_signal(run, pc, &signal)?;
    Ok(signal)
}

#[inline]
fn execute_node(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    match &node.kind {
        CompiledNodeKind::Nop => jump_to_next(run, node.next, node.id),
        CompiledNodeKind::SetConst { value } => set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => copy_slot(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => eval_expr_node(plan, run, node, store, *expr),
        CompiledNodeKind::BuildObject { fields } => build_object_node(run, node, fields, store),
        CompiledNodeKind::BuildList { items } => build_list_node(run, node, items, store),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => choose_slot_branch(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => choose_expr_branch(plan, run, store, branches, *otherwise),
        other => execute_boundary_node(run, other),
    }
}

#[inline]
fn execute_boundary_node(
    run: &mut RunFrame,
    kind: &CompiledNodeKind,
) -> Result<EngineSignal, EngineError> {
    match kind {
        CompiledNodeKind::Do { .. } => Ok(EngineSignal::AwaitingAction),
        CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. } => {
            Ok(EngineSignal::AwaitingWait)
        }
        CompiledNodeKind::Ask { .. } => Ok(EngineSignal::AwaitingAsk),
        CompiledNodeKind::Jump { target } => jump_to(run, *target),
        CompiledNodeKind::Finish { result } => finish_run(run, *result),
        _ => Err(EngineError::UnsupportedPrimitive {
            primitive: "not_yet_implemented",
        }),
    }
}

fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &EngineSignal,
) -> Result<(), EngineError> {
    match signal {
        EngineSignal::AwaitingWait => run.mark_waiting(step),
        EngineSignal::AwaitingAsk => run.mark_asking(step),
        EngineSignal::AwaitingAction => Ok(()),
        EngineSignal::Continue | EngineSignal::Finished(_) => run.mark_succeeded(step),
        EngineSignal::StepBudgetExhausted => Ok(()),
    }
}

fn choose_expr_branch(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &ValueStore,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<EngineSignal, EngineError> {
    let next = choose_expr_target(plan, run, store, branches, otherwise)?;
    jump_to(run, next)
}

fn choose_expr_target(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &ValueStore,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<StepIdx, EngineError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "choose expr branch index checked by loop bound",
            })?;
        if let Some(target) = choose_expr_branch_target(plan, run, store, branch)? {
            return Ok(target);
        }
        index = index.checked_add(1).ok_or({
            EngineError::InternalInvariantViolation {
                reason: "choose expr branch index overflow",
            }
        })?;
    }

    otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })
}

fn choose_expr_branch_target(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &ValueStore,
    branch: &ExprBranch,
) -> Result<Option<StepIdx>, EngineError> {
    match eval_expr_with_store(plan, run, store, branch.condition)? {
        SlotValue::Bool(true) => Ok(Some(branch.target)),
        SlotValue::Bool(false) => Ok(None),
        value => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: value.type_name(),
        }),
    }
}

/// Executes deterministic nodes until finish or budget exhaustion.
pub fn run_until_blocked(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    mut budget: StepBudget,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    drive_deterministic(plan, run, &mut budget, store)
}

/// Executes deterministic nodes until finish, suspension, or budget exhaustion.
pub fn drive_deterministic(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    while budget.try_take()? {
        let signal = step_once(plan, run, store)?;
        if !matches!(signal, EngineSignal::Continue) {
            return Ok(signal);
        }
    }
    Ok(EngineSignal::StepBudgetExhausted)
}

/// Constructs an object handle from field pairs read from frame slots.
pub fn build_object(
    store: &mut ValueStore,
    run: &RunFrame,
    fields: &[(SymbolId, SlotIdx)],
) -> Result<ObjectId, EngineError> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(fields.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    let mut index = 0usize;
    while index < fields.len() {
        let (key, slot) = fields
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_object field index checked by loop bound",
            })?;
        let value = *run.read_slot(*slot)?;
        entries.push(ObjectField { key: *key, value });
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_object field index overflow",
            })?;
    }
    store.insert_object(entries.into_boxed_slice())
}

/// Constructs a list handle from slot values read from the frame.
pub fn build_list(
    store: &mut ValueStore,
    run: &RunFrame,
    items: &[SlotIdx],
) -> Result<ListId, EngineError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(items.len())
        .map_err(|_| EngineError::AllocationFailed)?;
    let mut index = 0usize;
    while index < items.len() {
        let slot = items
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_list item index checked by loop bound",
            })?;
        values.push(*run.read_slot(*slot)?);
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "build_list item index overflow",
            })?;
    }
    store.insert_list(values.into_boxed_slice())
}

/// Validates compiled workflow IR integrity.
pub fn validate_compiled_workflow(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    CompiledWorkflow::try_from_parts(parts.clone())?;
    Ok(())
}

/// Validates resource contract bounds against hard limits.
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    if usize::from(contract.max_steps) > crate::limits::MAX_STEPS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_steps",
        });
    }
    if usize::from(contract.max_slots) > crate::limits::MAX_SLOTS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_slots",
        });
    }
    if usize::from(contract.max_constants) > crate::limits::MAX_CONSTANTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_constants",
        });
    }
    if usize::from(contract.max_accessors) > crate::limits::MAX_ACCESSORS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_accessors",
        });
    }
    if usize::from(contract.max_expressions) > crate::limits::MAX_EXPRESSIONS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expressions",
        });
    }
    if contract.max_expr_stack > crate::limits::MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expr_stack",
        });
    }
    Ok(())
}

/// Validates that all node indices are within the node array bounds.
pub fn validate_node_bounds(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in parts.nodes.iter() {
        if node.id.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds { step: node.id });
        }
        if let Some(next) = node.next
            && next.as_usize() >= node_count
        {
            return Err(WorkflowError::StepOutOfBounds { step: next });
        }
    }
    Ok(())
}

/// Validates that all step transition targets reference valid node indices.
pub fn validate_transition_target(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in parts.nodes.iter() {
        match &node.kind {
            CompiledNodeKind::Jump { target } if target.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *target });
            }
            CompiledNodeKind::Jump { .. } => {}
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                validate_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                validate_slot_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ForEachStart { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::ForEachNext { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for branch in branches.iter() {
                    if branch.as_usize() >= node_count {
                        return Err(WorkflowError::StepOutOfBounds { step: *branch });
                    }
                }
                if join.as_usize() >= node_count {
                    return Err(WorkflowError::StepOutOfBounds { step: *join });
                }
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                validate_two_step_targets(*entry, *join, node_count)?;
            }
            CompiledNodeKind::CollectStart { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::CollectPage { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::CollectNext { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::ReduceStart { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::ReduceNext { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::RepeatStart { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } if done.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *done });
            }
            CompiledNodeKind::RepeatCheck { .. } => {}
            CompiledNodeKind::RetryCheck {
                body, exhausted, ..
            } => {
                validate_two_step_targets(*body, *exhausted, node_count)?;
            }
            CompiledNodeKind::ErrorHandler { body, handler } => {
                validate_two_step_targets(*body, *handler, node_count)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_branch_targets(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_slot_branch_targets(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_two_step_targets(
    first: StepIdx,
    second: StepIdx,
    node_count: usize,
) -> Result<(), WorkflowError> {
    if first.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: first });
    }
    if second.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: second });
    }
    Ok(())
}

#[inline]
fn set_const(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    value: crate::ids::ConstIdx,
) -> Result<EngineSignal, EngineError> {
    let constant = plan
        .constant(value)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { index: value })?;
    let slot_value = constant.to_slot_value()?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot(output, slot_value)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
fn copy_slot(
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    source: SlotIdx,
) -> Result<EngineSignal, EngineError> {
    let value = *run.read_slot(source)?;
    let taint = run.read_taint(source)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot_with_taint(output, value, taint)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let next = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, next)
}

#[inline]
fn jump_to(run: &mut RunFrame, target: StepIdx) -> Result<EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(EngineSignal::Continue)
}

#[inline]
fn choose_slot_branch(
    run: &mut RunFrame,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<EngineSignal, EngineError> {
    let next = choose_slot_target(run, branches, otherwise)?;
    jump_to(run, next)
}

fn choose_slot_target(
    run: &RunFrame,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<StepIdx, EngineError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "choose slot branch index checked by loop bound",
            })?;
        if let Some(target) = choose_slot_branch_target(run, branch)? {
            return Ok(target);
        }
        index = index.checked_add(1).ok_or({
            EngineError::InternalInvariantViolation {
                reason: "choose slot branch index overflow",
            }
        })?;
    }

    otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })
}

fn choose_slot_branch_target(
    run: &RunFrame,
    branch: &SlotBranch,
) -> Result<Option<StepIdx>, EngineError> {
    match run.read_slot(branch.condition)? {
        SlotValue::Bool(true) => Ok(Some(branch.target)),
        SlotValue::Bool(false) => Ok(None),
        value => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: value.type_name(),
        }),
    }
}

/// Evaluates one expression program against the current frame.
pub fn eval_expr(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    eval_expr_inner(plan, run, None, expr)
}

/// Evaluates one expression program with cold arena access for accessor traversal.
pub fn eval_expr_with_store(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &ValueStore,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    eval_expr_inner(plan, run, Some(store), expr)
}

fn eval_expr_inner(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: Option<&ValueStore>,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .expression(expr)
        .ok_or(EngineError::ExprOutOfBounds { expr })?;
    let mut stack = ExprStack::new(program.max_stack)?;
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = expression_op(program.ops.as_ref(), index)?;
        eval_expr_op(plan, run, store, op, &mut stack)?;
        index = next_expr_index(index)?;
    }
    finish_expr_stack(&mut stack)
}

fn expression_op(ops: &[ExprOp], index: usize) -> Result<ExprOp, EngineError> {
    ops.get(index)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "expression op index checked by loop bound",
        })
}

fn next_expr_index(index: usize) -> Result<usize, EngineError> {
    index
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "expression op index overflow",
        })
}

fn finish_expr_stack(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    if stack.len() == 1 {
        stack.pop()
    } else {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        })
    }
}

#[inline]
fn eval_expr_node(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    store: &ValueStore,
    expr: ExprIdx,
) -> Result<EngineSignal, EngineError> {
    let value = eval_expr_with_store(plan, run, store, expr)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot(output, value)?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
fn build_object_node(
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    fields: &[(SymbolId, SlotIdx)],
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    let handle = build_object(store, run, fields)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot(output, SlotValue::Object(handle))?;
    jump_to_next(run, node.next, node.id)
}

#[inline]
fn build_list_node(
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    items: &[SlotIdx],
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    let handle = build_list(store, run, items)?;
    let output = node
        .output
        .ok_or(EngineError::MissingOutputSlot { step: node.id })?;
    run.write_slot(output, SlotValue::List(handle))?;
    jump_to_next(run, node.next, node.id)
}

fn eval_expr_op(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: Option<&ValueStore>,
    op: ExprOp,
    stack: &mut ExprStack,
) -> Result<(), EngineError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, stack, slot),
        ExprOp::LoadConst(constant) => eval_load_const(plan, stack, constant),
        ExprOp::LoadAccessor(accessor) => eval_load_accessor(plan, run, store, stack, accessor),
        other => eval_expr_operator(other, stack),
    }
}

fn eval_load_slot(run: &RunFrame, stack: &mut ExprStack, slot: SlotIdx) -> Result<(), EngineError> {
    push_value(stack, *run.read_slot(slot)?)
}

fn eval_load_const(
    plan: &CompiledWorkflow,
    stack: &mut ExprStack,
    constant: crate::ids::ConstIdx,
) -> Result<(), EngineError> {
    push_value(
        stack,
        plan.constant(constant)
            .ok_or(EngineError::ConstOutOfBounds { index: constant })?
            .to_slot_value()?,
    )
}

fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: Option<&ValueStore>,
    stack: &mut ExprStack,
    accessor: crate::ids::AccessorIdx,
) -> Result<(), EngineError> {
    push_value(stack, eval_accessor_inner(plan, run, store, accessor)?)
}

fn eval_expr_operator(op: ExprOp, stack: &mut ExprStack) -> Result<(), EngineError> {
    match op {
        ExprOp::Eq => eval_eq(stack, true),
        ExprOp::NotEq => eval_eq(stack, false),
        ExprOp::And => eval_bool_pair(stack, |left, right| left && right),
        ExprOp::Or => eval_bool_pair(stack, |left, right| left || right),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_i64_pair(stack, i64::checked_add),
        ExprOp::Sub => eval_i64_pair(stack, i64::checked_sub),
        ExprOp::Mul => eval_i64_pair(stack, i64::checked_mul),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_i64_cmp(stack, i64::gt),
        ExprOp::Gte => eval_i64_cmp(stack, i64::ge),
        ExprOp::Lt => eval_i64_cmp(stack, i64::lt),
        ExprOp::Lte => eval_i64_cmp(stack, i64::le),
        _ => Err(EngineError::InvalidCompiledWorkflow {
            reason: "unsupported expression op",
        }),
    }
}

/// Evaluates one accessor program against the current frame.
pub fn eval_accessor(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    accessor: crate::ids::AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, None, accessor)
}

/// Evaluates one accessor program with cold arena access.
pub fn eval_accessor_with_store(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &ValueStore,
    accessor: crate::ids::AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, Some(store), accessor)
}

fn eval_accessor_inner(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: Option<&ValueStore>,
    accessor: crate::ids::AccessorIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program(run, store, program)
}

fn eval_accessor_program(
    run: &RunFrame,
    store: Option<&ValueStore>,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let mut current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }

    let store = match store {
        Some(store) => store,
        None => {
            let segment = program.path.first().copied().ok_or({
                EngineError::InternalInvariantViolation {
                    reason: "accessor path checked non-empty",
                }
            })?;
            return Err(EngineError::UnsupportedAccessorTraversal {
                segment: path_segment_name(segment),
                found: current.type_name(),
            });
        }
    };
    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program.path.get(index).copied().ok_or({
            EngineError::InternalInvariantViolation {
                reason: "accessor path index checked by loop bound",
            }
        })?;
        current = traverse_accessor_segment(store, current, segment)?;
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "accessor path index overflow",
            })?;
    }
    Ok(current)
}

fn traverse_accessor_segment(
    store: &ValueStore,
    current: SlotValue,
    segment: PathSegment,
) -> Result<SlotValue, EngineError> {
    match (current, segment) {
        (SlotValue::Object(object), PathSegment::Field(field)) => store.object_field(object, field),
        (SlotValue::List(list), PathSegment::Index(index)) => store.list_item(list, index),
        (value, segment) => Err(EngineError::UnsupportedAccessorTraversal {
            segment: path_segment_name(segment),
            found: value.type_name(),
        }),
    }
}

const fn path_segment_name(segment: PathSegment) -> &'static str {
    match segment {
        PathSegment::Field(_) => "field",
        PathSegment::Index(_) => "index",
    }
}

struct ExprStack {
    values: [SlotValue; MAX_EXPRESSION_STACK_USIZE],
    len: u8,
    capacity: u8,
}

impl ExprStack {
    fn new(capacity: u8) -> Result<Self, EngineError> {
        if usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
            })
        } else {
            Err(EngineError::ExpressionStackOverflow { max: capacity })
        }
    }

    const fn len(&self) -> u8 {
        self.len
    }

    fn push(&mut self, value: SlotValue) -> Result<(), EngineError> {
        if self.len >= self.capacity {
            return Err(EngineError::ExpressionStackOverflow { max: self.capacity });
        }
        let index = usize::from(self.len);
        *self
            .values
            .get_mut(index)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })? = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })?;
        Ok(())
    }

    fn pop(&mut self) -> Result<SlotValue, EngineError> {
        if self.len == 0 {
            return Err(EngineError::ExpressionStackUnderflow);
        }
        self.len = self
            .len
            .checked_sub(1)
            .ok_or(EngineError::ExpressionStackUnderflow)?;
        self.values.get(usize::from(self.len)).copied().ok_or(
            EngineError::InternalInvariantViolation {
                reason: "expression stack pop index checked by length",
            },
        )
    }
}

fn push_value(stack: &mut ExprStack, value: SlotValue) -> Result<(), EngineError> {
    stack.push(value)
}

fn pop_value(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    stack.pop()
}

fn pop_pair(stack: &mut ExprStack) -> Result<(SlotValue, SlotValue), EngineError> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

fn eval_eq(stack: &mut ExprStack, positive: bool) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool((left == right) == positive))
}

fn eval_not(stack: &mut ExprStack) -> Result<(), EngineError> {
    let value = expect_bool(pop_value(stack)?)?;
    push_value(stack, SlotValue::Bool(!value))
}

fn eval_bool_pair(stack: &mut ExprStack, op: fn(bool, bool) -> bool) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(
        stack,
        SlotValue::Bool(op(expect_bool(left)?, expect_bool(right)?)),
    )
}

fn eval_i64_pair(
    stack: &mut ExprStack,
    op: fn(i64, i64) -> Option<i64>,
) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = op(left, right).ok_or(EngineError::InvalidCompiledWorkflow {
        reason: "integer arithmetic overflow",
    })?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_div(stack: &mut ExprStack) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = left.checked_div(right).ok_or(EngineError::DivisionByZero)?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_i64_cmp(stack: &mut ExprStack, op: fn(&i64, &i64) -> bool) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    push_value(stack, SlotValue::Bool(op(&left, &right)))
}

fn pop_i64_pair(stack: &mut ExprStack) -> Result<(i64, i64), EngineError> {
    let (left, right) = pop_pair(stack)?;
    Ok((expect_i64(left)?, expect_i64(right)?))
}

fn expect_bool(value: SlotValue) -> Result<bool, EngineError> {
    match value {
        SlotValue::Bool(value) => Ok(value),
        other => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: other.type_name(),
        }),
    }
}

fn expect_i64(value: SlotValue) -> Result<i64, EngineError> {
    match value {
        SlotValue::I64(value) => Ok(value),
        other => Err(EngineError::TypeMismatch {
            expected: "number",
            found: other.type_name(),
        }),
    }
}

#[inline]
fn finish_run(run: &mut RunFrame, result: SlotIdx) -> Result<EngineSignal, EngineError> {
    let value = *run.read_slot(result)?;
    run.increment_executed()?;
    Ok(EngineSignal::Finished(value))
}

#[cfg(test)]
mod tests {
    use super::{
        EngineError, EngineSignal, RunFrame, StepBudget, eval_accessor, eval_accessor_with_store,
        eval_expr, new_run_frame, run_until_blocked, step_once,
    };
    use crate::errors::CoreError;
    use crate::frame::StepState;
    use crate::ids::{
        AccessorIdx, ActionId, ConstIdx, ExprIdx, ObjectId, RunId, SlotIdx, StepIdx, SymbolId,
        WorkflowDigest,
    };
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::value_store::{ObjectField, ValueStore};
    use crate::workflow::{
        AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp,
        ExprProgram, PathSegment, SlotBranch, WorkflowParts,
    };

    fn test_store() -> ValueStore {
        ValueStore::new()
    }

    #[test]
    fn set_chain_finishes_with_slot_value() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(7), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::Finished(SlotValue::I64(42))))?;
        ensure_equal(run.executed(), 2)?;
        Ok(())
    }

    #[test]
    fn set_chain_finishes_with_object_slot_value() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(8), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::Finished(SlotValue::Bool(true))))?;
        Ok(())
    }

    #[test]
    fn const_finish_returns_constant_pool_value() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::Bool(true)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(9), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::Bool(true)) {
            Ok(())
        } else {
            Err(format!("unexpected const finish result: {result:?}"))
        }
    }

    #[test]
    fn set_const_rejects_missing_constant() -> Result<(), String> {
        let result = missing_constant_workflow(ConstIdx::new(1));

        match result {
            Err(crate::WorkflowError::ConstOutOfBounds { constant })
                if constant == ConstIdx::new(1) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected const validation result: {other:?}")),
        }
    }

    #[test]
    fn zero_budget_exhausts_without_execution() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(7), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 0)?;
        ensure_equal(run.pc(), StepIdx::new(0))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
        Ok(())
    }

    #[test]
    fn step_budget_try_take_consumes_exactly_one_transition() -> Result<(), String> {
        let mut budget = StepBudget::new(1);

        ensure_equal(budget.try_take().map_err(|error| error.to_string())?, true)?;
        ensure_equal(budget.remaining(), 0)?;
        ensure_equal(budget.try_take().map_err(|error| error.to_string())?, false)?;
        ensure_equal(budget.remaining(), 0)?;
        Ok(())
    }

    #[test]
    fn one_budget_executes_one_transition_and_exhausts() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(17), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 1)?;
        ensure_equal(run.pc(), StepIdx::new(1))?;
        ensure_equal(run.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(42)))?;
        Ok(())
    }

    #[test]
    fn copy_preserves_value_and_taint() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(18), &workflow)?;
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::I64(77),
            Taint::DerivedFromSecret,
        )
        .map_err(|error| error.to_string())?;

        let mut store = test_store();
        let signal =
            step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

        ensure_equal(signal, EngineSignal::Continue)?;
        ensure_equal(run.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(77)))?;
        ensure_equal(
            run.read_taint(SlotIdx::new(1)),
            Ok(Taint::DerivedFromSecret),
        )?;
        Ok(())
    }

    #[test]
    fn failed_node_is_marked_failed_on_typed_error() -> Result<(), String> {
        let workflow = copy_workflow(None).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(19), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        ensure_equal(
            result,
            Err(EngineError::MissingOutputSlot {
                step: StepIdx::new(0),
            }),
        )?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
        Ok(())
    }

    #[test]
    fn choose_slot_takes_first_true_branch() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(8), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(true), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(11)) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_slot_takes_later_true_branch() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(10), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(22)) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_slot_takes_otherwise_when_no_branch_matches() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(9), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(99)) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_slot_rejects_non_bool_condition_with_type_mismatch() -> Result<(), String> {
        let workflow = choose_slot_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(11), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
            Err(EngineError::TypeMismatch {
                expected: "boolean",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn choose_slot_without_otherwise_returns_missing_next_step() -> Result<(), String> {
        let workflow =
            choose_slot_without_otherwise_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(12), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(false), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
            Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn choose_expr_takes_first_true_branch() -> Result<(), String> {
        let workflow = choose_expr_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(13), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(11)) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn choose_expr_takes_later_true_branch() -> Result<(), String> {
        let workflow = choose_expr_workflow_with(
            ConstValue::Bool(false),
            ConstValue::Bool(true),
            Some(StepIdx::new(3)),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(20), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(result, EngineSignal::Finished(SlotValue::I64(22)))?;
        Ok(())
    }

    #[test]
    fn choose_expr_takes_otherwise_when_all_false() -> Result<(), String> {
        let workflow = choose_expr_workflow_with(
            ConstValue::Bool(false),
            ConstValue::Bool(false),
            Some(StepIdx::new(3)),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(21), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(result, EngineSignal::Finished(SlotValue::I64(99)))?;
        Ok(())
    }

    #[test]
    fn choose_expr_rejects_non_bool_condition() -> Result<(), String> {
        let workflow = choose_expr_workflow_with(
            ConstValue::I64(1),
            ConstValue::Bool(true),
            Some(StepIdx::new(3)),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(22), &workflow)?;
        let mut store = test_store();

        match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
            Err(EngineError::TypeMismatch {
                expected: "boolean",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn choose_expr_without_otherwise_returns_missing_next_step() -> Result<(), String> {
        let workflow =
            choose_expr_workflow_with(ConstValue::Bool(false), ConstValue::Bool(false), None)
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(25), &workflow)?;
        let mut store = test_store();

        match run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store) {
            Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn public_eval_expr_returns_exact_value() -> Result<(), String> {
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(23), &workflow)?;

        let value =
            eval_expr(&workflow, &run, ExprIdx::new(0)).map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::I64(42))?;
        Ok(())
    }

    #[test]
    fn public_eval_expr_rejects_invalid_expr_index() -> Result<(), String> {
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(26), &workflow)?;

        match eval_expr(&workflow, &run, ExprIdx::new(1)) {
            Err(EngineError::ExprOutOfBounds { expr }) if expr == ExprIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn public_eval_accessor_loads_root_value() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(24), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let value = eval_accessor(&workflow, &run, AccessorIdx::new(0))
            .map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::I64(77))?;
        Ok(())
    }

    #[test]
    fn public_eval_accessor_rejects_invalid_accessor_index() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(27), &workflow)?;

        match eval_accessor(&workflow, &run, AccessorIdx::new(1)) {
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "accessor index out of bounds",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_node_uses_fixed_stack_and_writes_output() -> Result<(), String> {
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(14), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(42)) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn load_accessor_with_empty_path_loads_root_slot() -> Result<(), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(15), &workflow)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::I64(77)) {
            Ok(())
        } else {
            Err(format!("unexpected result: {result:?}"))
        }
    }

    #[test]
    fn public_eval_accessor_reports_typed_error_without_store() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(16), &workflow)?;
        run.write_slot_with_taint(
            SlotIdx::new(0),
            SlotValue::Object(ObjectId::new(0)),
            Taint::Clean,
        )
        .map_err(|error| error.to_string())?;

        match eval_accessor(&workflow, &run, AccessorIdx::new(0)) {
            Err(EngineError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "object",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn load_accessor_reads_object_field_through_store() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(28), &workflow)?;
        let mut store = test_store();
        let object = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(7),
                    value: SlotValue::I64(123),
                }]
                .into_boxed_slice(),
            )
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(result, EngineSignal::Finished(SlotValue::I64(123)))?;
        Ok(())
    }

    #[test]
    fn eval_accessor_reads_list_item_through_store() -> Result<(), String> {
        let workflow = accessor_workflow(vec![PathSegment::Index(1)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(29), &workflow)?;
        let mut store = test_store();
        let list = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let value = eval_accessor_with_store(&workflow, &run, &store, AccessorIdx::new(0))
            .map_err(|error| error.to_string())?;

        ensure_equal(value, SlotValue::I64(2))?;
        Ok(())
    }

    #[test]
    fn eval_accessor_reports_missing_field_precisely() -> Result<(), String> {
        let workflow =
            accessor_workflow(vec![PathSegment::Field(SymbolId::new(9))].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(30), &workflow)?;
        let mut store = test_store();
        let object = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
            .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &store, AccessorIdx::new(0)) {
            Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(9) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_accessor_reports_list_index_precisely() -> Result<(), String> {
        let workflow = accessor_workflow(vec![PathSegment::Index(4)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(31), &workflow)?;
        let mut store = test_store();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
            .map_err(|error| error.to_string())?;

        match eval_accessor_with_store(&workflow, &run, &store, AccessorIdx::new(0)) {
            Err(EngineError::ListIndexOutOfBounds { index: 4 }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    fn tiny_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
        CompiledWorkflow::try_from_parts(tiny_workflow_parts(value))
    }

    fn tiny_workflow_parts(value: ConstValue) -> WorkflowParts {
        WorkflowParts {
            name: Box::<str>::from("tiny"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: tiny_workflow_nodes(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![value].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        }
    }

    fn missing_constant_workflow(
        constant: ConstIdx,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let mut parts = tiny_workflow_parts(ConstValue::Null);
        parts.nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::SetConst { value: constant },
            },
            tiny_finish_node(),
        ]
        .into_boxed_slice();
        CompiledWorkflow::try_from_parts(parts)
    }

    fn tiny_workflow_nodes() -> Box<[CompiledNode]> {
        vec![tiny_set_const_node(), tiny_finish_node()].into_boxed_slice()
    }

    fn tiny_set_const_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }
    }

    fn tiny_finish_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }
    }

    fn choose_slot_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        choose_slot_workflow_with_otherwise(Some(StepIdx::new(3)))
    }

    fn choose_slot_without_otherwise_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        choose_slot_workflow_with_otherwise(None)
    }

    fn choose_slot_workflow_with_otherwise(
        otherwise: Option<StepIdx>,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise,
                },
            },
            set_const_node(1, 2, 0),
            set_const_node(2, 2, 1),
            set_const_node(3, 2, 2),
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ];
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("choose_slot"),
            digest: WorkflowDigest::from_bytes([5; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![
                ConstValue::I64(11),
                ConstValue::I64(22),
                ConstValue::I64(99),
            ]
            .into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn choose_expr_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        choose_expr_workflow_with(
            ConstValue::Bool(true),
            ConstValue::Bool(false),
            Some(StepIdx::new(3)),
        )
    }

    fn choose_expr_workflow_with(
        first: ConstValue,
        second: ConstValue,
        otherwise: Option<StepIdx>,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let true_expr =
            ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
                .map_err(crate::WorkflowError::Expression)?;
        let false_expr =
            ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice())
                .map_err(crate::WorkflowError::Expression)?;
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise,
                },
            },
            set_const_node(1, 2, 2),
            set_const_node(2, 2, 3),
            set_const_node(3, 2, 4),
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            },
        ];
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("choose_expr"),
            digest: WorkflowDigest::from_bytes([6; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: vec![true_expr, false_expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![
                first,
                second,
                ConstValue::I64(11),
                ConstValue::I64(22),
                ConstValue::I64(99),
            ]
            .into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn copy_workflow(output: Option<SlotIdx>) -> Result<CompiledWorkflow, crate::WorkflowError> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("copy"),
            digest: WorkflowDigest::from_bytes([9; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Copy {
                        source: SlotIdx::new(0),
                    },
                },
                tiny_finish_node(),
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn eval_add_workflow() -> Result<CompiledWorkflow, crate::WorkflowError> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ]
            .into_boxed_slice(),
        )
        .map_err(crate::WorkflowError::Expression)?;
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("eval_add"),
            digest: WorkflowDigest::from_bytes([7; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                tiny_finish_node(),
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(19), ConstValue::I64(23)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn accessor_workflow(
        path: Box<[PathSegment]>,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let expression = ExprProgram::try_from_ops(
            vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
        )
        .map_err(crate::WorkflowError::Expression)?;
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("accessor"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::EvalExpr {
                        expr: ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: vec![AccessorProgram {
                root: SlotIdx::new(0),
                path,
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
    }

    fn set_const_node(id: u16, output: u16, constant: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: Some(SlotIdx::new(output)),
            next: Some(StepIdx::new(4)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(constant),
            },
        }
    }

    fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
        new_run_frame(run_id, workflow).map_err(|error| error.to_string())
    }

    fn eval_expr_value(
        ops: Box<[ExprOp]>,
        constants: Box<[ConstValue]>,
    ) -> Result<SlotValue, String> {
        let expression = ExprProgram::try_from_ops(ops).map_err(|error| error.to_string())?;
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("operator_expr"),
            digest: WorkflowDigest::from_bytes([0x5A; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants,
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        })
        .map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(117), &workflow)?;

        eval_expr(&workflow, &run, ExprIdx::new(0)).map_err(|error| error.to_string())
    }

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    // =========================================================================
    // Adversarial BDD tests — engine state machine attack vectors
    // =========================================================================

    // --- StepBudget attack vectors ---

    #[test]
    fn budget_zero_drive_deterministic_returns_step_budget_exhausted_without_touching_frame()
    -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(100), &workflow)?;
        let mut store = test_store();
        let initial_executed = run.executed();
        let initial_pc = run.pc();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), initial_executed)?;
        ensure_equal(run.pc(), initial_pc)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
        Ok(())
    }

    #[test]
    fn budget_one_executes_exactly_one_transition_then_exhausts() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(7)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(101), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 1)?;
        ensure_equal(run.pc(), StepIdx::new(1))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
        ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;
        Ok(())
    }

    #[test]
    fn budget_two_completes_two_step_workflow_with_finish() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(55)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(102), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(2), &mut store);

        ensure_equal(result, Ok(EngineSignal::Finished(SlotValue::I64(55))))?;
        ensure_equal(run.executed(), 2)?;
        Ok(())
    }

    #[test]
    fn step_budget_try_take_returns_false_after_depletion_without_error() -> Result<(), String> {
        let mut budget = StepBudget::new(0);
        let first = budget.try_take().map_err(|error| error.to_string())?;
        ensure_equal(first, false)?;
        ensure_equal(budget.remaining(), 0)?;

        let mut budget_one = StepBudget::new(1);
        let take1 = budget_one.try_take().map_err(|error| error.to_string())?;
        ensure_equal(take1, true)?;
        ensure_equal(budget_one.remaining(), 0)?;
        let take2 = budget_one.try_take().map_err(|error| error.to_string())?;
        ensure_equal(take2, false)?;
        ensure_equal(budget_one.remaining(), 0)?;
        Ok(())
    }

    #[test]
    fn step_budget_max_does_not_overflow_on_consecutive_takes() -> Result<(), String> {
        let mut budget = StepBudget::MAX;
        ensure_equal(budget.remaining(), u64::MAX)?;
        let take = budget.try_take().map_err(|error| error.to_string())?;
        ensure_equal(take, true)?;
        ensure_equal(budget.remaining(), u64::MAX - 1)?;
        Ok(())
    }

    // --- Invalid PC attack vectors ---

    #[test]
    fn step_once_with_invalid_pc_returns_invalid_program_counter() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(103), &workflow)?;
        let mut store = test_store();

        let result = run
            .set_pc(StepIdx::new(99))
            .and_then(|()| step_once(&workflow, &mut run, &mut store));

        match result {
            Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Missing next step attack vector ---

    #[test]
    fn nop_without_next_returns_missing_next_step() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("nop_no_next"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(104), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- SetConst without output slot attack vector ---

    #[test]
    fn set_const_without_output_slot_returns_missing_output_slot() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("set_const_no_output"),
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(105), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::MissingOutputSlot { step }) if step == StepIdx::new(0) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Finish with unreadable slot attack vector ---

    #[test]
    fn finish_with_uninitialized_result_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("finish_empty_slot"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(106), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        // read_slot on an uninitialized slot returns SlotOutOfBounds
        match result {
            Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(0) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Failed step marking attack vector ---

    #[test]
    fn failed_step_is_marked_failed_in_frame_after_engine_error() -> Result<(), String> {
        let workflow = copy_workflow(None).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(107), &workflow)?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
            .map_err(|error| error.to_string())?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        ensure_equal(
            result,
            Err(EngineError::MissingOutputSlot {
                step: StepIdx::new(0),
            }),
        )?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
        Ok(())
    }

    // --- Jump to invalid target attack vector (at runtime, bypassing validation) ---

    #[test]
    fn jump_node_to_out_of_bounds_target_returns_invalid_program_counter_on_next_step()
    -> Result<(), String> {
        // Create a workflow where node 0 is a Jump to node 1, and node 1 is a Finish.
        // Then manually set PC to out-of-bounds before stepping.
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(108), &workflow)?;
        let mut store = test_store();

        let result = run
            .set_pc(StepIdx::new(200))
            .and_then(|()| step_once(&workflow, &mut run, &mut store));

        match result {
            Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(200) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Copy with uninitialized source slot attack vector ---

    #[test]
    fn copy_from_uninitialized_source_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let workflow = copy_workflow(Some(SlotIdx::new(1))).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(109), &workflow)?;
        // Deliberately do NOT initialize slot 0
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(0) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Drive deterministic on a workflow that yields non-Continue signal stops loop ---

    #[test]
    fn drive_deterministic_stops_on_awaiting_action_signal() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("do_node"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(110), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::AwaitingAction))?;
        // AwaitingAction does NOT mark the step succeeded in mark_step_after_signal
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
        Ok(())
    }

    #[test]
    fn drive_deterministic_stops_on_awaiting_wait_signal() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("wait_node"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(111), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::AwaitingWait))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
        Ok(())
    }

    #[test]
    fn drive_deterministic_stops_on_awaiting_ask_signal() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("ask_node"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None,
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(112), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::AwaitingAsk))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))?;
        Ok(())
    }

    // --- Division by zero in expression evaluation ---

    #[test]
    fn eval_expr_division_by_zero_returns_division_by_zero_error() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("div_zero"),
            digest: WorkflowDigest::from_bytes([0x11; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(10), ConstValue::I64(0)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(113), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::DivisionByZero) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Integer overflow in expression evaluation ---

    #[test]
    fn eval_expr_integer_overflow_returns_invalid_compiled_workflow() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Mul,
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("int_overflow"),
            digest: WorkflowDigest::from_bytes([0x22; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(i64::MAX)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(114), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "integer arithmetic overflow",
            }) => {
                ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Type mismatch in boolean expression ---

    #[test]
    fn eval_expr_not_on_non_bool_returns_type_mismatch() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("not_on_int"),
            digest: WorkflowDigest::from_bytes([0x33; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(115), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::TypeMismatch {
                expected: "boolean",
                found: "number",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn eval_expr_operator_truth_table_is_exact() -> Result<(), String> {
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::Eq,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(5)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::NotEq,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(5), ConstValue::I64(6)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::And,
                ]
                .into_boxed_slice(),
                vec![ConstValue::Bool(true), ConstValue::Bool(false)].into_boxed_slice(),
            )?,
            SlotValue::Bool(false),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Or,
                ]
                .into_boxed_slice(),
                vec![ConstValue::Bool(false), ConstValue::Bool(true)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice(),
                vec![ConstValue::Bool(false)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Add,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(11),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Sub,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(3),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Mul,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(28),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Div,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(20), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::I64(5),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Gt,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(7), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Gte,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(4), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Lt,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(3), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        ensure_equal(
            eval_expr_value(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    ExprOp::Lte,
                ]
                .into_boxed_slice(),
                vec![ConstValue::I64(4), ConstValue::I64(4)].into_boxed_slice(),
            )?,
            SlotValue::Bool(true),
        )?;
        Ok(())
    }

    #[test]
    fn eval_expr_load_accessor_traverses_nested_object_list_path() -> Result<(), String> {
        let workflow = accessor_workflow(
            vec![
                PathSegment::Field(SymbolId::new(1)),
                PathSegment::Index(1),
                PathSegment::Field(SymbolId::new(2)),
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(118), &workflow)?;
        let mut store = test_store();
        let nested = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(2),
                    value: SlotValue::Bool(true),
                }]
                .into_boxed_slice(),
            )
            .map_err(|error| error.to_string())?;
        let list = store
            .insert_list(vec![SlotValue::I64(7), SlotValue::Object(nested)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let root = store
            .insert_object(
                vec![ObjectField {
                    key: SymbolId::new(1),
                    value: SlotValue::List(list),
                }]
                .into_boxed_slice(),
            )
            .map_err(|error| error.to_string())?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(root), Taint::Clean)
            .map_err(|error| error.to_string())?;

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|error| error.to_string())?;

        ensure_equal(result, EngineSignal::Finished(SlotValue::Bool(true)))?;
        Ok(())
    }

    #[test]
    fn eval_expr_stack_len_reports_two_after_two_pushes() -> Result<(), String> {
        let mut stack = super::ExprStack::new(2).map_err(|error| error.to_string())?;

        stack
            .push(SlotValue::I64(1))
            .map_err(|error| error.to_string())?;
        stack
            .push(SlotValue::I64(2))
            .map_err(|error| error.to_string())?;

        ensure_equal(stack.len(), 2)?;
        Ok(())
    }

    // --- Unsupported primitive attack vector ---

    #[test]
    fn unimplemented_node_kind_returns_unsupported_primitive() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("unsupported"),
            digest: WorkflowDigest::from_bytes([0x44; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                // ForEachJoin is handled by execute_boundary_node -> falls into the `_` match
                // which returns UnsupportedPrimitive
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(116), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::UnsupportedPrimitive {
                primitive: "not_yet_implemented",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- new_run_frame on valid workflow produces correct frame ---

    #[test]
    fn new_run_frame_produces_frame_with_workflow_entry_and_node_count() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(117), &workflow)?;

        ensure_equal(run.pc(), StepIdx::new(0))?;
        ensure_equal(run.step_count(), 2)?;
        ensure_equal(run.slot_count(), 1)?;
        ensure_equal(run.executed(), 0)?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
        ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;
        Ok(())
    }

    // --- StepBudget new(0) is idempotent ---

    #[test]
    fn step_budget_new_zero_try_take_never_panics_on_repeated_calls() -> Result<(), String> {
        let mut budget = StepBudget::new(0);
        for _ in 0..100 {
            let take = budget.try_take().map_err(|error| error.to_string())?;
            ensure_equal(take, false)?;
        }
        ensure_equal(budget.remaining(), 0)?;
        Ok(())
    }

    // --- Finished signal propagation through drive_deterministic ---

    #[test]
    fn drive_deterministic_propagates_finished_signal_with_exact_slot_value() -> Result<(), String>
    {
        let workflow = tiny_workflow(ConstValue::Bool(false)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(118), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store);

        ensure_equal(result, Ok(EngineSignal::Finished(SlotValue::Bool(false))))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
        ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
        Ok(())
    }

    // --- Expression evaluation with expr out of bounds at runtime ---

    #[test]
    fn eval_expr_node_with_out_of_bounds_expr_returns_expr_out_of_bounds() -> Result<(), String> {
        // Build a workflow with one EvalExpr node referencing ExprIdx(0) but no expressions in pool
        // Validation will catch this, so we need to test through the engine eval_expr function instead
        let workflow = eval_add_workflow().map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(119), &workflow)?;

        let result = eval_expr(&workflow, &run, ExprIdx::new(99));

        match result {
            Err(EngineError::ExprOutOfBounds { expr }) if expr == ExprIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Multi-step chain with budget exhaustion in the middle ---

    #[test]
    fn three_step_chain_with_budget_two_exhausts_after_second_step() -> Result<(), String> {
        // Build a 3-node chain: SetConst -> Nop -> Finish
        let parts = WorkflowParts {
            name: Box::<str>::from("three_step"),
            digest: WorkflowDigest::from_bytes([0x55; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(42)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(120), &workflow)?;
        let mut store = test_store();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(2), &mut store);

        ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
        ensure_equal(run.executed(), 2)?;
        ensure_equal(run.pc(), StepIdx::new(2))?;
        ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
        ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
        ensure_equal(run.step_state(StepIdx::new(2)), Ok(StepState::Pending))?;
        Ok(())
    }

    // =========================================================================
    // Phase 2 adversarial BDD tests — resource exhaustion & security vectors
    // =========================================================================

    // --- Expression stack overflow attack vector ---

    #[test]
    fn eval_expr_stack_at_max_depth_push_beyond_returns_expression_stack_overflow()
    -> Result<(), String> {
        let mut ops = Vec::new();
        for _ in 0..=crate::limits::MAX_EXPRESSION_STACK_USIZE {
            ops.push(ExprOp::LoadConst(ConstIdx::new(0)));
        }
        let expression = ExprProgram::try_from_ops(ops.into_boxed_slice());
        match expression {
            Err(CoreError::ExpressionStackOverflow { .. }) => Ok(()),
            Err(CoreError::InvalidCompiledWorkflow { reason }) if reason.contains("stack") => {
                Ok(())
            }
            other => Err(format!(
                "expected stack overflow during expression validation, got {other:?}"
            )),
        }
    }

    // --- Expression stack underflow detection ---

    #[test]
    fn eval_expr_stack_underflow_on_empty_stack_binary_op_returns_underflow() -> Result<(), String>
    {
        // A bare Add op fails at expression validation time (compile-time), not runtime.
        // Verify it is caught by try_from_ops.
        let result = ExprProgram::try_from_ops(vec![ExprOp::Add].into_boxed_slice());
        match result {
            Err(CoreError::ExpressionStackUnderflow) => Ok(()),
            other => Err(format!(
                "expected ExpressionStackUnderflow from try_from_ops, got {other:?}"
            )),
        }
    }

    // --- ResourceContract with max_steps = 0 rejects any workflow with nodes ---

    #[test]
    fn resource_contract_zero_max_steps_rejects_workflow_with_nodes() -> Result<(), String> {
        let mut contract = crate::ResourceContract::DEFAULT;
        contract.max_steps = 0;
        let parts = WorkflowParts {
            name: Box::<str>::from("zero_max_steps"),
            digest: WorkflowDigest::from_bytes([0xA2; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: contract,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(crate::WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }) => Ok(()),
            other => Err(format!(
                "expected ResourceContractExceeded for max_steps, got {other:?}"
            )),
        }
    }

    // --- ResourceContract with all fields at u16::MAX validates against hard limits ---

    #[test]
    fn resource_contract_all_fields_max_accepted_when_within_hard_limits() -> Result<(), String> {
        let contract = crate::ResourceContract {
            max_steps: u16::try_from(crate::limits::MAX_STEPS_PER_WORKFLOW)
                .map_err(|e| e.to_string())?,
            max_slots: u16::try_from(crate::limits::MAX_SLOTS_PER_WORKFLOW)
                .map_err(|e| e.to_string())?,
            max_constants: u16::try_from(crate::limits::MAX_CONSTANTS)
                .map_err(|e| e.to_string())?,
            max_accessors: u16::try_from(crate::limits::MAX_ACCESSORS)
                .map_err(|e| e.to_string())?,
            max_expressions: u16::try_from(crate::limits::MAX_EXPRESSIONS)
                .map_err(|e| e.to_string())?,
            max_expr_stack: crate::limits::MAX_EXPRESSION_STACK,
            ..crate::ResourceContract::DEFAULT
        };
        let parts = WorkflowParts {
            name: Box::<str>::from("max_contract"),
            digest: WorkflowDigest::from_bytes([0xA3; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: contract,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        ensure_equal(workflow.node_count(), 1)?;
        Ok(())
    }

    // --- ResourceContract max_expr_stack exceeding hard limit rejected ---

    #[test]
    fn resource_contract_max_expr_stack_exceeding_hard_limit_returns_too_large()
    -> Result<(), String> {
        let mut contract = crate::ResourceContract::DEFAULT;
        contract.max_expr_stack = crate::limits::MAX_EXPRESSION_STACK
            .checked_add(1)
            .ok_or("overflow computing test value")?;
        let parts = WorkflowParts {
            name: Box::<str>::from("expr_stack_too_large"),
            digest: WorkflowDigest::from_bytes([0xA4; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: contract,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(crate::WorkflowError::ResourceContractTooLarge {
                resource: "max_expr_stack",
            }) => Ok(()),
            other => Err(format!(
                "expected ResourceContractTooLarge for max_expr_stack, got {other:?}"
            )),
        }
    }

    // --- Crafted workflow with entry point out of bounds rejected ---

    #[test]
    fn workflow_with_entry_out_of_bounds_returns_entry_out_of_bounds() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("bad_entry"),
            digest: WorkflowDigest::from_bytes([0xA5; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(99),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(crate::WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(99) => {
                Ok(())
            }
            other => Err(format!(
                "expected EntryOutOfBounds for step 99, got {other:?}"
            )),
        }
    }

    // --- Empty nodes array rejected ---

    #[test]
    fn workflow_with_empty_nodes_returns_empty_nodes_error() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("empty_nodes"),
            digest: WorkflowDigest::from_bytes([0xA6; 32]),
            nodes: Box::new([]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(crate::WorkflowError::EmptyNodes) => Ok(()),
            other => Err(format!("expected EmptyNodes, got {other:?}")),
        }
    }

    // --- Node ID mismatch detected ---

    #[test]
    fn workflow_with_node_id_mismatch_returns_node_id_mismatch() -> Result<(), String> {
        // Node id mismatch: node at position 0 has id=5, but entry=0 so entry check passes.
        // The NodeIdMismatch check runs after entry validation, so entry must be valid.
        let parts = WorkflowParts {
            name: Box::<str>::from("id_mismatch"),
            digest: WorkflowDigest::from_bytes([0xA7; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(5),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0), // entry is valid (index 0 exists), but node id mismatch
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(crate::WorkflowError::NodeIdMismatch { expected, actual })
                if expected == StepIdx::new(0) && actual == StepIdx::new(5) =>
            {
                Ok(())
            }
            other => Err(format!(
                "expected NodeIdMismatch(expected=0, actual=5), got {other:?}"
            )),
        }
    }

    // --- StepIdx overflow: step target at u16::MAX with only 2 nodes ---

    #[test]
    fn workflow_with_next_target_exceeding_node_count_returns_step_out_of_bounds()
    -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("next_oob"),
            digest: WorkflowDigest::from_bytes([0xA8; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(u16::MAX)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(crate::WorkflowError::StepOutOfBounds { step })
                if step == StepIdx::new(u16::MAX) =>
            {
                Ok(())
            }
            other => Err(format!(
                "expected StepOutOfBounds for u16::MAX, got {other:?}"
            )),
        }
    }

    // --- Expression evaluation leaves more than one value on stack ---

    #[test]
    fn eval_expr_leaving_two_values_on_stack_returns_invalid_compiled_workflow()
    -> Result<(), String> {
        // Expression with two LoadConst ops and no reduction leaves 2 values on the stack.
        // This is caught at expression validation time by try_from_ops.
        let result = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
            ]
            .into_boxed_slice(),
        );
        match result {
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "expression leaves non-single result",
            }) => Ok(()),
            other => Err(format!(
                "expected InvalidCompiledWorkflow('expression leaves non-single result'), got {other:?}"
            )),
        }
    }

    // --- SlotIdx at exact boundary: write to slot_count slot index fails ---

    #[test]
    fn frame_write_slot_at_exact_slot_count_returns_slot_out_of_bounds() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(212), &workflow)?;
        let result = run.write_slot(SlotIdx::new(1), SlotValue::I64(42));
        match result {
            Err(CoreError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => Ok(()),
            other => Err(format!(
                "expected SlotOutOfBounds for slot 1, got {other:?}"
            )),
        }
    }

    // --- RunFrame with zero step_count rejected ---

    #[test]
    fn run_frame_with_zero_step_count_returns_invalid_compiled_workflow() -> Result<(), String> {
        let result = RunFrame::new(RunId::new(213), StepIdx::new(0), 0, 1);
        match result {
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "step_count_zero",
            }) => Ok(()),
            other => Err(format!(
                "expected InvalidCompiledWorkflow for zero steps, got {other:?}"
            )),
        }
    }

    // --- RunFrame with entry step out of bounds rejected ---

    #[test]
    fn run_frame_with_entry_exceeding_step_count_returns_invalid_program_counter()
    -> Result<(), String> {
        let result = RunFrame::new(RunId::new(214), StepIdx::new(5), 4, 1);
        match result {
            Err(CoreError::InvalidProgramCounter { step }) if step == StepIdx::new(5) => Ok(()),
            other => Err(format!(
                "expected InvalidProgramCounter for step 5, got {other:?}"
            )),
        }
    }

    // --- StepIdx::MAX as PC target at runtime ---

    #[test]
    fn step_once_with_stepidx_max_pc_on_small_workflow_returns_invalid_program_counter()
    -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(216), &workflow)?;
        let mut store = test_store();

        let result = run
            .set_pc(StepIdx::new(u16::MAX))
            .and_then(|()| step_once(&workflow, &mut run, &mut store));

        match result {
            Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(u16::MAX) => {
                Ok(())
            }
            other => Err(format!(
                "expected InvalidProgramCounter for u16::MAX, got {other:?}"
            )),
        }
    }

    // --- Integer subtraction overflow ---

    #[test]
    fn eval_expr_subtraction_underflow_returns_invalid_compiled_workflow() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("sub_underflow"),
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: vec![expression].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(i64::MIN), ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(217), &workflow)?;
        let mut store = test_store();

        let result = step_once(&workflow, &mut run, &mut store);

        match result {
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "integer arithmetic overflow",
            }) => Ok(()),
            other => Err(format!(
                "expected integer arithmetic overflow, got {other:?}"
            )),
        }
    }

    // --- Read taint on out-of-bounds slot ---

    #[test]
    fn frame_read_taint_on_out_of_bounds_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(218), &workflow)?;
        let result = run.read_taint(SlotIdx::new(1));
        match result {
            Err(CoreError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => Ok(()),
            other => Err(format!(
                "expected SlotOutOfBounds for taint read, got {other:?}"
            )),
        }
    }

    // --- Write taint on out-of-bounds slot ---

    #[test]
    fn frame_write_taint_on_out_of_bounds_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let mut run = test_frame(RunId::new(219), &workflow)?;
        let result = run.write_taint(SlotIdx::new(5), Taint::Secret);
        match result {
            Err(CoreError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
            other => Err(format!(
                "expected SlotOutOfBounds for taint write, got {other:?}"
            )),
        }
    }

    // --- Step state on out-of-bounds step ---

    #[test]
    fn frame_step_state_on_out_of_bounds_step_returns_step_state_out_of_bounds()
    -> Result<(), String> {
        let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
        let run = test_frame(RunId::new(220), &workflow)?;
        let result = run.step_state(StepIdx::new(99));
        match result {
            Err(CoreError::StepStateOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
            other => Err(format!(
                "expected StepStateOutOfBounds for step 99, got {other:?}"
            )),
        }
    }

    // --- Validate node bounds catches out-of-bounds node id ---

    #[test]
    fn validate_node_bounds_rejects_node_with_id_exceeding_node_count() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("node_id_oob"),
            digest: WorkflowDigest::from_bytes([0xAC; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(5), // id 5 >= node_count(1)
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
        };
        match super::validate_node_bounds(&parts) {
            Err(crate::WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(5) => {
                Ok(())
            }
            other => Err(format!(
                "expected StepOutOfBounds for node id 5, got {other:?}"
            )),
        }
    }
}

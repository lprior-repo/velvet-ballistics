//! Compiled workflow IR.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::{MAX_EXPRESSION_OPS, MAX_EXPRESSION_STACK};
use crate::value::ConstValue;
use thiserror::Error;

/// Immutable compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    nodes: Box<[CompiledNode]>,
    expressions: Box<[ExprProgram]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
    entry: StepIdx,
}

impl CompiledWorkflow {
    /// Creates a compiled workflow after validating all numeric references.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validate_parts(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            nodes: parts.nodes,
            expressions: parts.expressions,
            constants: parts.constants,
            slot_count: parts.slot_count,
            entry: parts.entry,
        })
    }

    /// Workflow name retained for cold diagnostics.
    #[must_use]
    pub const fn name(&self) -> &str {
        &self.name
    }

    /// Compiled workflow digest.
    #[must_use]
    pub const fn digest(&self) -> WorkflowDigest {
        self.digest
    }

    /// Entry step for new runs.
    #[must_use]
    pub const fn entry(&self) -> StepIdx {
        self.entry
    }

    /// Number of slots required by each run frame.
    #[must_use]
    pub const fn slot_count(&self) -> u16 {
        self.slot_count
    }

    /// Returns a checked node reference.
    #[must_use]
    pub fn node(&self, step: StepIdx) -> Option<&CompiledNode> {
        self.nodes.get(step.as_usize())
    }

    /// Returns a checked expression program reference.
    #[must_use]
    pub fn expression(&self, expr: ExprIdx) -> Option<&ExprProgram> {
        self.expressions.get(expr.as_usize())
    }

    /// Returns a checked constant reference.
    #[must_use]
    pub fn constant(&self, constant: ConstIdx) -> Option<&ConstValue> {
        self.constants.get(constant.as_usize())
    }
}

/// Untrusted compiled workflow parts emitted by a compiler boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowParts {
    /// Workflow name retained for cold diagnostics.
    pub name: Box<str>,
    /// Compiled workflow digest.
    pub digest: WorkflowDigest,
    /// Numeric nodes.
    pub nodes: Box<[CompiledNode]>,
    /// Expression bytecode table.
    pub expressions: Box<[ExprProgram]>,
    /// Constant pool.
    pub constants: Box<[ConstValue]>,
    /// Number of runtime slots.
    pub slot_count: u16,
    /// Entry step.
    pub entry: StepIdx,
}

/// Workflow IR validation failures.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WorkflowError {
    /// The compiler emitted no nodes.
    #[error("compiled workflow must contain at least one node")]
    EmptyNodes,
    /// Entry step is outside the node array.
    #[error("entry step {entry:?} is outside the node array")]
    EntryOutOfBounds {
        /// Invalid entry step.
        entry: StepIdx,
    },
    /// A node target step is outside the node array.
    #[error("node target step {step:?} is outside the node array")]
    StepOutOfBounds {
        /// Invalid target step.
        step: StepIdx,
    },
    /// A slot reference is outside the run frame.
    #[error("slot {slot:?} is outside slot_count")]
    SlotOutOfBounds {
        /// Invalid slot.
        slot: SlotIdx,
    },
    /// A constant reference is outside the constant pool.
    #[error("constant {constant:?} is outside the constant pool")]
    ConstOutOfBounds {
        /// Invalid constant.
        constant: ConstIdx,
    },
    /// Expression program failed bytecode validation.
    #[error("expression program is invalid: {0}")]
    Expression(#[from] CoreError),
}

/// Bounded postfix expression bytecode program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprProgram {
    /// Postfix bytecode operations.
    pub ops: Box<[ExprOp]>,
    /// Maximum stack entries required by this program.
    pub max_stack: u8,
}

impl ExprProgram {
    /// Builds a program and computes the exact required stack depth.
    pub fn try_from_ops(ops: Box<[ExprOp]>) -> CoreResult<Self> {
        let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
        Ok(Self { ops, max_stack })
    }

    /// Builds a program from untrusted parts and rejects stale stack metadata.
    pub fn try_from_parts(ops: Box<[ExprOp]>, max_stack: u8) -> CoreResult<Self> {
        let computed = check_expr_stack_bound(&ops, max_stack)?;
        if computed == max_stack {
            Ok(Self { ops, max_stack })
        } else {
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "expression max_stack mismatch",
            })
        }
    }
}

/// Postfix expression bytecode operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprOp {
    /// Push a runtime slot value.
    LoadSlot(SlotIdx),
    /// Push a constant-pool value.
    LoadConst(ConstIdx),
    /// Push a value resolved by an accessor program.
    LoadAccessor(AccessorIdx),
    /// Equality comparison.
    Eq,
    /// Inequality comparison.
    NotEq,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Gte,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Lte,
    /// Boolean conjunction.
    And,
    /// Boolean disjunction.
    Or,
    /// Boolean negation.
    Not,
    /// Numeric addition.
    Add,
    /// Numeric subtraction.
    Sub,
    /// Numeric multiplication.
    Mul,
    /// Numeric division.
    Div,
    /// `contains` helper.
    Contains,
    /// `starts_with` helper.
    StartsWith,
    /// `ends_with` helper.
    EndsWith,
    /// `has` helper.
    Has,
    /// `exists` helper.
    Exists,
    /// `length` helper.
    Length,
    /// `empty` helper.
    Empty,
    /// `append` helper.
    Append,
    /// `append_if` helper.
    AppendIf,
    /// `merge` helper.
    Merge,
    /// `sum` helper.
    Sum,
    /// `count` helper.
    Count,
    /// `unique` helper.
    Unique,
}

/// Validates stack effects and returns the exact required stack depth.
pub fn check_expr_stack_bound(ops: &[ExprOp], capacity: u8) -> CoreResult<u8> {
    validate_expr_op_count(ops)?;
    let mut depth = 0u8;
    let mut required = 0u8;
    for op in ops {
        depth = apply_expr_stack_effect(depth, *op)?;
        required = required.max(depth);
        validate_expr_stack_capacity(required, capacity)?;
    }
    validate_expr_final_depth(depth)?;
    Ok(required)
}

/// One compiled state-machine node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledNode {
    /// Step index of this node.
    pub id: StepIdx,
    /// Node behavior.
    pub kind: CompiledNodeKind,
}

/// Hot-path node variants. All references are numeric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledNodeKind {
    /// Write a constant-pool value into the output slot.
    SetConst {
        /// Output slot.
        output: SlotIdx,
        /// Constant-pool index.
        value: ConstIdx,
        /// Fallthrough step.
        next: StepIdx,
    },
    /// Copy one slot into the output slot.
    Copy {
        /// Output slot.
        output: SlotIdx,
        /// Source slot.
        source: SlotIdx,
        /// Fallthrough step.
        next: StepIdx,
    },
    /// Branch using a pre-materialized boolean condition slot.
    ChooseSlot {
        /// Condition slot.
        condition: SlotIdx,
        /// Target when condition is true.
        on_true: StepIdx,
        /// Target when condition is false.
        on_false: StepIdx,
    },
    /// Finish the run with the selected result slot.
    Finish {
        /// Result slot.
        result: SlotIdx,
    },
    /// Finish the run with a constant-pool value.
    FinishConst {
        /// Result constant.
        value: ConstIdx,
    },
}

fn validate_parts(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if parts.nodes.is_empty() {
        return Err(WorkflowError::EmptyNodes);
    }
    validate_entry(parts.entry, parts.nodes.len())?;
    validate_expressions(&parts.expressions)?;
    for node in parts.nodes.as_ref() {
        validate_node(node, parts)?;
    }
    Ok(())
}

fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    validate_step(entry, node_count).map_err(|_| WorkflowError::EntryOutOfBounds { entry })
}

fn validate_node(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    match node.kind {
        CompiledNodeKind::SetConst {
            output,
            value,
            next,
        } => validate_set_const(output, value, next, parts),
        CompiledNodeKind::Copy {
            output,
            source,
            next,
        } => validate_copy(output, source, next, parts),
        CompiledNodeKind::ChooseSlot {
            condition,
            on_true,
            on_false,
        } => validate_choose(condition, on_true, on_false, parts),
        CompiledNodeKind::Finish { result } => validate_slot(result, parts.slot_count),
        CompiledNodeKind::FinishConst { value } => validate_const(value, parts.constants.len()),
    }
}

fn validate_set_const(
    output: SlotIdx,
    value: ConstIdx,
    next: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(output, parts.slot_count)?;
    validate_const(value, parts.constants.len())?;
    validate_step(next, parts.nodes.len())
}

fn validate_copy(
    output: SlotIdx,
    source: SlotIdx,
    next: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(output, parts.slot_count)?;
    validate_slot(source, parts.slot_count)?;
    validate_step(next, parts.nodes.len())
}

fn validate_choose(
    condition: SlotIdx,
    on_true: StepIdx,
    on_false: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(condition, parts.slot_count)?;
    validate_step(on_true, parts.nodes.len())?;
    validate_step(on_false, parts.nodes.len())
}

fn validate_step(step: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    if step.as_usize() < node_count {
        Ok(())
    } else {
        Err(WorkflowError::StepOutOfBounds { step })
    }
}

fn validate_slot(slot: SlotIdx, slot_count: u16) -> Result<(), WorkflowError> {
    if slot.as_usize() < usize::from(slot_count) {
        Ok(())
    } else {
        Err(WorkflowError::SlotOutOfBounds { slot })
    }
}

fn validate_const(constant: ConstIdx, const_count: usize) -> Result<(), WorkflowError> {
    if constant.as_usize() < const_count {
        Ok(())
    } else {
        Err(WorkflowError::ConstOutOfBounds { constant })
    }
}

fn validate_expressions(expressions: &[ExprProgram]) -> Result<(), WorkflowError> {
    for expression in expressions {
        ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
    }
    Ok(())
}

fn validate_expr_op_count(ops: &[ExprOp]) -> CoreResult<()> {
    if ops.len() > MAX_EXPRESSION_OPS {
        Err(CoreError::ResourceLimitExceeded {
            resource: "expression ops",
        })
    } else {
        Ok(())
    }
}

fn apply_expr_stack_effect(depth: u8, op: ExprOp) -> CoreResult<u8> {
    let effect = expr_stack_effect(op);
    let consumed = depth
        .checked_sub(effect.pop)
        .ok_or(CoreError::ExpressionStackUnderflow)?;
    consumed
        .checked_add(effect.push)
        .ok_or(CoreError::ExpressionStackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
}

fn validate_expr_stack_capacity(required: u8, capacity: u8) -> CoreResult<()> {
    if required <= capacity && required <= MAX_EXPRESSION_STACK {
        Ok(())
    } else {
        Err(CoreError::ExpressionStackOverflow { max: capacity })
    }
}

fn validate_expr_final_depth(depth: u8) -> CoreResult<()> {
    match depth {
        0 => Err(CoreError::ExpressionStackUnderflow),
        1 => Ok(()),
        _ => Err(CoreError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StackEffect {
    pop: u8,
    push: u8,
}

const fn expr_stack_effect(op: ExprOp) -> StackEffect {
    match op {
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => effect(0, 1),
        ExprOp::Not
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => effect(1, 1),
        ExprOp::AppendIf => effect(3, 1),
        _ => effect(2, 1),
    }
}

const fn effect(pop: u8, push: u8) -> StackEffect {
    StackEffect { pop, push }
}

#[cfg(test)]
mod tests {
    use super::{ExprOp, ExprProgram, check_expr_stack_bound};
    use crate::errors::CoreError;
    use crate::ids::ConstIdx;

    #[test]
    fn expr_program_tracks_binary_stack_depth() -> Result<(), String> {
        let ops = vec![load(0), load(1), ExprOp::Eq].into_boxed_slice();

        let program = ExprProgram::try_from_ops(ops).map_err(|error| error.to_string())?;

        if program.max_stack == 2 {
            Ok(())
        } else {
            Err(format!("unexpected max stack: {}", program.max_stack))
        }
    }

    #[test]
    fn expr_program_rejects_unary_underflow() -> Result<(), String> {
        match ExprProgram::try_from_ops(vec![ExprOp::Not].into_boxed_slice()) {
            Err(CoreError::ExpressionStackUnderflow) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn expr_program_rejects_binary_underflow() -> Result<(), String> {
        let ops = vec![load(0), ExprOp::Eq].into_boxed_slice();

        match ExprProgram::try_from_ops(ops) {
            Err(CoreError::ExpressionStackUnderflow) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn expr_program_rejects_capacity_overflow() -> Result<(), String> {
        let ops = vec![load(0), load(1)].into_boxed_slice();

        match check_expr_stack_bound(&ops, 1) {
            Err(CoreError::ExpressionStackOverflow { max: 1 }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn expr_program_rejects_extra_final_value() -> Result<(), String> {
        let ops = vec![load(0), load(1)].into_boxed_slice();

        match ExprProgram::try_from_ops(ops) {
            Err(CoreError::InvalidCompiledWorkflow { .. }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn expr_program_rejects_op_limit() -> Result<(), String> {
        let ops = vec![load(0); 257].into_boxed_slice();

        match ExprProgram::try_from_ops(ops) {
            Err(CoreError::ResourceLimitExceeded { resource: "expression ops" }) => {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn expr_program_rejects_stale_max_stack_metadata() -> Result<(), String> {
        let ops = vec![load(0), load(1), ExprOp::Eq].into_boxed_slice();

        match ExprProgram::try_from_parts(ops, 3) {
            Err(CoreError::InvalidCompiledWorkflow { .. }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    fn load(index: u16) -> ExprOp {
        ExprOp::LoadConst(ConstIdx::new(index))
    }
}

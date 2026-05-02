//! Compiled workflow IR.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{
    AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::{
    MAX_ACCESSORS, MAX_CONSTANTS, MAX_EXPRESSIONS, MAX_EXPRESSION_OPS, MAX_EXPRESSION_STACK,
    MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_SLOTS_PER_WORKFLOW,
    MAX_STEPS_PER_WORKFLOW,
};
use crate::value::ConstValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Immutable compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    nodes: Box<[CompiledNode]>,
    expressions: Box<[ExprProgram]>,
    accessors: Box<[AccessorProgram]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
    entry: StepIdx,
    resource_contract: ResourceContract,
}

impl CompiledWorkflow {
    /// Creates a compiled workflow after validating all numeric references.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validate_parts(&parts)?;
        validate_budget(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
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

    /// Number of compiled nodes.
    #[must_use]
    pub fn node_count(&self) -> u16 {
        u16::try_from(self.nodes.len()).map_or(u16::MAX, |value| value)
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

    /// Returns a checked accessor program reference.
    #[must_use]
    pub fn accessor(&self, accessor: AccessorIdx) -> Option<&AccessorProgram> {
        self.accessors.get(accessor.as_usize())
    }

    /// Returns a checked constant reference.
    #[must_use]
    pub fn constant(&self, constant: ConstIdx) -> Option<&ConstValue> {
        self.constants.get(constant.as_usize())
    }

    /// Explicit compiled resource bounds for admission and allocation.
    #[must_use]
    pub const fn resource_contract(&self) -> ResourceContract {
        self.resource_contract
    }

    /// Converts back to the serializable parts representation for artifact emission.
    #[must_use]
    pub fn to_parts(&self) -> WorkflowParts {
        WorkflowParts {
            name: self.name.clone(),
            digest: self.digest,
            nodes: self.nodes.clone(),
            expressions: self.expressions.clone(),
            accessors: self.accessors.clone(),
            constants: self.constants.clone(),
            slot_count: self.slot_count,
            entry: self.entry,
            resource_contract: self.resource_contract,
        }
    }
}

/// Explicit compiled resource bounds accepted at run admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContract {
    /// Maximum node count admitted for this workflow.
    pub max_steps: u16,
    /// Maximum runtime slot count admitted for this workflow.
    pub max_slots: u16,
    /// Maximum constant-pool entries admitted for this workflow.
    pub max_constants: u16,
    /// Maximum accessor programs admitted for this workflow.
    pub max_accessors: u16,
    /// Maximum expression programs admitted for this workflow.
    pub max_expressions: u16,
    /// Maximum expression stack entries admitted for this workflow.
    pub max_expr_stack: u8,
    /// Maximum deterministic transitions per runtime tick.
    pub max_step_budget_per_tick: u64,
    /// Maximum input bytes accepted at admission.
    pub max_input_bytes: u32,
    /// Maximum output bytes produced by a run.
    pub max_output_bytes: u32,
    /// Maximum blob payload bytes.
    pub max_blob_bytes: u64,
    /// Maximum IPC payload bytes.
    pub max_ipc_payload_bytes: u32,
    /// Maximum retry attempts for action policies.
    pub max_retry_attempts: u16,
    /// Maximum branch fanout.
    pub max_fanout: u16,
    /// Maximum collect items.
    pub max_collect_items: u32,
    /// Maximum runtime queue depth.
    pub max_queue_depth: u32,
    /// Maximum journal batch bytes.
    pub max_journal_batch_bytes: u32,
}

impl ResourceContract {
    /// Conservative default used until policy profiles become user-configurable.
    pub const DEFAULT: Self = Self {
        max_steps: 10_000,
        max_slots: 1_024,
        max_constants: u16::MAX,
        max_accessors: 8_192,
        max_expressions: 4_096,
        max_expr_stack: 64,
        max_step_budget_per_tick: 10_000,
        max_input_bytes: 1_048_576,
        max_output_bytes: 1_048_576,
        max_blob_bytes: 16_777_216,
        max_ipc_payload_bytes: 1_048_576,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: 1_024,
        max_queue_depth: 1_024,
        max_journal_batch_bytes: 1_048_576,
    };
}

/// Expression branch target used by final choose IR scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExprBranch {
    /// Expression condition index.
    pub condition: ExprIdx,
    /// Target node when the condition is true.
    pub target: StepIdx,
}

/// Materialized boolean-slot branch target used by final choose IR scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotBranch {
    /// Boolean condition slot.
    pub condition: SlotIdx,
    /// Target node when the condition is true.
    pub target: StepIdx,
}

/// Untrusted compiled workflow parts emitted by a compiler boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowParts {
    /// Workflow name retained for cold diagnostics.
    pub name: Box<str>,
    /// Compiled workflow digest.
    pub digest: WorkflowDigest,
    /// Numeric nodes.
    pub nodes: Box<[CompiledNode]>,
    /// Expression bytecode table.
    pub expressions: Box<[ExprProgram]>,
    /// Accessor bytecode table.
    pub accessors: Box<[AccessorProgram]>,
    /// Constant pool.
    pub constants: Box<[ConstValue]>,
    /// Number of runtime slots.
    pub slot_count: u16,
    /// Entry step.
    pub entry: StepIdx,
    /// Explicit resource bounds carried with the compiled artifact.
    pub resource_contract: ResourceContract,
}

/// Bounded accessor program for slot-rooted path traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessorProgram {
    /// Root slot for the traversal.
    pub root: SlotIdx,
    /// Bounded path from root to selected value.
    pub path: Box<[PathSegment]>,
}

/// One path segment in an accessor program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathSegment {
    /// Object field by interned symbol.
    Field(SymbolId),
    /// List index.
    Index(u32),
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
    /// Node identity does not match its table position.
    #[error("node id mismatch: expected {expected:?}, found {actual:?}")]
    NodeIdMismatch {
        /// Expected node id for this table position.
        expected: StepIdx,
        /// Actual node id emitted by the compiler.
        actual: StepIdx,
    },
    /// Expression program failed bytecode validation.
    #[error("expression program is invalid: {0}")]
    Expression(#[from] CoreError),
    /// Resource contract does not cover the compiled artifact.
    #[error("resource contract exceeded: {resource}")]
    ResourceContractExceeded {
        /// Resource name.
        resource: &'static str,
    },
    /// Resource contract exceeds protocol hard limits.
    #[error("resource contract exceeds hard limit: {resource}")]
    ResourceContractTooLarge {
        /// Resource name.
        resource: &'static str,
    },
    /// Branching node has no branch and no otherwise route.
    #[error("branch table must contain a branch or otherwise target")]
    EmptyBranchTable,
    /// A node is not reachable from the entry step.
    #[error("node {step:?} is not reachable from the entry step")]
    UnreachableNode {
        /// Unreachable step index.
        step: StepIdx,
    },
    /// An edge points backward without being a recognized loop back-edge.
    #[error("backward edge from {from:?} to {to:?}")]
    BackwardEdge {
        /// Source step of the backward edge.
        from: StepIdx,
        /// Target step of the backward edge.
        to: StepIdx,
    },
    /// An inner loop exceeds its outer loop span.
    #[error("inner loop at {inner:?} exceeds outer loop done at {outer_done:?}")]
    ImproperLoopNesting {
        /// Inner loop start step.
        inner: StepIdx,
        /// Outer loop done step.
        outer_done: StepIdx,
    },
    /// Whole-workflow budget exceeded the boundedness policy.
    #[error("budget policy exceeded: {detail}")]
    BudgetPolicyExceeded {
        /// Human-readable detail describing which dimension failed.
        detail: &'static str,
    },
}

/// Bounded postfix expression bytecode program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledNode {
    /// Step index of this node.
    pub id: StepIdx,
    /// Optional output slot written by this node.
    pub output: Option<SlotIdx>,
    /// Optional fallthrough step.
    pub next: Option<StepIdx>,
    /// Node behavior.
    pub kind: CompiledNodeKind,
}

/// Hot-path node variants. All references are numeric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum CompiledNodeKind {
    /// No-op transition to `next`.
    Nop,
    /// Write a constant-pool value into the output slot.
    SetConst {
        /// Constant-pool index.
        value: ConstIdx,
    },
    /// Copy one slot into the output slot.
    Copy {
        /// Source slot.
        source: SlotIdx,
    },
    /// Evaluate expression bytecode into `output`.
    EvalExpr { expr: ExprIdx },
    /// Build an object handle from numeric field and slot references.
    BuildObject {
        fields: Box<[(crate::ids::SymbolId, SlotIdx)]>,
    },
    /// Build a list handle from numeric slot references.
    BuildList { items: Box<[SlotIdx]> },
    /// Schedule an external action and suspend.
    Do { action: ActionId, input: SlotIdx },
    /// Branch using a pre-materialized boolean condition slot.
    Choose {
        /// Ordered expression branches.
        branches: Box<[ExprBranch]>,
        /// Target when no branch condition is true.
        otherwise: Option<StepIdx>,
    },
    /// Branch using pre-materialized boolean condition slots.
    ChooseSlot {
        /// Ordered slot branches.
        branches: Box<[SlotBranch]>,
        /// Target when no branch condition is true.
        otherwise: Option<StepIdx>,
    },
    /// Start a bounded for-each loop.
    ForEachStart {
        input: SlotIdx,
        item_slot: SlotIdx,
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance a bounded for-each loop.
    ForEachNext {
        iterator_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Join a for-each loop output.
    ForEachJoin { output: SlotIdx },
    /// Start bounded parallel branches.
    TogetherStart {
        branches: Box<[StepIdx]>,
        join: StepIdx,
    },
    /// Execute one together branch.
    TogetherBranch {
        branch: u16,
        entry: StepIdx,
        join: StepIdx,
        /// Slot holding the accumulator list (shared with TogetherStart).
        accumulator: SlotIdx,
    },
    /// Join together branches.
    TogetherJoin {
        branch_count: u16,
        /// Slot holding the accumulator list (shared with TogetherStart).
        accumulator: SlotIdx,
    },
    /// Start bounded collection.
    CollectStart {
        source: SlotIdx,
        limit: u32,
        page_size: u32,
        body: StepIdx,
        done: StepIdx,
    },
    /// Process one collection page.
    CollectPage {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance collection.
    CollectNext {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Finish collection.
    CollectFinish { collector_slot: SlotIdx },
    /// Start bounded reduction.
    ReduceStart {
        input: SlotIdx,
        accumulator: SlotIdx,
        initial: ConstIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance reduction.
    ReduceNext {
        iterator_slot: SlotIdx,
        accumulator: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Finish reduction.
    ReduceFinish { accumulator: SlotIdx },
    /// Start bounded repeat.
    RepeatStart {
        max_attempts: u16,
        body: StepIdx,
        done: StepIdx,
    },
    /// Execute repeat attempt.
    RepeatAttempt {
        attempt_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Check repeat state.
    RepeatCheck {
        attempt_slot: SlotIdx,
        done: StepIdx,
    },
    /// Finish repeat.
    RepeatFinish { result: SlotIdx },
    /// Wait until a deadline slot.
    WaitUntil { deadline_slot: SlotIdx },
    /// Wait for an event slot.
    WaitEvent {
        event: SlotIdx,
        timeout_slot: Option<SlotIdx>,
    },
    /// Ask for external input.
    Ask {
        prompt: SlotIdx,
        timeout_slot: Option<SlotIdx>,
    },
    /// Resume an ask.
    AskResume { answer: SlotIdx },
    /// Check retry policy.
    RetryCheck {
        policy_slot: SlotIdx,
        body: StepIdx,
        exhausted: StepIdx,
    },
    /// Run error handler.
    ErrorHandler { body: StepIdx, handler: StepIdx },
    /// Jump to a numeric target.
    Jump { target: StepIdx },
    /// Finish the run with the selected result slot.
    Finish {
        /// Result slot.
        result: SlotIdx,
    },
}

fn validate_parts(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if parts.nodes.is_empty() {
        return Err(WorkflowError::EmptyNodes);
    }
    validate_resource_contract(parts)?;
    validate_entry(parts.entry, parts.nodes.len())?;
    validate_expressions(&parts.expressions, parts.accessors.len())?;
    validate_accessors(&parts.accessors, parts.slot_count)?;
    for (index, node) in parts.nodes.iter().enumerate() {
        validate_node_id(node, index)?;
        validate_node(node, parts)?;
    }
    validate_accessor_path_symbols(&parts.accessors)?;
    validate_reachability(parts)?;
    validate_forward_edges(parts)?;
    Ok(())
}

fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    use crate::budget::{BoundednessPolicy, BudgetError, WholeWorkflowBudget};

    let budget = WholeWorkflowBudget::compute(
        &parts.nodes,
        parts.entry,
        &parts.resource_contract,
    )?;

    match BoundednessPolicy::DEFAULT.validate(&budget) {
        Ok(()) => Ok(()),
        Err(BudgetError::TotalStepsExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_total_steps",
        }),
        Err(BudgetError::TotalSlotsExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_total_slots",
        }),
        Err(BudgetError::FanoutExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_fanout",
        }),
        Err(BudgetError::NestingDepthExceeded { .. }) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: "max_nesting_depth",
        }),
    }
}

fn validate_node_id(node: &CompiledNode, index: usize) -> Result<(), WorkflowError> {
    if node.id.as_usize() == index {
        Ok(())
    } else {
        Err(WorkflowError::NodeIdMismatch {
            expected: StepIdx::new(u16::try_from(index).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
            actual: node.id,
        })
    }
}

fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    validate_resource_counts(parts, contract)?;
    validate_expr_stack_contract(parts.expressions.as_ref(), contract.max_expr_stack)
}

fn validate_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_primary_resource_counts(parts, contract)?;
    validate_expression_resource_counts(parts, contract)
}

fn validate_primary_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_contract_limit(
        "max_steps",
        parts.nodes.len(),
        usize::from(contract.max_steps),
        MAX_STEPS_PER_WORKFLOW,
    )?;
    validate_contract_limit(
        "max_slots",
        usize::from(parts.slot_count),
        usize::from(contract.max_slots),
        MAX_SLOTS_PER_WORKFLOW,
    )?;
    validate_contract_limit(
        "max_constants",
        parts.constants.len(),
        usize::from(contract.max_constants),
        MAX_CONSTANTS,
    )
}

fn validate_expression_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_contract_limit(
        "max_accessors",
        parts.accessors.len(),
        usize::from(contract.max_accessors),
        MAX_ACCESSORS,
    )?;
    validate_contract_limit(
        "max_expressions",
        parts.expressions.len(),
        usize::from(contract.max_expressions),
        MAX_EXPRESSIONS,
    )
}

fn validate_contract_limit(
    resource: &'static str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> Result<(), WorkflowError> {
    if declared > hard_limit {
        return Err(WorkflowError::ResourceContractTooLarge { resource });
    }
    if actual > declared {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

fn validate_expr_stack_contract(
    expressions: &[ExprProgram],
    max_expr_stack: u8,
) -> Result<(), WorkflowError> {
    if max_expr_stack > MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expr_stack",
        });
    }
    if expressions
        .iter()
        .any(|expression| expression.max_stack > max_expr_stack)
    {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_expr_stack",
        })
    } else {
        Ok(())
    }
}

fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    validate_step(entry, node_count).map_err(|_| WorkflowError::EntryOutOfBounds { entry })
}

#[allow(clippy::match_same_arms)] // Field names differ; bodies are structurally identical but not mergeable
fn validate_node(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_optional_slot(node.output, parts.slot_count)?;
    validate_optional_step(node.next, parts.nodes.len())?;
    match &node.kind {
        CompiledNodeKind::Nop => Ok(()),
        CompiledNodeKind::SetConst { value } => validate_const(*value, parts.constants.len()),
        CompiledNodeKind::Copy { source } => validate_slot(*source, parts.slot_count),
        CompiledNodeKind::EvalExpr { expr } => validate_expr(*expr, parts.expressions.len()),
        CompiledNodeKind::BuildObject { fields } => validate_build_object(fields, parts),
        CompiledNodeKind::BuildList { items } => validate_build_list(items, parts.slot_count),
        CompiledNodeKind::Do { action: _, input } => validate_slot(*input, parts.slot_count),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_slot_choose(branches, *otherwise, parts),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_expr_choose(branches, *otherwise, parts),
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit: _,
            body,
            done,
        } => validate_for_each_start(*input, *item_slot, *body, *done, parts),
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => validate_slot_and_steps(*iterator_slot, *body, *done, parts),
        CompiledNodeKind::ForEachJoin { output } => validate_slot(*output, parts.slot_count),
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together(branches, *join, parts)
        }
        CompiledNodeKind::TogetherBranch {
            branch: _,
            entry,
            join,
            accumulator,
        } => {
            validate_two_steps(*entry, *join, parts)?;
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            validate_nonzero_u16(*branch_count, "branch_count")?;
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::CollectStart {
            source,
            limit: _,
            page_size: _,
            body,
            done,
        } => validate_slot_and_steps(*source, *body, *done, parts),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        }
        | CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => validate_slot_and_steps(*collector_slot, *body, *done, parts),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            validate_slot(*collector_slot, parts.slot_count)
        }
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => validate_reduce_start(*input, *accumulator, *initial, *body, *done, parts),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => validate_reduce_next(*iterator_slot, *accumulator, *body, *done, parts),
        CompiledNodeKind::ReduceFinish { accumulator } => {
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => validate_repeat_start(*max_attempts, *body, *done, parts),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => validate_slot_and_steps(*attempt_slot, *body, *done, parts),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            validate_slot(*attempt_slot, parts.slot_count)?;
            validate_step(*done, parts.nodes.len())
        }
        CompiledNodeKind::RepeatFinish { result } => validate_slot(*result, parts.slot_count),
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            validate_slot(*deadline_slot, parts.slot_count)
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            validate_slot(*event, parts.slot_count)?;
            validate_optional_slot(*timeout_slot, parts.slot_count)
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            validate_slot(*prompt, parts.slot_count)?;
            validate_optional_slot(*timeout_slot, parts.slot_count)
        }
        CompiledNodeKind::AskResume { answer } => validate_slot(*answer, parts.slot_count),
        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => validate_slot_and_steps(*policy_slot, *body, *exhausted, parts),
        CompiledNodeKind::ErrorHandler { body, handler } => {
            validate_two_steps(*body, *handler, parts)
        }
        CompiledNodeKind::Jump { target } => validate_step(*target, parts.nodes.len()),
        CompiledNodeKind::Finish { result } => validate_slot(*result, parts.slot_count),
    }
}

fn validate_optional_slot(slot: Option<SlotIdx>, slot_count: u16) -> Result<(), WorkflowError> {
    slot.map_or(Ok(()), |value| validate_slot(value, slot_count))
}

fn validate_slots(slots: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    for slot in slots {
        validate_slot(*slot, slot_count)?;
    }
    Ok(())
}

fn validate_build_list(items: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    if items.len() > MAX_LIST_ITEMS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "list_items",
        });
    }
    validate_slots(items, slot_count)
}

fn validate_build_object(
    fields: &[(crate::ids::SymbolId, SlotIdx)],
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    if fields.len() > MAX_OBJECT_FIELDS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "object_fields",
        });
    }
    for (_, slot) in fields {
        validate_slot(*slot, parts.slot_count)?;
    }
    Ok(())
}

fn validate_for_each_start(
    input: SlotIdx,
    item_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(item_slot, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

fn validate_slot_and_steps(
    slot: SlotIdx,
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(slot, parts.slot_count)?;
    validate_two_steps(first, second, parts)
}

fn validate_two_steps(
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_step(first, parts.nodes.len())?;
    validate_step(second, parts.nodes.len())
}

fn validate_together(
    branches: &[StepIdx],
    join: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), Some(join))?;
    for branch in branches {
        validate_step(*branch, parts.nodes.len())?;
    }
    validate_step(join, parts.nodes.len())
}

fn validate_nonzero_u16(value: u16, resource: &'static str) -> Result<(), WorkflowError> {
    if value == 0 {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

fn validate_reduce_start(
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_const(initial, parts.constants.len())?;
    validate_two_steps(body, done, parts)
}

fn validate_reduce_next(
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(iterator_slot, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

fn validate_repeat_start(
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_nonzero_u16(max_attempts, "max_retry_attempts")?;
    validate_two_steps(body, done, parts)
}

fn validate_slot_choose(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise)?;
    branches.iter().try_for_each(|branch| {
        validate_slot(branch.condition, parts.slot_count)?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step(otherwise, parts.nodes.len())
}

fn validate_expr_choose(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise)?;
    branches.iter().try_for_each(|branch| {
        validate_expr(branch.condition, parts.expressions.len())?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step(otherwise, parts.nodes.len())
}

fn validate_branch_route(
    branch_count: usize,
    otherwise: Option<StepIdx>,
) -> Result<(), WorkflowError> {
    if branch_count == 0 && otherwise.is_none() {
        Err(WorkflowError::EmptyBranchTable)
    } else {
        Ok(())
    }
}

fn validate_optional_step(step: Option<StepIdx>, node_count: usize) -> Result<(), WorkflowError> {
    step.map_or(Ok(()), |target| validate_step(target, node_count))
}

fn validate_expr(expr: ExprIdx, expression_count: usize) -> Result<(), WorkflowError> {
    if expr.as_usize() < expression_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds {
            expr,
        }))
    }
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

fn validate_expressions(
    expressions: &[ExprProgram],
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for expression in expressions {
        ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
        validate_expression_accessors(expression, accessor_count)?;
    }
    Ok(())
}

fn validate_expression_accessors(
    expression: &ExprProgram,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for op in expression.ops.as_ref() {
        if let ExprOp::LoadAccessor(accessor) = op {
            validate_accessor(*accessor, accessor_count)?;
        }
    }
    Ok(())
}

fn validate_accessors(accessors: &[AccessorProgram], slot_count: u16) -> Result<(), WorkflowError> {
    for accessor in accessors {
        validate_slot(accessor.root, slot_count)?;
    }
    Ok(())
}

fn validate_accessor(accessor: AccessorIdx, accessor_count: usize) -> Result<(), WorkflowError> {
    if accessor.as_usize() < accessor_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(
            CoreError::InvalidCompiledWorkflow {
                reason: "accessor index out of bounds",
            },
        ))
    }
}

/// Validates that accessor path index segments do not use the reserved u32::MAX value.
fn validate_accessor_path_symbols(accessors: &[AccessorProgram]) -> Result<(), WorkflowError> {
    for accessor in accessors {
        for segment in accessor.path.as_ref() {
            if let PathSegment::Index(index) = *segment
                && index == u32::MAX
            {
                return Err(WorkflowError::Expression(
                    CoreError::InvalidCompiledWorkflow {
                        reason: "accessor path index uses reserved value u32::MAX",
                    },
                ));
            }
        }
    }
    Ok(())
}

/// Check A: every node must be reachable from the entry step via a forward walk
/// following `next` edges and kind-specific targets.
fn validate_reachability(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    if node_count == 0 {
        return Ok(());
    }

    let mut visited: Vec<bool> = vec![false; node_count];
    let mut queue: Vec<usize> = Vec::new();

    let entry_usize = parts.entry.as_usize();
    if entry_usize >= node_count {
        return Ok(());
    }
    visited[entry_usize] = true;
    queue.push(entry_usize);

    let mut head = 0usize;
    while head < queue.len() {
        let current = queue[head];
        head = head.saturating_add(1);

        let mut targets: Vec<StepIdx> = Vec::new();
        let node = &parts.nodes[current];
        if let Some(next) = node.next {
            targets.push(next);
        }
        collect_node_targets(&node.kind, &mut targets);

        for target in targets {
            let target_usize = target.as_usize();
            if target_usize < node_count && !visited[target_usize] {
                visited[target_usize] = true;
                queue.push(target_usize);
            }
        }
    }

    for (index, was_visited) in visited.iter().enumerate() {
        if !was_visited {
            return Err(WorkflowError::UnreachableNode {
                step: StepIdx::new(u16::try_from(index).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: "max_steps",
                    }
                })?),
            });
        }
    }
    Ok(())
}

/// Collects all StepIdx targets referenced by a node kind (branch targets,
/// loop body/done, jump target, etc.) but NOT the `next` field.
#[allow(clippy::match_same_arms)] // Arms grouped by semantic category for readability
fn collect_node_targets(kind: &CompiledNodeKind, targets: &mut Vec<StepIdx>) {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => {}
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                targets.push(branch.target);
            }
            if let Some(fallback) = *otherwise {
                targets.push(fallback);
            }
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                targets.push(branch.target);
            }
            if let Some(fallback) = *otherwise {
                targets.push(fallback);
            }
        }
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. }
        | CompiledNodeKind::RetryCheck {
            body,
            exhausted: done,
            ..
        } => {
            targets.push(*body);
            targets.push(*done);
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            targets.push(*done);
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches.as_ref() {
                targets.push(*branch);
            }
            targets.push(*join);
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            targets.push(*entry);
            targets.push(*join);
        }
        CompiledNodeKind::TogetherJoin { .. } => {}
        CompiledNodeKind::WaitEvent { .. } => {}
        CompiledNodeKind::ErrorHandler { body, handler } => {
            targets.push(*body);
            targets.push(*handler);
        }
        CompiledNodeKind::Jump { target } => {
            targets.push(*target);
        }
    }
}

/// Check B: all edges must point forward except recognized loop back-edges.
/// Check D: loop spans must be properly nested (no overlapping loops).
fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::new();

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }
        })?);

        if let Some(next) = node.next {
            validate_forward_target(next, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
#[allow(clippy::match_same_arms)] // Arms grouped by semantic category for readability
fn validate_kind_edges(
    kind: &CompiledNodeKind,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => Ok(()),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                validate_forward_target(branch.target, ci, cid)?;
            }
            if let Some(fallback) = *otherwise {
                validate_forward_target(fallback, ci, cid)?;
            }
            Ok(())
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                validate_forward_target(branch.target, ci, cid)?;
            }
            if let Some(fallback) = *otherwise {
                validate_forward_target(fallback, ci, cid)?;
            }
            Ok(())
        }
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            let _ = branches;
            validate_forward_target(*join, ci, cid)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            let _ = entry;
            validate_forward_target(*join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => {
            let _ = body;
            validate_forward_target(*exhausted, ci, cid)
        }
        CompiledNodeKind::ErrorHandler { body, handler } => {
            let _ = body;
            validate_forward_target(*handler, ci, cid)
        }
        CompiledNodeKind::Jump { .. } => Ok(()),
    }
}

/// Validates a target step is strictly forward from the current node.
fn validate_forward_target(target: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    if target.as_usize() > ci {
        Ok(())
    } else {
        Err(WorkflowError::BackwardEdge {
            from: cid,
            to: target,
        })
    }
}

/// Tracks loop spans for nesting validation (Check D).
fn push_loop_span(
    kind: &CompiledNodeKind,
    ci: usize,
    spans: &mut Vec<(usize, usize)>,
) -> Result<(), WorkflowError> {
    let done_usize: Option<usize> = match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(done.as_usize()),
        CompiledNodeKind::TogetherStart { join, .. } => Some(join.as_usize()),
        _ => None,
    };

    let Some(done_idx) = done_usize else {
        return Ok(());
    };

    if let Some(&(_outer_start, outer_done)) = spans.last()
        && done_idx > outer_done
    {
        return Err(WorkflowError::ImproperLoopNesting {
            inner: StepIdx::new(u16::try_from(ci).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
            outer_done: StepIdx::new(u16::try_from(outer_done).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
        });
    }

    while spans
        .last()
        .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
    {
        spans.pop();
    }

    spans.push((ci, done_idx));
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
    use super::{
        check_expr_stack_bound, AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow,
        ExprBranch, ExprOp, ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError,
        WorkflowParts,
    };
    use crate::errors::CoreError;
    use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};
    use crate::value::ConstValue;

    #[test]

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
            Err(CoreError::ResourceLimitExceeded {
                resource: "expression ops",
            }) => Ok(()),
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

    #[test]
    fn workflow_parts_accept_resource_contract_at_exact_usage_bounds() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let contract = resource_contract(1, 0, 1, 1, 1);
        let parts = finish_const_parts_with(contract, vec![expression].into_boxed_slice());

        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;

        if workflow.resource_contract() == contract {
            Ok(())
        } else {
            Err(format!(
                "unexpected resource contract: {:?}",
                workflow.resource_contract()
            ))
        }
    }

    #[test]
    fn workflow_parts_reject_nodes_exceeding_resource_contract() -> Result<(), String> {
        expect_resource_error(resource_contract(0, 0, 1, 0, 0), "max_steps")
    }

    #[test]
    fn workflow_parts_reject_constants_exceeding_resource_contract() -> Result<(), String> {
        expect_resource_error(resource_contract(1, 0, 0, 0, 0), "max_constants")
    }

    #[test]
    fn workflow_parts_reject_slot_count_exceeding_resource_contract() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_slots",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_expressions_exceeding_resource_contract() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let parts = finish_const_parts_with(
            resource_contract(1, 0, 1, 0, 1),
            vec![expression].into_boxed_slice(),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_expressions",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_expression_stack_exceeding_resource_contract() -> Result<(), String> {
        let expression =
            ExprProgram::try_from_ops(vec![load(0), load(0), ExprOp::Eq].into_boxed_slice())
                .map_err(|error| error.to_string())?;
        let parts = finish_const_parts_with(
            resource_contract(1, 0, 1, 1, 1),
            vec![expression].into_boxed_slice(),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_expr_stack",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_hard_limit_exceeding_contract() -> Result<(), String> {
        let contract = resource_contract(1, 0, 1, 0, 0);
        let parts = finish_const_parts_with(
            ResourceContract {
                max_expressions: u16::MAX,
                ..contract
            },
            Box::new([]),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_expressions",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_accessors_hard_limit_exceeding_contract() -> Result<(), String> {
        let contract = resource_contract(1, 0, 1, 0, 0);
        let parts = finish_const_parts_with(
            ResourceContract {
                max_accessors: u16::MAX,
                ..contract
            },
            Box::new([]),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_accessors",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_expression_stack_hard_limit_exceeding_contract() -> Result<(), String>
    {
        let contract = resource_contract(1, 0, 1, 0, 0);
        let parts = finish_const_parts_with(
            ResourceContract {
                max_expr_stack: u8::MAX,
                ..contract
            },
            Box::new([]),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_expr_stack",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_node_id_mismatch() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice();

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::NodeIdMismatch { expected, actual })
                if expected == StepIdx::new(0) && actual == StepIdx::new(1) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_accept_choose_slot_branch_table_with_otherwise() -> Result<(), String> {
        let parts = choose_slot_parts(
            vec![SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
        );

        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn workflow_parts_reject_choose_slot_condition_out_of_bounds() -> Result<(), String> {
        let parts = choose_slot_parts(
            vec![SlotBranch {
                condition: SlotIdx::new(1),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_choose_slot_branch_target_out_of_bounds() -> Result<(), String> {
        let parts = choose_slot_parts(
            vec![SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(3),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
        );

        expect_step_out_of_bounds(parts, StepIdx::new(3))
    }

    #[test]
    fn workflow_parts_reject_choose_slot_otherwise_out_of_bounds() -> Result<(), String> {
        let parts = choose_slot_parts(
            vec![SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(3)),
        );

        expect_step_out_of_bounds(parts, StepIdx::new(3))
    }

    #[test]
    fn workflow_parts_reject_empty_branch_table_without_otherwise() -> Result<(), String> {
        let parts = choose_slot_parts(Box::new([]), None);

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::EmptyBranchTable) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_accept_choose_expr_branch_table_with_otherwise() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let parts = choose_expr_parts(
            vec![ExprBranch {
                condition: ExprIdx::new(0),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
            vec![expression].into_boxed_slice(),
        );

        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn workflow_parts_reject_choose_expr_condition_out_of_bounds() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let parts = choose_expr_parts(
            vec![ExprBranch {
                condition: ExprIdx::new(1),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
            vec![expression].into_boxed_slice(),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::Expression(CoreError::ExprOutOfBounds { expr }))
                if expr == ExprIdx::new(1) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_choose_expr_branch_target_out_of_bounds() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let parts = choose_expr_parts(
            vec![ExprBranch {
                condition: ExprIdx::new(0),
                target: StepIdx::new(3),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
            vec![expression].into_boxed_slice(),
        );

        expect_step_out_of_bounds(parts, StepIdx::new(3))
    }

    #[test]
    fn workflow_parts_reject_choose_expr_otherwise_out_of_bounds() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let parts = choose_expr_parts(
            vec![ExprBranch {
                condition: ExprIdx::new(0),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(3)),
            vec![expression].into_boxed_slice(),
        );

        expect_step_out_of_bounds(parts, StepIdx::new(3))
    }

    #[test]
    fn workflow_parts_accept_build_list_at_exact_item_limit() -> Result<(), String> {
        let items = vec![SlotIdx::new(0); MAX_LIST_ITEMS_PER_VALUE].into_boxed_slice();
        let parts = construction_parts(CompiledNodeKind::BuildList { items }, 1, 1);

        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn workflow_parts_reject_build_list_over_item_limit() -> Result<(), String> {
        let items =
            vec![SlotIdx::new(0); MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)].into_boxed_slice();
        let parts = construction_parts(CompiledNodeKind::BuildList { items }, 1, 1);

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "list_items",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_build_list_item_slot_out_of_bounds() -> Result<(), String> {
        let parts = construction_parts(
            CompiledNodeKind::BuildList {
                items: vec![SlotIdx::new(0), SlotIdx::new(2)].into_boxed_slice(),
            },
            2,
            2,
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(2) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_accept_build_object_at_exact_field_limit() -> Result<(), String> {
        let fields =
            vec![(crate::ids::SymbolId::new(0), SlotIdx::new(0)); MAX_OBJECT_FIELDS_PER_VALUE]
                .into_boxed_slice();
        let parts = construction_parts(CompiledNodeKind::BuildObject { fields }, 1, 1);

        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn workflow_parts_reject_build_object_over_field_limit() -> Result<(), String> {
        let fields = vec![
            (crate::ids::SymbolId::new(0), SlotIdx::new(0));
            MAX_OBJECT_FIELDS_PER_VALUE.saturating_add(1)
        ]
        .into_boxed_slice();
        let parts = construction_parts(CompiledNodeKind::BuildObject { fields }, 1, 1);

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "object_fields",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_reject_build_object_field_slot_out_of_bounds() -> Result<(), String> {
        let parts = construction_parts(
            CompiledNodeKind::BuildObject {
                fields: vec![
                    (crate::ids::SymbolId::new(1), SlotIdx::new(0)),
                    (crate::ids::SymbolId::new(2), SlotIdx::new(3)),
                ]
                .into_boxed_slice(),
            },
            2,
            2,
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(3) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_parts_preserve_build_object_duplicate_field_order() -> Result<(), String> {
        let key = crate::ids::SymbolId::new(5);
        let fields = vec![(key, SlotIdx::new(0)), (key, SlotIdx::new(1))].into_boxed_slice();
        let parts = construction_parts(CompiledNodeKind::BuildObject { fields }, 2, 2);

        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
        let copied = workflow.to_parts();
        let node = copied
            .nodes
            .first()
            .ok_or(String::from("missing construction node"))?;

        match &node.kind {
            CompiledNodeKind::BuildObject { fields } => {
                if fields.as_ref() == [(key, SlotIdx::new(0)), (key, SlotIdx::new(1))] {
                    Ok(())
                } else {
                    Err(format!("unexpected fields: {fields:?}"))
                }
            }
            other => Err(format!("unexpected node kind: {other:?}")),
        }
    }

    fn load(index: u16) -> ExprOp {
        ExprOp::LoadConst(ConstIdx::new(index))
    }

    fn construction_parts(
        kind: CompiledNodeKind,
        slot_count: u16,
        max_slots: u16,
    ) -> WorkflowParts {
        WorkflowParts {
            name: Box::<str>::from("construction_validation"),
            digest: WorkflowDigest::from_bytes([0x42; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, max_slots, 0, 0, 0),
        }
    }

    fn expect_resource_error(
        contract: ResourceContract,
        resource: &'static str,
    ) -> Result<(), String> {
        let parts = finish_const_parts_with(contract, Box::new([]));

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded { resource: found })
                if found == resource =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    fn expect_step_out_of_bounds(parts: WorkflowParts, step: StepIdx) -> Result<(), String> {
        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::StepOutOfBounds { step: found }) if found == step => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    fn finish_const_parts_with(
        resource_contract: ResourceContract,
        expressions: Box<[ExprProgram]>,
    ) -> WorkflowParts {
        WorkflowParts {
            name: Box::<str>::from("resource_case"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions,
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract,
        }
    }

    fn choose_slot_parts(branches: Box<[SlotBranch]>, otherwise: Option<StepIdx>) -> WorkflowParts {
        branch_parts(
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            },
            Box::new([]),
            1,
        )
    }

    fn choose_expr_parts(
        branches: Box<[ExprBranch]>,
        otherwise: Option<StepIdx>,
        expressions: Box<[ExprProgram]>,
    ) -> WorkflowParts {
        branch_parts(
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            },
            expressions,
            0,
        )
    }

    fn branch_parts(
        branch_kind: CompiledNodeKind,
        expressions: Box<[ExprProgram]>,
        slot_count: u16,
    ) -> WorkflowParts {
        let validated_slot_count = slot_count.max(1);
        WorkflowParts {
            name: Box::<str>::from("branch_case"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: branch_kind,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(2)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
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
            expressions,
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: validated_slot_count,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(3, validated_slot_count, 1, 1, 1),
        }
    }

    const fn resource_contract(
        max_steps: u16,
        max_slots: u16,
        max_constants: u16,
        max_expressions: u16,
        max_expr_stack: u8,
    ) -> ResourceContract {
        ResourceContract {
            max_steps,
            max_slots,
            max_constants,
            max_accessors: 0,
            max_expressions,
            max_expr_stack,
            max_step_budget_per_tick: 1,
            max_input_bytes: 1,
            max_output_bytes: 1,
            max_blob_bytes: 1,
            max_ipc_payload_bytes: 1,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 1,
            max_journal_batch_bytes: 1,
        }
    }

    // -- WorkflowError exact variant assertions --

    #[test]
    fn workflow_error_empty_nodes_exact_variant() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("empty"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::new([]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, 0, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::EmptyNodes) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_entry_out_of_bounds_exact_variant() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("entry_oob"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 0,
            entry: StepIdx::new(5),
            resource_contract: resource_contract(1, 0, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_step_out_of_bounds_exact_variant() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("step_oob"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(99)),
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, 0, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_slot_out_of_bounds_exact_variant() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(5)),
            next: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_const_out_of_bounds_exact_variant() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(50),
            },
        }]
        .into_boxed_slice();

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(50) => {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_node_id_mismatch_exact_variant() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(7),
            output: None,
            next: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice();

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::NodeIdMismatch { expected, actual })
                if expected == StepIdx::new(0) && actual == StepIdx::new(7) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_expression_wrapped_core_error_exact_variant() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;

        let parts = choose_expr_parts(
            vec![ExprBranch {
                condition: ExprIdx::new(1),
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            Some(StepIdx::new(1)),
            vec![expression].into_boxed_slice(),
        );

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::Expression(CoreError::ExprOutOfBounds { expr }))
                if expr == ExprIdx::new(1) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn workflow_error_empty_branch_table_exact_variant() -> Result<(), String> {
        let parts = choose_slot_parts(Box::new([]), None);

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::EmptyBranchTable) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // =========================================================================
    // Adversarial BDD tests -- workflow validation attack vectors
    // =========================================================================

    // --- Empty nodes attack vector ---

    #[test]
    fn workflow_empty_nodes_rejected_with_empty_nodes_error() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("empty"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::new([]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(0, 0, 0, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::EmptyNodes) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Entry step beyond node array ---

    #[test]
    fn workflow_entry_step_at_node_count_rejected_with_entry_out_of_bounds() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("entry_at_boundary"),
            digest: WorkflowDigest::from_bytes([1; 32]),
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
            entry: StepIdx::new(1), // exactly at len
            resource_contract: resource_contract(1, 0, 0, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- SetConst with out-of-bounds constant pool index ---

    #[test]
    fn workflow_set_const_out_of_bounds_constant_returns_const_out_of_bounds() -> Result<(), String>
    {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(99),
            },
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(99) => {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Copy with out-of-bounds source slot ---

    #[test]
    fn workflow_copy_out_of_bounds_source_slot_returns_slot_out_of_bounds() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(5),
            },
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- EvalExpr with out-of-bounds expression index ---

    #[test]
    fn workflow_eval_expr_out_of_bounds_returns_expression_wrapped_error() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(99),
            },
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::Expression(CoreError::ExprOutOfBounds { expr }))
                if expr == ExprIdx::new(99) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Node with output slot beyond slot_count ---

    #[test]
    fn workflow_node_output_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(5)),
            next: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Jump target out of bounds ---

    #[test]
    fn workflow_jump_target_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(50),
            },
        }]
        .into_boxed_slice();

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(50) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Finish with result slot out of bounds ---

    #[test]
    fn workflow_finish_result_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String>
    {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(10),
            },
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(10) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Nop with next step out of bounds ---

    #[test]
    fn workflow_nop_next_step_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(200)),
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice();

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(200) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Resource contract max_steps set to 0 with 1 node ---

    #[test]
    fn workflow_zero_max_steps_with_one_node_returns_resource_contract_exceeded(
    ) -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("zero_max_steps"),
            digest: WorkflowDigest::from_bytes([2; 32]),
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
            resource_contract: resource_contract(0, 0, 0, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- BuildList with slot out of bounds ---

    #[test]
    fn workflow_build_list_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            kind: CompiledNodeKind::BuildList {
                items: vec![SlotIdx::new(0), SlotIdx::new(5)].into_boxed_slice(),
            },
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- TogetherStart with out-of-bounds branch target ---

    #[test]
    fn workflow_together_start_branch_out_of_bounds_returns_step_out_of_bounds(
    ) -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(3, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1), StepIdx::new(99)].into_boxed_slice(),
                    join: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice();
        parts.resource_contract.max_steps = 3;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- TogetherStart with out-of-bounds join target ---

    #[test]
    fn workflow_together_start_join_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String>
    {
        let mut parts = finish_const_parts_with(resource_contract(2, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1)].into_boxed_slice(),
                    join: StepIdx::new(50),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice();
        parts.resource_contract.max_steps = 2;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(50) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- TogetherJoin with branch_count=0 is rejected ---

    #[test]
    fn workflow_together_join_zero_branch_count_returns_resource_exceeded() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
        parts.nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 0,
                accumulator: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice();
        parts.slot_count = 1;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "branch_count",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- RepeatStart with max_attempts=0 is rejected ---

    #[test]
    fn workflow_repeat_start_zero_max_attempts_returns_resource_exceeded() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(3, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts: 0,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice();
        parts.resource_contract.max_steps = 3;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_retry_attempts",
            }) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- Node ID mismatch at different positions ---

    #[test]
    fn workflow_second_node_id_mismatch_returns_node_id_mismatch() -> Result<(), String> {
        let mut parts = finish_const_parts_with(resource_contract(2, 0, 1, 0, 0), Box::new([]));
        parts.nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(5), // should be 1
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice();
        parts.resource_contract.max_steps = 2;

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::NodeIdMismatch { expected, actual })
                if expected == StepIdx::new(1) && actual == StepIdx::new(5) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    // --- CompiledWorkflow accessor and constant lookup return None for invalid indices ---

    #[test]
    fn compiled_workflow_constant_returns_none_for_out_of_bounds() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
            resource_contract(1, 0, 1, 0, 0),
            Box::new([]),
        ))
        .map_err(|error| error.to_string())?;

        if workflow.constant(ConstIdx::new(1)).is_some() {
            return Err(String::from("expected None for out-of-bounds constant"));
        }
        Ok(())
    }

    #[test]
    fn compiled_workflow_expression_returns_none_for_out_of_bounds() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
            resource_contract(1, 0, 1, 0, 0),
            Box::new([]),
        ))
        .map_err(|error| error.to_string())?;

        if workflow.expression(ExprIdx::new(0)).is_some() {
            return Err(String::from("expected None for out-of-bounds expression"));
        }
        Ok(())
    }

    #[test]
    fn compiled_workflow_accessor_returns_none_for_out_of_bounds() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
            resource_contract(1, 0, 1, 0, 0),
            Box::new([]),
        ))
        .map_err(|error| error.to_string())?;

        if workflow.accessor(AccessorIdx::new(0)).is_some() {
            return Err(String::from("expected None for out-of-bounds accessor"));
        }
        Ok(())
    }

    #[test]
    fn compiled_workflow_node_returns_none_for_out_of_bounds() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
            resource_contract(1, 0, 1, 0, 0),
            Box::new([]),
        ))
        .map_err(|error| error.to_string())?;

        if workflow.node(StepIdx::new(5)).is_some() {
            return Err(String::from("expected None for out-of-bounds node"));
        }
        Ok(())
    }

    // --- to_parts roundtrip preserves identity ---

    #[test]
    fn compiled_workflow_to_parts_roundtrip_preserves_fields() -> Result<(), String> {
        let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
            .map_err(|error| error.to_string())?;
        let original = finish_const_parts_with(
            resource_contract(1, 0, 1, 1, 1),
            vec![expression].into_boxed_slice(),
        );
        let workflow =
            CompiledWorkflow::try_from_parts(original).map_err(|error| error.to_string())?;

        let recovered = workflow.to_parts();
        if recovered.name.as_ref() != workflow.name() {
            return Err(String::from("name mismatch"));
        }
        if recovered.digest != workflow.digest() {
            return Err(String::from("digest mismatch"));
        }
        if recovered.entry != workflow.entry() {
            return Err(String::from("entry mismatch"));
        }
        if recovered.slot_count != workflow.slot_count() {
            return Err(String::from("slot_count mismatch"));
        }
        Ok(())
    }

    // --- Phase 46: IR structural validation tests ---

    fn phase46_parts_with_nodes(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
        let max_steps = u16::try_from(nodes.len()).unwrap_or(u16::MAX);
        WorkflowParts {
            name: Box::<str>::from("phase46"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(max_steps, slot_count, 1, 0, 0),
        }
    }

    #[test]
    fn phase46_rejects_unreachable_node() -> Result<(), String> {
        let parts = phase46_parts_with_nodes(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ],
            1,
        );
        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(2) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_accepts_reachable_chain() -> Result<(), String> {
        let parts = phase46_parts_with_nodes(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Nop,
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
            ],
            1,
        );
        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn phase46_rejects_backward_next() -> Result<(), String> {
        let parts = phase46_parts_with_nodes(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(0)),
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ],
            1,
        );
        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::BackwardEdge { from, to }) => {
                if from == StepIdx::new(1) && to == StepIdx::new(0) {
                    Ok(())
                } else {
                    Err(format!("wrong from/to: {from:?} -> {to:?}"))
                }
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_accepts_jump_backward() -> Result<(), String> {
        let parts = phase46_parts_with_nodes(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(0),
                    },
                },
            ],
            1,
        );
        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn phase46_accepts_foreach_forward() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("phase46_foreach"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
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
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(2),
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
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
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(4, 3, 1, 0, 0),
        };
        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn phase46_rejects_improper_nesting() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("phase46_nesting"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(4),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(1),
                        item_slot: SlotIdx::new(2),
                        limit: 10,
                        body: StepIdx::new(2),
                        done: StepIdx::new(5),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(3),
                        body: StepIdx::new(2),
                        done: StepIdx::new(5),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(4),
                        body: StepIdx::new(1),
                        done: StepIdx::new(5),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(5),
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
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 5,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(6, 5, 1, 0, 0),
        };
        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::ImproperLoopNesting { inner, outer_done }) => {
                if inner == StepIdx::new(1) && outer_done == StepIdx::new(4) {
                    Ok(())
                } else {
                    Err(format!(
                        "wrong inner/outer: inner={inner:?}, outer_done={outer_done:?}"
                    ))
                }
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_accepts_proper_nesting() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("phase46_proper"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(5),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(1),
                        item_slot: SlotIdx::new(2),
                        limit: 10,
                        body: StepIdx::new(2),
                        done: StepIdx::new(4),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: Some(StepIdx::new(3)),
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(3),
                        body: StepIdx::new(2),
                        done: StepIdx::new(4),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(4),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(4),
                        body: StepIdx::new(1),
                        done: StepIdx::new(5),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(5),
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
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 5,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(6, 5, 1, 0, 0),
        };
        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn phase46_accepts_accessor_field() -> Result<(), String> {
        let accessor = AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(SymbolId::new(42))].into_boxed_slice(),
        };
        let mut contract = resource_contract(1, 1, 1, 0, 0);
        contract.max_accessors = 1;
        let parts = WorkflowParts {
            name: Box::<str>::from("phase46_acc_field"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
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
            accessors: vec![accessor].into_boxed_slice(),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: contract,
        };
        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn phase46_accepts_accessor_index() -> Result<(), String> {
        let accessor = AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(7)].into_boxed_slice(),
        };
        let mut contract = resource_contract(1, 1, 1, 0, 0);
        contract.max_accessors = 1;
        let parts = WorkflowParts {
            name: Box::<str>::from("phase46_acc_index"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
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
            accessors: vec![accessor].into_boxed_slice(),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: contract,
        };
        CompiledWorkflow::try_from_parts(parts)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    // =========================================================================
    // Phase 46 adversarial tests -- IR structural validation edge cases
    // =========================================================================

    #[test]
    fn phase46_rejects_cycle_via_backward_next_edge() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("cycle_next"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
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
                    output: Some(SlotIdx::new(0)),
                    // Backward edge: node 1 -> node 0 creates a cycle.
                    next: Some(StepIdx::new(0)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(2, 1, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::BackwardEdge { from, to })
                if from == StepIdx::new(1) && to == StepIdx::new(0) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_rejects_duplicate_step_idx_via_node_id_mismatch() -> Result<(), String> {
        // Two nodes with the same StepIdx (both claim to be step 0).
        let parts = WorkflowParts {
            name: Box::<str>::from("dup_step_idx"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                // Second node at index 1 but claims to be step 0 (duplicate id).
                CompiledNode {
                    id: StepIdx::new(0),
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
            resource_contract: resource_contract(2, 1, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::NodeIdMismatch { expected, actual })
                if expected == StepIdx::new(1) && actual == StepIdx::new(0) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_rejects_slot_idx_out_of_bounds_in_finish() -> Result<(), String> {
        // Finish node references slot 99 but slot_count is only 1.
        let parts = WorkflowParts {
            name: Box::<str>::from("slot_oob_finish"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(99),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, 1, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_rejects_slot_idx_out_of_bounds_in_build_list() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("slot_oob_list"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0), SlotIdx::new(50)].into_boxed_slice(),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, 2, 0, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(50) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_rejects_slot_idx_out_of_bounds_in_output() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("slot_oob_output"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                // Output slot 200 is out of bounds for slot_count=1.
                output: Some(SlotIdx::new(200)),
                next: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(1)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, 1, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(200) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }

    #[test]
    fn phase46_rejects_unreachable_node_from_entry() -> Result<(), String> {
        // Node 0 finishes immediately; node 1 is never reached.
        let parts = WorkflowParts {
            name: Box::<str>::from("unreachable"),
            digest: WorkflowDigest::from_bytes([0x46; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
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
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(2, 1, 1, 0, 0),
        };

        match CompiledWorkflow::try_from_parts(parts) {
            Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(1) => Ok(()),
            other => Err(format!("unexpected result: {other:?}")),
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ResourceContract,
        WorkflowError, WorkflowParts,
    };
    use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn resource_contract_max_steps_is_positive(_unused in 0u8..1u8) {
            let contract = ResourceContract::DEFAULT;
            prop_assert!(contract.max_steps > 0);
        }
    }

    proptest! {
        #[test]
        fn resource_contract_max_slots_is_positive(_unused in 0u8..1u8) {
            let contract = ResourceContract::DEFAULT;
            prop_assert!(contract.max_slots > 0);
        }
    }

    // =========================================================================
    // Property A: Valid minimal workflow always passes validation
    //
    // Generate random (but structurally valid) workflows with 2-10 steps,
    // each forming a SetConst -> ... -> Finish chain.
    // =========================================================================

    /// Builds a valid linear workflow with `step_count` nodes.
    /// Nodes 0..N-2 are SetConst, node N-1 is Finish.
    /// slot_count = 1 (slot 0 is used throughout).
    fn build_valid_chain(step_count: usize) -> WorkflowParts {
        let mut nodes = Vec::with_capacity(step_count);
        let last = step_count.saturating_sub(1);
        for i in 0..last {
            let next_step = u16::try_from(i.saturating_add(1)).map_or(u16::MAX, |v| v);
            nodes.push(CompiledNode {
                id: StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v)),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(next_step)),
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(u16::try_from(last).map_or(u16::MAX, |v| v)),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
        WorkflowParts {
            name: Box::<str>::from("proptest_valid_chain"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
        }
    }

    proptest! {
        #[test]
        fn prop_a_valid_chain_workflow_passes_validation(step_count in 2u16..10u16) {
            let parts = build_valid_chain(usize::from(step_count));
            let result = CompiledWorkflow::try_from_parts(parts);
            prop_assert!(
                result.is_ok(),
                "valid chain with {} steps should pass validation, got {:?}",
                step_count,
                result
            );
        }
    }

    // =========================================================================
    // Property B: SlotIdx out of bounds always rejected
    //
    // Generate workflows where Finish or SetConst references a slot >= slot_count.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_b_finish_slot_out_of_bounds_rejected(
            slot_count in 1u16..10u16,
            bad_slot_delta in 1u16..50u16
        ) {
            let bad_slot = slot_count.saturating_add(bad_slot_delta);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_b_finish_oob"),
                digest: WorkflowDigest::from_bytes([0xBB; 32]),
                nodes: vec![CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(bad_slot),
                    },
                }]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(1, slot_count, 1, 0, 0),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::SlotOutOfBounds { slot }) => {
                    prop_assert_eq!(slot, SlotIdx::new(bad_slot));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected SlotOutOfBounds, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_b_setconst_output_slot_out_of_bounds_rejected(
            slot_count in 1u16..10u16,
            bad_slot_delta in 1u16..50u16
        ) {
            let bad_slot = slot_count.saturating_add(bad_slot_delta);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_b_output_oob"),
                digest: WorkflowDigest::from_bytes([0xBC; 32]),
                nodes: vec![
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: Some(SlotIdx::new(bad_slot)),
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
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(2, slot_count, 1, 0, 0),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::SlotOutOfBounds { slot }) => {
                    prop_assert_eq!(slot, SlotIdx::new(bad_slot));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected SlotOutOfBounds, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property C: Duplicate StepIdx always rejected
    //
    // Generate workflows with two nodes claiming the same StepIdx.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_c_duplicate_step_idx_rejected(
            step_count in 3u16..10u16,
            duplicate_id_pos in 1u16..9u16
        ) {
            let n = usize::from(step_count);
            let dup_pos = usize::from(duplicate_id_pos.min(step_count.saturating_sub(1)));
            let mut nodes = Vec::with_capacity(n);
            for i in 0..n {
                let next = if i < n.saturating_sub(1) {
                    Some(StepIdx::new(u16::try_from(i.saturating_add(1)).map_or(u16::MAX, |v| v)))
                } else {
                    None
                };
                let kind = if i == n.saturating_sub(1) {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                } else {
                    CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    }
                };
                // Node at dup_pos claims to be step 0 (duplicate).
                let claimed_id = if i == dup_pos {
                    StepIdx::new(0)
                } else {
                    StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v))
                };
                nodes.push(CompiledNode {
                    id: claimed_id,
                    output: Some(SlotIdx::new(0)),
                    next,
                    kind,
                });
            }
            let max_steps = u16::try_from(n).map_or(u16::MAX, |v| v);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_c_dup"),
                digest: WorkflowDigest::from_bytes([0xCC; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: 1,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::NodeIdMismatch { expected, actual }) => {
                    prop_assert_eq!(expected, StepIdx::new(u16::try_from(dup_pos).map_or(u16::MAX, |v| v)));
                    prop_assert_eq!(actual, StepIdx::new(0));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected NodeIdMismatch, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property D: Unreachable nodes always rejected
    //
    // Generate workflows where an extra node exists but no other node points to it.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_d_unreachable_node_rejected(
            chain_len in 2u16..8u16,
            unreachable_count in 1u16..3u16
        ) {
            let chain_n = usize::from(chain_len);
            let extra_n = usize::from(unreachable_count);
            let total = chain_n.saturating_add(extra_n);
            let mut nodes = Vec::with_capacity(total);

            // Build a valid chain of chain_len nodes.
            for i in 0..chain_n {
                let is_last = i == chain_n.saturating_sub(1);
                let next = if is_last {
                    None
                } else {
                    Some(StepIdx::new(u16::try_from(i.saturating_add(1)).map_or(u16::MAX, |v| v)))
                };
                let kind = if is_last {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                } else {
                    CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    }
                };
                nodes.push(CompiledNode {
                    id: StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v)),
                    output: Some(SlotIdx::new(0)),
                    next,
                    kind,
                });
            }

            // Add unreachable nodes at the end.
            for i in chain_n..total {
                nodes.push(CompiledNode {
                    id: StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v)),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                });
            }

            let max_steps = u16::try_from(total).map_or(u16::MAX, |v| v);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_d_unreachable"),
                digest: WorkflowDigest::from_bytes([0xDD; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: 1,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::UnreachableNode { step }) => {
                    // The first unreachable node should be at index chain_len.
                    prop_assert_eq!(step, StepIdx::new(u16::try_from(chain_n).map_or(u16::MAX, |v| v)));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected UnreachableNode, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property E: Resource contract bounds respected
    //
    // Workflows with step_count > max_steps fail with ResourceContractExceeded.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_e_resource_contract_max_steps_violated(
            actual_steps in 2u16..10u16,
            shortfall in 1u16..5u16
        ) {
            let max_steps_declared = actual_steps.saturating_sub(shortfall);
            // Build a valid chain but with a contract that doesn't cover it.
            let valid_parts = build_valid_chain(usize::from(actual_steps));
            let parts = WorkflowParts {
                resource_contract: resource_contract(max_steps_declared, 1, 1, 0, 0),
                ..valid_parts
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::ResourceContractExceeded { resource }) => {
                    prop_assert_eq!(resource, "max_steps");
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected ResourceContractExceeded for max_steps, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_e_resource_contract_max_slots_violated(
            actual_slots in 1u16..10u16,
            shortfall in 1u16..5u16
        ) {
            let declared_slots = actual_slots.saturating_sub(shortfall);
            // Single node that uses a slot at the boundary.
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_e_slots"),
                digest: WorkflowDigest::from_bytes([0xEE; 32]),
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
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: actual_slots,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(1, declared_slots, 1, 0, 0),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::ResourceContractExceeded { resource }) => {
                    prop_assert_eq!(resource, "max_slots");
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected ResourceContractExceeded for max_slots, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    fn resource_contract(
        max_steps: u16,
        max_slots: u16,
        max_constants: u16,
        max_expressions: u16,
        max_expr_stack: u8,
    ) -> ResourceContract {
        ResourceContract {
            max_steps,
            max_slots,
            max_constants,
            max_accessors: 0,
            max_expressions,
            max_expr_stack,
            max_step_budget_per_tick: 1,
            max_input_bytes: 1,
            max_output_bytes: 1,
            max_blob_bytes: 1,
            max_ipc_payload_bytes: 1,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 1,
            max_journal_batch_bytes: 1,
        }
    }
}

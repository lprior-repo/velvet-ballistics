#![forbid(unsafe_code)]
//! Compiled workflow IR.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{
    AccessorIdx, ActionId, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::{
    MAX_ACCESSORS, MAX_CONSTANTS, MAX_EXPRESSION_OPS, MAX_EXPRESSION_STACK, MAX_EXPRESSIONS,
    MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_PATH_DEPTH, MAX_SLOTS_PER_WORKFLOW,
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
    symbols_count: u32,
    entry: StepIdx,
    resource_contract: ResourceContract,
    step_names: Box<[Box<str>]>,
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
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
        })
    }

    /// Creates a compiled workflow without validation.
    ///
    /// Enabled by the `test-util` feature. For test use only.
    /// Production code must use [`Self::try_from_parts`].
    #[cfg(feature = "test-util")]
    pub fn from_parts_unchecked(parts: WorkflowParts) -> Self {
        Self {
            name: parts.name,
            digest: parts.digest,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
        }
    }

    /// Creates a compiled workflow for Kani harnesses that need concrete runtime
    /// APIs without re-proving workflow validation internals in each harness.
    #[cfg(kani)]
    pub fn kani_from_parts_unchecked(parts: WorkflowParts) -> Self {
        Self {
            name: parts.name,
            digest: parts.digest,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
        }
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

    /// Number of interned symbols referenced by this workflow.
    #[must_use]
    pub const fn symbols_count(&self) -> u32 {
        self.symbols_count
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

    /// Returns the human-readable step name for a given step index.
    #[must_use]
    pub fn step_name(&self, step: StepIdx) -> Option<&str> {
        self.step_names.get(step.as_usize()).map(|s| s.as_ref())
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
            symbols_count: self.symbols_count,
            entry: self.entry,
            resource_contract: self.resource_contract,
            step_names: self.step_names.clone(),
        }
    }
    pub(crate) fn error_handler_for_body(&self, body_step: StepIdx) -> Option<&CompiledNode> {
        self.nodes.iter().find(|node| {
            matches!(node.kind, CompiledNodeKind::ErrorHandler { body, .. } if body == body_step)
        })
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
    /// Maximum transitions per runtime tick.
    pub max_transitions_per_tick: u64,
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
    /// Whether secret-tainted results are allowed in answer payloads.
    pub allows_secret_results: bool,
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
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1_048_576,
        max_output_bytes: 262_144,
        max_blob_bytes: 16_777_216,
        max_ipc_payload_bytes: 1_048_576,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: 1_024,
        max_queue_depth: 1_024,
        max_journal_batch_bytes: 1_048_576,
        allows_secret_results: false,
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
    /// Number of interned symbols referenced by this workflow.
    pub symbols_count: u32,
    /// Entry step.
    pub entry: StepIdx,
    /// Explicit resource bounds carried with the compiled artifact.
    pub resource_contract: ResourceContract,
    /// Human-readable step names indexed by StepIdx.
    pub step_names: Box<[Box<str>]>,
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
#[non_exhaustive]
pub enum PathSegment {
    /// Object field by interned symbol.
    Field(SymbolId),
    /// List index.
    Index(u32),
}

/// Workflow IR validation failures.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
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
    /// Budget step count overflowed during computation.
    #[error("budget step count overflow: {actual} cannot be represented")]
    StepCountOverflow {
        /// The overflowing value.
        actual: u64,
    },
    /// Nesting depth overflowed `u16::MAX` during budget computation.
    #[error("nesting depth overflow: {depth} cannot be incremented past u16::MAX")]
    DepthOverflow {
        /// The actual pre-overflow depth value.
        depth: u16,
    },
    /// A symbol identifier exceeded the declared symbols table bound.
    #[error("symbol {symbol:?} exceeds symbols_count")]
    SymbolOutOfBounds {
        /// Invalid symbol identifier.
        symbol: SymbolId,
    },
    /// An accessor path exceeded the maximum allowed depth.
    #[error("accessor path depth {depth} exceeds maximum {max}")]
    AccessorPathTooDeep {
        /// Actual path depth.
        depth: usize,
        /// Maximum allowed path depth.
        max: usize,
    },
    /// A jump creates a cycle that would cause infinite execution.
    #[error(
        "jump cycle detected: {step:?} jumps to {target:?} which is already in the current traversal path"
    )]
    JumpCycle {
        /// Step issuing the jump.
        step: StepIdx,
        /// Jump target creating the cycle.
        target: StepIdx,
    },
    /// A `TogetherStart` node is reachable from the body of another
    /// `TogetherStart` node. Nested parallel sections are rejected because
    /// `compute_max_parallel_in_flight` reports a per-node maximum and
    /// therefore cannot represent the true peak concurrency of nested
    /// parallel branches, leading to spurious `ParallelLimitExceeded`
    /// failures at runtime. Surface this at compile time instead.
    #[error("nested TogetherStart: outer {outer:?} contains inner {inner:?}")]
    NestedTogether {
        /// The outer `TogetherStart` step that owns the branch body.
        outer: StepIdx,
        /// The inner `TogetherStart` step reachable from the outer branch.
        inner: StepIdx,
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
#[non_exhaustive]
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
    /// Optional error handler step. When this step fails and `on_error` is
    /// set, the engine routes PC to this handler instead of failing the run.
    pub on_error: Option<StepIdx>,
    /// Optional slot where failure information is written before routing to
    /// the error handler. The slot receives an object with `code`, `message`,
    /// and `step` fields describing the failure.
    pub error_slot: Option<SlotIdx>,
    /// Node behavior.
    pub kind: CompiledNodeKind,
}

/// Hot-path node variants. All references are numeric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[non_exhaustive]
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
    ErrorHandler {
        /// Body step to execute.
        body: StepIdx,
        /// Handler step to route to on body failure.
        handler: StepIdx,
        /// Optional slot to write failed step index for handler consumption.
        error_slot: Option<SlotIdx>,
    },
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
    validate_accessor_paths(&parts.accessors, parts.symbols_count)?;
    validate_constants_symbols(&parts.constants, parts.symbols_count)?;
    validate_build_object_symbols(&parts.nodes, parts.symbols_count)?;
    validate_reachability(parts)?;
    validate_forward_edges(parts)?;
    // RE-001: reject nested `TogetherStart`. Walked inline here (the
    // helper lives in `crate::engine::validate` to avoid an
    // engine->workflow dependency cycle) so `try_from_parts` rejects
    // the shape at the source. The helper returns
    // `WorkflowError::NestedTogether { outer, inner }`.
    crate::engine::validate::validate_no_nested_together(parts)?;
    Ok(())
}

fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    use crate::budget::{BoundednessPolicy, WholeWorkflowBudget};

    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;

    validate_budget_result(BoundednessPolicy::DEFAULT.validate(&budget))
}

fn validate_budget_result(
    result: Result<(), crate::budget::BudgetError>,
) -> Result<(), WorkflowError> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: budget_error_detail(&error),
        }),
    }
}

fn budget_error_detail(error: &crate::budget::BudgetError) -> &'static str {
    match error {
        crate::budget::BudgetError::TotalStepsExceeded { .. } => "max_total_steps",
        crate::budget::BudgetError::TotalSlotsExceeded { .. } => "max_total_slots",
        crate::budget::BudgetError::FanoutExceeded { .. } => "max_fanout",
        crate::budget::BudgetError::NestingDepthExceeded { .. } => "max_nesting_depth",
        crate::budget::BudgetError::ParallelExceeded { .. } => "max_parallel_in_flight",
        crate::budget::BudgetError::ActionTicketsExceeded { .. } => "max_action_tickets",
        crate::budget::BudgetError::RunTimeExceeded { .. } => "max_run_time_seconds",
        crate::budget::BudgetError::ResultBytesExceeded { .. } => "max_result_bytes",
        crate::budget::BudgetError::StepsExecutableExceeded { .. } => "max_steps_executable",
        crate::budget::BudgetError::TimerEntriesExceeded { .. } => "max_timer_entries",
        crate::budget::BudgetError::TraceEventsExceeded { .. } => "max_trace_events",
        crate::budget::BudgetError::JournalBatchBytesExceeded { .. } => "max_journal_batch_bytes",
        crate::budget::BudgetError::QueueDepthExceeded { .. } => "max_queue_depth",
        crate::budget::BudgetError::IpcPayloadBytesExceeded { .. } => "max_ipc_payload_bytes",
        crate::budget::BudgetError::BlobBytesExceeded { .. } => "max_blob_bytes",
        crate::budget::BudgetError::InputBytesExceeded { .. } => "max_input_bytes",
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
    validate_expr_stack_contract(parts.expressions.as_ref(), contract.max_expr_stack)?;
    validate_transitions_per_tick(contract.max_transitions_per_tick)
}

/// Validates that `max_transitions_per_tick` is within acceptable bounds.
/// Must be at least 1 (non-zero) and must not exceed the protocol hard limit.
fn validate_transitions_per_tick(max_transitions_per_tick: u64) -> Result<(), WorkflowError> {
    use crate::limits::MAX_STEP_BUDGET;
    if max_transitions_per_tick == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "max_transitions_per_tick",
        });
    }
    if max_transitions_per_tick > MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_transitions_per_tick",
        });
    }
    Ok(())
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

fn validate_node(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_node_common(node, parts)?;
    validate_node_kind(&node.kind, parts)
}

fn validate_node_common(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_optional_slot(node.output, parts.slot_count)?;
    validate_optional_step(node.next, parts.nodes.len())?;
    validate_optional_step(node.on_error, parts.nodes.len())?;
    validate_optional_slot(node.error_slot, parts.slot_count)
}

fn validate_node_kind(kind: &CompiledNodeKind, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    match kind {
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
        CompiledNodeKind::ErrorHandler {
            body,
            handler,
            error_slot,
        } => {
            validate_two_steps(*body, *handler, parts)?;
            validate_optional_slot(*error_slot, parts.slot_count)
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

/// Validates accessor paths: depth limits, reserved index values, and SymbolId bounds.
fn validate_accessor_paths(
    accessors: &[AccessorProgram],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for accessor in accessors {
        let path_len = accessor.path.len();
        if path_len > MAX_PATH_DEPTH {
            return Err(WorkflowError::AccessorPathTooDeep {
                depth: path_len,
                max: MAX_PATH_DEPTH,
            });
        }
        for segment in accessor.path.as_ref() {
            match *segment {
                PathSegment::Field(symbol) => {
                    validate_symbol(symbol, symbols_count)?;
                }
                PathSegment::Index(index) => {
                    if index == u32::MAX {
                        return Err(WorkflowError::Expression(
                            CoreError::InvalidCompiledWorkflow {
                                reason: "accessor path index uses reserved value u32::MAX",
                            },
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validates SymbolId values in the constant pool against the declared symbols count.
fn validate_constants_symbols(
    constants: &[ConstValue],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for constant in constants {
        if let ConstValue::Symbol(symbol) = *constant {
            validate_symbol(symbol, symbols_count)?;
        }
    }
    Ok(())
}

/// Validates SymbolId values in BuildObject fields across all nodes.
fn validate_build_object_symbols(
    nodes: &[CompiledNode],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for node in nodes {
        if let CompiledNodeKind::BuildObject { fields } = &node.kind {
            for (symbol, _slot) in fields.as_ref() {
                validate_symbol(*symbol, symbols_count)?;
            }
        }
    }
    Ok(())
}

/// Validates that a symbol identifier falls within the declared symbols table bound.
fn validate_symbol(symbol: SymbolId, symbols_count: u32) -> Result<(), WorkflowError> {
    if symbol.get() < symbols_count {
        Ok(())
    } else {
        Err(WorkflowError::SymbolOutOfBounds { symbol })
    }
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
    let Some(entry_flag) = visited.get_mut(entry_usize) else {
        return Err(WorkflowError::EntryOutOfBounds { entry: parts.entry });
    };
    *entry_flag = true;
    queue.push(entry_usize);

    let mut head = 0usize;
    while head < queue.len() {
        let current = match queue.get(head) {
            Some(&v) => v,
            None => break,
        };
        head = match head.checked_add(1) {
            Some(v) => v,
            None => break,
        };

        let mut targets: Vec<StepIdx> = Vec::new();
        let node = match parts.nodes.get(current) {
            Some(n) => n,
            None => break,
        };
        if let Some(next) = node.next {
            targets.push(next);
        }
        if let Some(handler) = node.on_error {
            targets.push(handler);
        }
        collect_node_targets(&node.kind, &mut targets);

        for target in targets {
            let target_usize = target.as_usize();
            if target_usize < node_count {
                let Some(flag) = visited.get_mut(target_usize) else {
                    continue;
                };
                if !*flag {
                    *flag = true;
                    queue.push(target_usize);
                }
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
        | CompiledNodeKind::Finish { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::WaitEvent { .. } => {}
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            collect_choose_slot_targets(branches, *otherwise, targets);
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            collect_choose_expr_targets(branches, *otherwise, targets);
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
            collect_together_start_targets(branches, *join, targets);
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            targets.push(*entry);
            targets.push(*join);
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            targets.push(*body);
            targets.push(*handler);
        }
        CompiledNodeKind::Jump { target } => {
            targets.push(*target);
        }
    }
}

fn collect_choose_slot_targets(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        targets.push(fallback);
    }
}

fn collect_choose_expr_targets(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        targets.push(fallback);
    }
}

fn collect_together_start_targets(branches: &[StepIdx], join: StepIdx, targets: &mut Vec<StepIdx>) {
    for branch in branches {
        targets.push(*branch);
    }
    targets.push(join);
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

        if let Some(handler) = node.on_error {
            validate_forward_target(handler, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
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
        | CompiledNodeKind::Finish { .. }
        | CompiledNodeKind::Jump { .. } => Ok(()),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_choose_slot_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_choose_expr_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            validate_loop_start_edges(*body, *done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { done, .. } => validate_loop_done_only(*done, ci, cid),
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together_start_edges(branches, *join, ci, cid)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            validate_together_branch_edges(*entry, *join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. } => {
            validate_loop_start_edges(*body, *done, ci, cid)
        }
        CompiledNodeKind::CollectPage { done, .. }
        | CompiledNodeKind::CollectNext { done, .. }
        | CompiledNodeKind::ReduceNext { done, .. }
        | CompiledNodeKind::RepeatAttempt { done, .. } => validate_loop_done_only(*done, ci, cid),
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck { exhausted, .. } => {
            validate_loop_done_only(*exhausted, ci, cid)
        }
        CompiledNodeKind::ErrorHandler { handler, .. } => {
            validate_loop_done_only(*handler, ci, cid)
        }
    }
}

fn validate_choose_slot_edges(
    branches: &[SlotBranch],
    otherwise: &Option<StepIdx>,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = *otherwise {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_choose_expr_edges(
    branches: &[ExprBranch],
    otherwise: &Option<StepIdx>,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = *otherwise {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_loop_done_only(done: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(done, ci, cid)
}

/// Validates that both `body` and `done` edges of a `*Start` loop node point
/// strictly forward from the current node. `body` here is the entry into the
/// loop body (a forward edge); only `*Next` carries the back-edge in its
/// `body` slot, so `*Start` must require `body` to be forward.
fn validate_loop_start_edges(
    body: StepIdx,
    done: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(body, ci, cid)?;
    validate_forward_target(done, ci, cid)
}

fn validate_together_start_edges(
    branches: &[StepIdx],
    join: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(*branch, ci, cid)?;
    }
    validate_forward_target(join, ci, cid)
}

fn validate_together_branch_edges(
    entry: StepIdx,
    join: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(entry, ci, cid)?;
    validate_forward_target(join, ci, cid)
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

    match spans.last().copied() {
        Some((_outer_start, outer_done)) if done_idx > outer_done => {
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
        _ => {}
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

// ============================================================================
// Lifecycle state machine
// ============================================================================

/// Lifecycle state of a run derived from journal event replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LifecycleState {
    /// Run accepted but not yet active.
    Pending,
    /// Run is actively executing.
    Active,
    /// Run is waiting for an external answer.
    WaitingAnswer,
    /// Run was cancelled.
    Cancelled,
    /// Run completed successfully.
    Completed,
    /// Run failed.
    Failed,
}

impl LifecycleState {
    /// Returns true if this is a terminal state.
    /// Note: Failed is NOT terminal because retry can transition from Failed.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Completed)
    }
}

/// Lifecycle command issued by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LifecycleCommand {
    /// Cancel the run.
    Cancel,
    /// Resume a waiting run.
    Resume,
    /// Retry a failed run.
    Retry,
    /// Answer a waiting run's question.
    Answer,
}

/// Checks if a lifecycle state transition is valid for the given command.
#[must_use]
pub const fn check_lifecycle_transition(state: LifecycleState, cmd: LifecycleCommand) -> bool {
    match (state, cmd) {
        // Cancel is valid from Active or WaitingAnswer
        (LifecycleState::Active, LifecycleCommand::Cancel) => true,
        (LifecycleState::WaitingAnswer, LifecycleCommand::Cancel) => true,
        // Resume is valid from WaitingAnswer
        (LifecycleState::WaitingAnswer, LifecycleCommand::Resume) => true,
        // Retry is valid from Failed
        (LifecycleState::Failed, LifecycleCommand::Retry) => true,
        // Answer is valid from WaitingAnswer
        (LifecycleState::WaitingAnswer, LifecycleCommand::Answer) => true,
        // All other transitions are invalid
        _ => false,
    }
}

/// Run state snapshot returned by replay.
#[derive(Debug, Clone)]
pub struct RunState {
    /// Current lifecycle state.
    pub lifecycle: LifecycleState,
    /// Run identifier.
    pub run_id: RunId,
}

impl RunState {
    /// Returns true if this run is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.lifecycle.is_terminal()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

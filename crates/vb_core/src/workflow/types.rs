//! Core workflow types: resource bounds, branches, and parts representation.

use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::ConstValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
        max_output_bytes: 262_144,
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
    pub nodes: Box<[super::CompiledNode]>,
    /// Expression bytecode table.
    pub expressions: Box<[super::ExprProgram]>,
    /// Accessor bytecode table.
    pub accessors: Box<[super::AccessorProgram]>,
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
    Expression(#[from] crate::errors::CoreError),
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
}

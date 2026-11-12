#![forbid(unsafe_code)]
//! Workflow IR validation failures.

use super::{ContractResource, InputConstraintIssue, InputSlotKind};
use crate::budget::BudgetDimension;
use crate::errors::CoreError;
use crate::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId};
use thiserror::Error;

/// Workflow IR validation failures.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowError {
    /// Bound symbol payloads failed metadata validation.
    #[error("invalid symbol metadata: {source}")]
    InvalidSymbolMetadata {
        /// Exact metadata validation failure.
        source: super::source_metadata::SourceMetadataError,
    },
    /// A step wrote to a repeat counter owned by a different step.
    #[error("step {writer:?} wrote to repeat counter slot {slot:?} owned by step {owner:?}")]
    RepeatCounterForeignWrite {
        /// Owner RepeatStart step.
        owner: StepIdx,
        /// Foreign writing step.
        writer: StepIdx,
        /// Reserved counter slot.
        slot: SlotIdx,
    },
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
        /// Resource dimension that exceeded its declared bound.
        resource: ContractResource,
    },
    /// Resource contract exceeds protocol hard limits.
    #[error("resource contract exceeds hard limit: {resource}")]
    ResourceContractTooLarge {
        /// Resource dimension that exceeds the protocol hard limit.
        resource: ContractResource,
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

    /// Reduce branch body and done targets are the same step.
    #[error("reduce node {step:?} aliases body and done at {target:?}")]
    ReduceBranchTargetAlias {
        /// The reduce node with the alias.
        step: StepIdx,
        /// The shared target step.
        target: StepIdx,
    },
    /// ForEachNext branch body and done targets are the same step.
    #[error("for_each_next node {step:?} aliases body and done at {target:?}")]
    ForEachBranchTargetAlias {
        /// The for_each_next node with the alias.
        step: StepIdx,
        /// The shared target step.
        target: StepIdx,
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
        /// Budget dimension that failed the policy bound.
        detail: BudgetDimension,
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
    /// A compiled input slot references a slot index that exceeds the declared slot count.
    #[error("input slot {slot:?} kind {kind:?} is outside slot_count")]
    InputSlotOutOfBounds {
        /// The out-of-bounds input slot.
        slot: SlotIdx,
        /// The kind classification of the offending slot.
        kind: InputSlotKind,
    },
    /// A compiled input slot index is duplicated.
    #[error("duplicate input slot {slot:?} kind {kind:?}")]
    DuplicateInputSlot {
        /// The duplicated slot index.
        slot: SlotIdx,
        /// The kind classification of the duplicate.
        kind: InputSlotKind,
    },
    /// Compiled input slots are not in strictly increasing index order.
    #[error("input slots out of canonical order: {prev_slot:?} followed by {curr_slot:?}")]
    NonCanonicalInputSlotOrder {
        /// The preceding slot index.
        prev_slot: SlotIdx,
        /// The out-of-order slot index.
        curr_slot: SlotIdx,
    },
    /// A retained input constraint is structurally inconsistent.
    #[error("input slot {slot:?} has inconsistent constraint: {issue:?}")]
    InconsistentInputConstraint {
        /// The slot carrying the inconsistent constraint.
        slot: SlotIdx,
        /// The structural inconsistency found.
        issue: InputConstraintIssue,
    },
    /// A loop control step has no matching loop body or done target.
    #[error("loop control step {control:?} is unpaired")]
    LoopRegionUnpaired {
        /// The unpaired loop control step.
        control: StepIdx,
    },
    /// Loop regions cross each other, violating proper nesting.
    #[error("loop region at {control:?} crosses from {from:?} to {to:?}")]
    LoopRegionCrossing {
        /// The loop control step with the crossing region.
        control: StepIdx,
        /// The start of the crossing region.
        from: StepIdx,
        /// The end of the crossing region.
        to: StepIdx,
    },
    /// A loop control step references an unsupported node kind.
    #[error("loop control step {control:?} references unsupported step {step:?}")]
    LoopRegionUnsupported {
        /// The loop control step.
        control: StepIdx,
        /// The unsupported referenced step.
        step: StepIdx,
    },
}

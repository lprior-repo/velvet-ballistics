//! Typed checked errors for the owned root bundle constructor.
//!
//! This module defines the complete error surface for constructing an
//! `OwnedRootBundle` from roots, a checkpoint, and a compiled workflow.
//! Errors are organized by validation phase and carry typed context
//! for deterministic diagnostics.

use thiserror::Error;

use crate::errors::CoreError;
use crate::ids::{BlobId, ListId, ObjectId, SlotIdx, SymbolId};
use crate::value::{SlotValue, Taint};
use crate::workflow::InputSlotKind;

/// Typed arena node identity for graph validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArenaNode {
    /// Interned symbol node.
    Symbol(SymbolId),
    /// Runtime list arena node.
    List(ListId),
    /// Runtime object arena node.
    Object(ObjectId),
    /// Runtime blob storage node.
    Blob(BlobId),
}

/// The 8 exact runtime value kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// Explicit null value.
    Null,
    /// Boolean scalar.
    Bool,
    /// Signed integer scalar.
    I64,
    /// Finite floating-point scalar.
    F64,
    /// Interned symbol handle.
    Symbol,
    /// Runtime list arena handle.
    List,
    /// Runtime object arena handle.
    Object,
    /// Runtime blob storage handle.
    Blob,
}

impl ValueKind {
    /// Derive the value kind from a runtime slot value.
    pub(super) fn of(value: SlotValue) -> Self {
        match value {
            SlotValue::Null => Self::Null,
            SlotValue::Bool(_) => Self::Bool,
            SlotValue::I64(_) => Self::I64,
            SlotValue::F64(_) => Self::F64,
            SlotValue::Symbol(_) => Self::Symbol,
            SlotValue::List(_) => Self::List,
            SlotValue::Object(_) => Self::Object,
            SlotValue::Blob(_) => Self::Blob,
        }
    }
}

/// Typed checked errors for the owned root bundle constructor.
#[derive(Debug, Error)]
pub enum OwnedRootBundleError {
    /// Checkpoint cap and workflow max slots disagree or are zero.
    #[error("cap mismatch: checkpoint {checkpoint_cap} vs workflow {workflow_max_slots}")]
    CapMismatch {
        /// Checkpoint's declared arena entry bound.
        checkpoint_cap: u64,
        /// Workflow's declared max slots.
        workflow_max_slots: u64,
    },

    /// Arena entry count arithmetic overflowed u64.
    #[error("arena entry count overflowed u64")]
    ArenaCountOverflow,

    /// Total arena entries exceed the declared cap.
    #[error("arena entries {count} exceed cap {limit}")]
    ArenaLimit {
        /// Computed total arena entry count.
        count: u64,
        /// Declared arena entry cap.
        limit: u64,
    },

    /// Blob payload exceeds the workflow blob byte cap.
    #[error("blob {blob:?} length {len} exceeds workflow cap {limit}")]
    BlobLimit {
        /// Blob identifier.
        blob: BlobId,
        /// Blob payload length in bytes.
        len: u64,
        /// Workflow's max_blob_bytes cap.
        limit: u64,
    },

    /// Checkpoint validation failed; wraps the underlying core error.
    #[error("checkpoint validation failed: {0}")]
    Checkpoint(CoreError),

    /// Root slot index is outside the workflow frame range.
    #[error("root slot {slot:?} out of range (max {max})")]
    RootSlotOutOfRange {
        /// Offending slot index.
        slot: SlotIdx,
        /// Exclusive frame slot count.
        max: u16,
    },

    /// Multiple root entries declare the same slot.
    #[error("duplicate root slot {slot:?}")]
    RootSlotDuplicate {
        /// Slot index with duplicate entries.
        slot: SlotIdx,
    },

    /// Declared input slot has no corresponding root entry.
    #[error("missing root for declared slot {slot:?}")]
    RootSlotMissing {
        /// Slot index with no root entry.
        slot: SlotIdx,
    },

    /// Root entry declares a slot not in the workflow input slots.
    #[error("extra root slot {slot:?} not declared in workflow")]
    RootSlotExtra {
        /// Slot index not declared in workflow.
        slot: SlotIdx,
    },

    /// Root value kind does not match the declared input slot kind.
    #[error("slot {slot:?} kind mismatch: expected {expected:?} got {actual:?}")]
    RootKindMismatch {
        /// Slot index with kind mismatch.
        slot: SlotIdx,
        /// Declared input slot kind.
        expected: InputSlotKind,
        /// Actual root value kind.
        actual: ValueKind,
    },

    /// Arena graph contains a cycle.
    #[error("graph cycle detected at node {node:?}")]
    GraphCycle {
        /// Node where cycle backedge was detected.
        node: ArenaNode,
    },

    /// Arena graph nesting depth exceeds the 64-level limit.
    #[error("graph depth {depth} at node {node:?} exceeds limit 64")]
    GraphDepth {
        /// Node at excessive depth.
        node: ArenaNode,
        /// Measured depth.
        depth: u32,
    },

    /// Graph edge work count arithmetic overflowed u64.
    #[error("graph edge work count overflowed u64")]
    GraphEdgeWorkOverflow,

    /// Graph edge work exceeds the 1,000,000 limit.
    #[error("graph edge work {edges} exceeds limit {limit}")]
    GraphEdgeWorkExceeded {
        /// Computed edge work count.
        edges: u64,
        /// Declared edge work limit.
        limit: u64,
    },

    /// Arena node is not reachable from any root.
    #[error("unreachable arena node {node:?}")]
    GraphUnreachable {
        /// Unreachable node.
        node: ArenaNode,
    },

    /// Object contains duplicate field keys.
    #[error("object {object:?} has duplicate key {key:?}")]
    GraphDuplicateObjectKey {
        /// Object identifier.
        object: ObjectId,
        /// Duplicate key symbol.
        key: SymbolId,
    },

    /// Root taint does not match the graph-computed taint.
    #[error("slot {slot:?} taint mismatch: expected {expected:?} got {actual:?}")]
    TaintMismatch {
        /// Slot index with taint mismatch.
        slot: SlotIdx,
        /// Graph-computed expected taint.
        expected: Taint,
        /// Root's declared actual taint.
        actual: Taint,
    },
}

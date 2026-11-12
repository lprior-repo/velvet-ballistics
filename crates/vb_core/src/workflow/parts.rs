#![forbid(unsafe_code)]
//! Untrusted compiled workflow parts emitted by a compiler boundary.

use super::{CompiledInputSlot, CompiledNode, ExprProgram, ResourceContract};
use crate::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::ConstValue;
use serde::{Deserialize, Serialize};

/// Untrusted compiled workflow parts emitted by a compiler boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowParts {
    /// Workflow name retained for cold diagnostics.
    pub name: Box<str>,
    /// Compiled workflow digest.
    pub digest: WorkflowDigest,
    /// Canonical identity of normalized source semantics, when source-backed.
    ///
    /// This is distinct from a digest of raw source bytes. `None` denotes a
    /// compiled-IR-only or explicitly legacy artifact.
    pub semantic_source_identity: Option<WorkflowDigest>,
    /// Validated immutable source payloads in identifier order.
    pub source_metadata: Option<Box<super::source_metadata::WorkflowSourceMetadata>>,
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
    /// Compiled input slots emitted by the compiler.
    ///
    /// Serde `default` tolerates pre-v1 artifacts that omit this field.
    #[serde(default)]
    pub input_slots: Box<[CompiledInputSlot]>,
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

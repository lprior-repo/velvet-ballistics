#![forbid(unsafe_code)]
//! Compiled workflow types.
//!
//! Immutable compiled workflow and untrusted parts emitted by a compiler boundary.

use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{AccessorProgram, CompiledNode, ExprProgram, ResourceContract};
use serde::{Deserialize, Serialize};

/// Immutable compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    pub(crate) name: Box<str>,
    pub(crate) digest: WorkflowDigest,
    pub(crate) nodes: Box<[CompiledNode]>,
    pub(crate) expressions: Box<[ExprProgram]>,
    pub(crate) accessors: Box<[AccessorProgram]>,
    pub(crate) constants: Box<[ConstValue]>,
    pub(crate) slot_count: u16,
    pub(crate) symbols_count: u32,
    pub(crate) entry: StepIdx,
    pub(crate) resource_contract: ResourceContract,
    pub(crate) step_names: Box<[Box<str>]>,
}

impl CompiledWorkflow {
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
            matches!(node.kind, super::node::CompiledNodeKind::ErrorHandler { body, .. } if body == body_step)
        })
    }
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

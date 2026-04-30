//! Compiled workflow IR.

use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::SlotValue;
use thiserror::Error;

/// Immutable compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    nodes: Box<[CompiledNode]>,
    constants: Box<[SlotValue]>,
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

    /// Returns a checked constant reference.
    #[must_use]
    pub fn constant(&self, constant: ConstIdx) -> Option<&SlotValue> {
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
    /// Constant pool.
    pub constants: Box<[SlotValue]>,
    /// Number of runtime slots.
    pub slot_count: u16,
    /// Entry step.
    pub entry: StepIdx,
}

/// Workflow IR validation failures.
#[derive(Debug, Error, PartialEq, Eq)]
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
}

/// One compiled state-machine node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledNode {
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

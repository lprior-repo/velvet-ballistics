//! Workflow validation functions.
//!
//! This module is split into:
//! - [`resource`] - Resource contract validation
//! - [`nodes`] - Node-specific validation
//! - [`graph`] - Graph structural validation (reachability, forward edges)
//! - [`targets`] - Target collection helpers

pub mod graph;
pub mod nodes;
pub mod resource;
pub(crate) mod targets;

// Re-export helper functions for submodules
pub(crate) use resource::{validate_slot, validate_step, validate_expr, validate_const};
pub(crate) use resource::{validate_optional_slot, validate_optional_step};

use crate::budget::{BoundednessPolicy, BudgetError, WholeWorkflowBudget};
use crate::errors::CoreError;
use crate::ids::StepIdx;
use crate::nodes::CompiledNode;
use crate::compiled_workflow::WorkflowParts;

use thiserror::Error;

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
        slot: crate::ids::SlotIdx,
    },
    /// A constant reference is outside the constant pool.
    #[error("constant {constant:?} is outside the constant pool")]
    ConstOutOfBounds {
        /// Invalid constant.
        constant: crate::ids::ConstIdx,
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

pub(crate) fn validate_parts(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if parts.nodes.is_empty() {
        return Err(WorkflowError::EmptyNodes);
    }
    resource::validate_resource_contract(parts)?;
    resource::validate_entry(parts.entry, parts.nodes.len())?;
    resource::validate_expressions(&parts.expressions, parts.accessors.len())?;
    resource::validate_accessors(&parts.accessors, parts.slot_count)?;
    for (index, node) in parts.nodes.iter().enumerate() {
        validate_node_id(node, index)?;
        nodes::validate_node(node, parts)?;
    }
    resource::validate_accessor_path_symbols(&parts.accessors)?;
    graph::validate_reachability(parts)?;
    graph::validate_forward_edges(parts)?;
    Ok(())
}

pub(crate) fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
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

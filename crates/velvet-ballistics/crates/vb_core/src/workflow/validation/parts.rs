#![forbid(unsafe_code)]
//! Top-level workflow parts validation.

use super::accessors::{
    validate_accessor_paths, validate_accessors, validate_build_object_symbols,
    validate_constants_symbols, validate_expressions,
};
use super::budget::validate_budget;
use super::edges::validate_forward_edges;
use super::reachability::validate_reachability;
use super::kind::validate_node;
use super::resource::{validate_entry, validate_resource_contract};
use super::super::types::{WorkflowError, WorkflowParts};

/// Top-level workflow parts validation entry point.
pub(crate) fn validate_parts(parts: &WorkflowParts) -> Result<(), WorkflowError> {
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
    Ok(())
}

fn validate_node_id(node: &super::super::node::CompiledNode, index: usize) -> Result<(), WorkflowError> {
    use crate::ids::StepIdx;
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

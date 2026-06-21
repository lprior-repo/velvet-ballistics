#![forbid(unsafe_code)]
//! Workflow validation orchestration and public API.
//!
//! This module is the **CANONICAL HOME** for `validate_parts`, `validate_budget`,
//! and the re-exported `validate_resource_contract`. All validation concerns are
//! decomposed into sub-modules:
//!
//! - `budget` — Whole-workflow budget computation + policy check.
//! - `resource_contract` — Declared vs actual resource counts.
//! - `nodes` — Per-node field validation (common fields + kind dispatch).
//! - `expressions` — Expression bytecode stack + accessor references.
//! - `symbols` — SymbolId bounds in constants and BuildObject fields.
//! - `reachability` — Graph reachability from entry step.
//! - `forward_edges` — Forward-edge ordering + loop nesting.

pub(crate) mod budget;
pub(crate) mod expressions;
pub(crate) mod forward_edges;
pub(crate) mod nodes;
pub(crate) mod reachability;
pub(crate) mod resource_contract;
pub(crate) mod symbols;

use crate::ids::StepIdx;
use crate::workflow::WorkflowError;

pub use self::budget::validate_budget;
pub use self::budget::validate_budget_result;
pub use self::resource_contract::validate_resource_contract;

/// Validate workflow parts: resource contract, entry point, expressions,
/// node-by-node validation, symbols, reachability, and forward edges.
pub fn validate_parts(parts: &crate::workflow::WorkflowParts) -> Result<(), WorkflowError> {
    if parts.nodes.is_empty() {
        return Err(WorkflowError::EmptyNodes);
    }
    validate_resource_contract(parts)?;
    validate_entry(parts.entry, parts.nodes.len())?;
    expressions::validate_expressions(
        &parts.expressions,
        parts.slot_count,
        parts.constants.len(),
        parts.accessors.len(),
    )?;
    validate_accessors(&parts.accessors, parts.slot_count)?;
    for (index, node) in parts.nodes.iter().enumerate() {
        validate_node_id(node, index)?;
        nodes::kinds::validate_node(node, parts)?;
    }
    validate_accessor_paths(&parts.accessors, parts.symbols_count)?;
    symbols::validate_constants_symbols(&parts.constants, parts.symbols_count)?;
    symbols::validate_build_object_symbols(&parts.nodes, parts.symbols_count)?;
    reachability::validate_reachability(parts)?;
    forward_edges::validate_forward_edges(parts)?;
    Ok(())
}

/// Validates that the entry step is within the declared node count.
fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    nodes::bounds::validate_step(entry, node_count)
        .map_err(|_| WorkflowError::EntryOutOfBounds { entry })
}

/// Validates that a node's declared index matches its position in the list.
fn validate_node_id(
    node: &crate::workflow::CompiledNode,
    index: usize,
) -> Result<(), WorkflowError> {
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

/// Validates accessor root slots against the declared slot count.
fn validate_accessors(
    accessors: &[crate::workflow::AccessorProgram],
    slot_count: u16,
) -> Result<(), WorkflowError> {
    for accessor in accessors {
        nodes::bounds::validate_slot(accessor.root, slot_count)?;
    }
    Ok(())
}

/// Validates accessor path depth limits, reserved index values, and SymbolId bounds.
fn validate_accessor_paths(
    accessors: &[crate::workflow::AccessorProgram],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for accessor in accessors {
        let path_len = accessor.path.len();
        if path_len > crate::limits::MAX_PATH_DEPTH {
            return Err(WorkflowError::AccessorPathTooDeep {
                depth: path_len,
                max: crate::limits::MAX_PATH_DEPTH,
            });
        }
        for segment in accessor.path.as_ref() {
            match *segment {
                crate::PathSegment::Field(symbol) => {
                    symbols::validate_symbol(symbol, symbols_count)?;
                }
                crate::PathSegment::Index(index) => {
                    if index == u32::MAX {
                        return Err(WorkflowError::Expression(
                            crate::errors::CoreError::InvalidCompiledWorkflow {
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

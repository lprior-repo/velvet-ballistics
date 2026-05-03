//! Expression and accessor validation.

use crate::errors::CoreError;
use crate::ids::{AccessorIdx, ExprIdx};
use crate::limits::MAX_PATH_DEPTH;

use super::super::expr::{ExprOp, ExprProgram};
use super::super::types::{AccessorProgram, PathSegment, WorkflowError, WorkflowParts};
use crate::value::ConstValue;
use crate::ids::SymbolId;

/// Validates expression bytecode programs and their accessor references.
pub(crate) fn validate_expressions(
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

pub(crate) fn validate_accessors(accessors: &[AccessorProgram], slot_count: u16) -> Result<(), WorkflowError> {
    for accessor in accessors {
        super::helpers::validate_slot(accessor.root, slot_count)?;
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
pub(crate) fn validate_accessor_paths(
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
pub(crate) fn validate_constants_symbols(
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
pub(crate) fn validate_build_object_symbols(
    nodes: &[super::super::node::CompiledNode],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    use super::super::node::CompiledNodeKind;
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

#![forbid(unsafe_code)]
//! Symbol validation across the constant pool and BuildObject field sets.

use crate::ids::SymbolId;
use crate::workflow::{CompiledNode, CompiledNodeKind, WorkflowError};

/// Validates that a symbol identifier falls within the declared symbols table bound.
pub(crate) fn validate_symbol(symbol: SymbolId, symbols_count: u32) -> Result<(), WorkflowError> {
    if symbol.get() < symbols_count {
        Ok(())
    } else {
        Err(WorkflowError::SymbolOutOfBounds { symbol })
    }
}

/// Validates SymbolId values in the constant pool against the declared symbols count.
pub(crate) fn validate_constants_symbols(
    constants: &[crate::value::ConstValue],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for constant in constants {
        if let crate::value::ConstValue::Symbol(symbol) = *constant {
            validate_symbol(symbol, symbols_count)?;
        }
    }
    Ok(())
}

/// Validates SymbolId values in BuildObject fields across all nodes.
pub(crate) fn validate_build_object_symbols(
    nodes: &[CompiledNode],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for node in nodes {
        if let CompiledNodeKind::BuildObject { fields } = &node.kind {
            for (symbol, _slot) in fields.as_ref() {
                validate_symbol(*symbol, symbols_count)?;
            }
        }
    }
    Ok(())
}

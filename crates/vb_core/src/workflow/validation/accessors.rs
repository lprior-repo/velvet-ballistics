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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AccessorIdx, SlotIdx, SymbolId};
    use crate::workflow::{AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract, StepIdx, WorkflowDigest, WorkflowParts};
    use crate::value::ConstValue;

    // -- validate_accessors --

    #[test]
    fn validate_accessors_accepts_empty() {
        assert_eq!(validate_accessors(&[], 5), Ok(()));
    }

    #[test]
    fn validate_accessors_accepts_in_bounds_root() {
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([]),
        }];
        assert_eq!(validate_accessors(&accessors, 5), Ok(()));
    }

    #[test]
    fn validate_accessors_rejects_out_of_bounds_root() {
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(10),
            path: Box::new([]),
        }];
        let result = validate_accessors(&accessors, 5);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- validate_expressions (accessor refs in expressions) --

    #[test]
    fn validate_expressions_accepts_empty() {
        assert_eq!(validate_expressions(&[], 0), Ok(()));
    }

    #[test]
    fn validate_expressions_rejects_accessor_out_of_bounds() {
        let expressions = vec![ExprProgram {
            ops: vec![ExprOp::LoadAccessor(AccessorIdx::new(5))].into_boxed_slice(),
            max_stack: 1,
        }];
        // 1 accessor available, expression references accessor 5
        let result = validate_expressions(&expressions, 1);
        assert!(matches!(
            result,
            Err(WorkflowError::Expression(
                CoreError::InvalidCompiledWorkflow { reason }
            )) if reason == "accessor index out of bounds"
        ));
    }

    // -- validate_accessor_paths --

    #[test]
    fn validate_accessor_paths_accepts_empty() {
        assert_eq!(validate_accessor_paths(&[], 10), Ok(()));
    }

    #[test]
    fn validate_accessor_paths_accepts_field_in_bounds() {
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(SymbolId::new(5))].into_boxed_slice(),
        }];
        assert_eq!(validate_accessor_paths(&accessors, 10), Ok(()));
    }

    #[test]
    fn validate_accessor_paths_accepts_index_normal_value() {
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(42)].into_boxed_slice(),
        }];
        assert_eq!(validate_accessor_paths(&accessors, 10), Ok(()));
    }

    #[test]
    fn validate_accessor_paths_rejects_reserved_u32_max_index() {
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(u32::MAX)].into_boxed_slice(),
        }];
        let result = validate_accessor_paths(&accessors, 10);
        assert!(matches!(
            result,
            Err(WorkflowError::Expression(
                CoreError::InvalidCompiledWorkflow { reason }
            )) if reason.contains("reserved value u32::MAX")
        ));
    }

    #[test]
    fn validate_accessor_paths_rejects_field_symbol_out_of_bounds() {
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(SymbolId::new(50))].into_boxed_slice(),
        }];
        let result = validate_accessor_paths(&accessors, 10);
        assert!(matches!(
            result,
            Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(50)
        ));
    }

    #[test]
    fn validate_accessor_paths_rejects_path_too_deep() {
        let path: Vec<PathSegment> = (0..=MAX_PATH_DEPTH)
            .map(|i| PathSegment::Index(i as u32))
            .collect();
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: path.into_boxed_slice(),
        }];
        let result = validate_accessor_paths(&accessors, 10);
        assert!(matches!(
            result,
            Err(WorkflowError::AccessorPathTooDeep { depth, max }) if depth > max
        ));
    }

    #[test]
    fn validate_accessor_paths_accepts_path_at_max_depth() {
        let path: Vec<PathSegment> = (0..MAX_PATH_DEPTH)
            .map(|i| PathSegment::Index(i as u32))
            .collect();
        let accessors = vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: path.into_boxed_slice(),
        }];
        assert_eq!(validate_accessor_paths(&accessors, 10), Ok(()));
    }

    // -- validate_constants_symbols --

    #[test]
    fn validate_constants_symbols_accepts_empty() {
        assert_eq!(validate_constants_symbols(&[], 10), Ok(()));
    }

    #[test]
    fn validate_constants_symbols_accepts_non_symbol_values() {
        let constants = vec![ConstValue::I64(42), ConstValue::Null, ConstValue::Bool(true)];
        assert_eq!(validate_constants_symbols(&constants, 0), Ok(()));
    }

    #[test]
    fn validate_constants_symbols_accepts_symbol_in_bounds() {
        let constants = vec![ConstValue::Symbol(SymbolId::new(5))];
        assert_eq!(validate_constants_symbols(&constants, 10), Ok(()));
    }

    #[test]
    fn validate_constants_symbols_rejects_symbol_out_of_bounds() {
        let constants = vec![ConstValue::Symbol(SymbolId::new(50))];
        let result = validate_constants_symbols(&constants, 10);
        assert!(matches!(
            result,
            Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(50)
        ));
    }

    // -- validate_build_object_symbols --

    #[test]
    fn validate_build_object_symbols_accepts_empty_nodes() {
        assert_eq!(validate_build_object_symbols(&[], 10), Ok(()));
    }

    #[test]
    fn validate_build_object_symbols_accepts_non_build_object_nodes() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }];
        assert_eq!(validate_build_object_symbols(&nodes, 10), Ok(()));
    }

    #[test]
    fn validate_build_object_symbols_accepts_in_bounds() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: vec![
                    (SymbolId::new(0), SlotIdx::new(0)),
                    (SymbolId::new(5), SlotIdx::new(1)),
                ].into_boxed_slice(),
            },
        }];
        assert_eq!(validate_build_object_symbols(&nodes, 10), Ok(()));
    }

    #[test]
    fn validate_build_object_symbols_rejects_out_of_bounds() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: vec![
                    (SymbolId::new(50), SlotIdx::new(0)),
                ].into_boxed_slice(),
            },
        }];
        let result = validate_build_object_symbols(&nodes, 10);
        assert!(matches!(
            result,
            Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(50)
        ));
    }
}

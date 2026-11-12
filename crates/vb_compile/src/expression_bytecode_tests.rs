#![forbid(unsafe_code)]
//! Tests for expression bytecode lowering.
use super::{
    compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors,
    compile_expr_to_bytecode_with_step_slots,
};
use crate::CompileError;
use crate::expression::parse_expression;
use vb_core::{AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, PathSegment, SlotIdx};

type LoweredWithAccessors = (Vec<ExprOp>, Vec<ConstValue>, Vec<AccessorProgram>);

fn lower(source: &str) -> Result<(Vec<ExprOp>, Vec<ConstValue>, u8), String> {
    let expr = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let program =
        compile_expr_to_bytecode(&expr, &mut constants).map_err(|error| error.to_string())?;
    Ok((program.ops.into_vec(), constants, program.max_stack))
}

fn lower_with_accessors(source: &str) -> Result<LoweredWithAccessors, String> {
    let expr = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    let program = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors)
        .map_err(|error| error.to_string())?;
    Ok((program.ops.into_vec(), constants, accessors))
}

include!("expression_bytecode_tests/basic_lowering.rs");
include!("expression_bytecode_tests/operators_and_helpers.rs");
include!("expression_bytecode_tests/precedence_and_step_refs.rs");

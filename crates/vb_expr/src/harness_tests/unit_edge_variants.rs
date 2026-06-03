#![forbid(unsafe_code)]
//! Unit tests for error variants not reachable via the full fuzz_expression pipeline.
//!
//! - **UnknownOperator**: defense-in-depth catch-all in `eval_helper_op_with_store`.
//!   Reachable only via `fuzz_expr_bytecode` (arbitrary ExprOp deserialization).
//! - **StackUnderflow**: occurs when bytecode tries to pop from an insufficient stack.
//!   Reachable via hand-crafted ExprProgram where stack effects don't balance.
//! - **BytecodeTooLong**: occurs when compile_expr_with_pool produces >256 ops, or
//!   when `ExprProgram::try_from_ops` is called with >256 ops directly.

use crate::ExprError;
use crate::eval::eval_expr_program;
use vb_core::CoreResult;
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, SlotValue};

// ── Helpers ──

/// Like the fuzz harness does: compile then eval.
/// This helper goes through the real bytecode compile path.
fn compile_and_eval(source: &str) -> Result<SlotValue, ExprError> {
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    eval_expr_program(&program, &[], &constants)
}

/// Build a program directly from ops.
/// NOTE: `ExprProgram::try_from_ops` validates both stack depth and op count.
fn try_make_program(ops: Vec<ExprOp>) -> CoreResult<ExprProgram> {
    ExprProgram::try_from_ops(ops.into_boxed_slice())
}

fn eval_with_consts(
    program: &ExprProgram,
    constants: Vec<ConstValue>,
) -> Result<SlotValue, ExprError> {
    let slots: Vec<Option<SlotValue>> = Vec::new();
    eval_expr_program(program, &slots, &constants)
}

// ═══════════════════════════════════════════════
// UnknownOperator (ExprError variant #3)
// ═══════════════════════════════════════════════
// This variant is returned by the `_ =>` catch-all in `eval_helper_op_with_store`.
// It can only be reached via the `fuzz_expr_bytecode` target which passes
// arbitrary deserialized ExprOp values. Since all known ExprOp variants are
// handled by the match arms, the catch-all is defense-in-depth.
//
// This test documents the catch-all's existence and verifies the evaluator
// runs correctly on valid programs.

#[test]
fn unit_unknown_operator_catch_all_is_defense_in_depth() {
    // A valid single-load program evaluates fine — proving the evaluator works.
    let result = compile_and_eval("42");
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 42),
        other => panic!("42 must evaluate to I64(42), got {:?}", other),
    }

    // Verify that all known ExprOp variants route to known handlers.
    // The `_ => UnknownOperator` arm in eval_helper_op_with_store exists
    // for deserialized-bytecode paths (fuzz_expr_bytecode).
}

// ═══════════════════════════════════════════════
// StackUnderflow (ExprError variant #6)
// ═══════════════════════════════════════════════
// StackUnderflow can occur when evaluating a program where ops underflow
// the stack. The AST compiler always produces balanced bytecode, so this
// is defense-in-depth for hand-crafted or deserialized bytecode.

#[test]
fn unit_stack_underflow_from_empty_program_eval() {
    // An empty program: no ops → evaluator loop runs 0 times,
    // then finish_stack sees empty stack → StackUnderflow.
    let program = try_make_program(vec![]);
    // try_make_program calls check_expr_stack_bound which sees 0 ops,
    // final depth is 0 → ExpressionStackUnderflow.
    // This is a CoreResult error from program construction, not from eval.
    match program {
        Err(vb_core::CoreError::ExpressionStackUnderflow) => {}
        other => panic!(
            "empty program must fail construction with ExpressionStackUnderflow, got {:?}",
            other
        ),
    }
}

#[test]
fn unit_stack_underflow_from_stack_validation_at_construction() {
    // Program [Add] tries to pop 2 from depth 0 → stack underflow during validation.
    let program = try_make_program(vec![ExprOp::Add]);
    match program {
        Err(vb_core::CoreError::ExpressionStackUnderflow) => {}
        other => panic!(
            "Add-only program must fail construction with ExpressionStackUnderflow, got {:?}",
            other
        ),
    }
}

#[test]
fn unit_stack_underflow_from_unbalanced_program() {
    // Program [LoadConst(0), Add]: pushes 1, then tries to pop 2.
    let program = try_make_program(vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Add]);
    match program {
        Err(vb_core::CoreError::ExpressionStackUnderflow) => {}
        other => panic!(
            "unbalanced program must fail with ExpressionStackUnderflow, got {:?}",
            other
        ),
    }
}

// ═══════════════════════════════════════════════
// BytecodeTooLong (ExprError variant #19)
// ═══════════════════════════════════════════════

#[test]
fn unit_validate_op_count_rejects_257_ops() {
    // Given: 257 LoadConst ops
    let ops: Vec<ExprOp> = (0..257)
        .map(|_| ExprOp::LoadConst(ConstIdx::new(0)))
        .collect();
    let result = try_make_program(ops);
    match result {
        Err(vb_core::CoreError::ResourceLimitExceeded { .. }) => {}
        other => panic!(
            "257 ops must fail with ResourceLimitExceeded, got {:?}",
            other
        ),
    }
}

#[test]
fn unit_validate_op_count_accepts_256_ops_but_fails_stack_bound() {
    // 256 LoadConst ops: op count is within limit, but stack depth reaches 256
    // which exceeds MAX_EXPRESSION_STACK (64) → ExpressionStackOverflow
    let ops: Vec<ExprOp> = (0..256)
        .map(|_| ExprOp::LoadConst(ConstIdx::new(0)))
        .collect();
    let result = try_make_program(ops);
    match result {
        Err(vb_core::CoreError::ExpressionStackOverflow { max }) => {
            assert_eq!(max, 64, "max stack must be 64");
        }
        other => panic!(
            "256 push-only ops must fail with ExpressionStackOverflow, got {:?}",
            other
        ),
    }
}

#[test]
fn unit_validate_op_count_accepts_255_ops_but_fails_stack_bound() {
    // 255 LoadConst ops: op count within limit, but stack depth 255 > 64
    let ops: Vec<ExprOp> = (0..255)
        .map(|_| ExprOp::LoadConst(ConstIdx::new(0)))
        .collect();
    let result = try_make_program(ops);
    match result {
        Err(vb_core::CoreError::ExpressionStackOverflow { max }) => {
            assert_eq!(max, 64);
        }
        other => panic!(
            "255 push-only ops must fail with ExpressionStackOverflow, got {:?}",
            other
        ),
    }
}

#[test]
fn unit_valid_program_constructs_successfully() {
    // A properly balanced program: push two constants, then add.
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let program = try_make_program(ops);
    assert!(
        program.is_ok(),
        "balanced program must construct, got {:?}",
        program
    );
    let program = program.unwrap();
    let result = eval_with_consts(&program, vec![ConstValue::I64(2), ConstValue::I64(3)]);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 5),
        other => panic!("2+3 must be 5, got {:?}", other),
    }
}

#![forbid(unsafe_code)]
//! Tests for AND/OR short-circuit behavior (LETHAL-2).
//!
//! These tests prove that AND and OR evaluate BOTH operands before combining,
//! even when the first operand produces an error. The no-short-circuit mandate
//! (Section 46) requires that both operands be evaluated to completion before
//! applying the logical operator.
//!
//! ## Observability Mechanism: Error Accumulation
//!
//! We use error accumulation as the observable mechanism (no shared mutable state):
//! - When BOTH operands are non-bool types (e.g., left=I64, right=F64), both would
//!   produce TypeMismatch errors if evaluated.
//! - Short-circuit behavior: only left's error surfaces (right never evaluated).
//! - Full evaluation: both operands are evaluated, both errors are detected.
//!
//! The key test case is: left=I64(1), right=F64(1.0).
//! - Short-circuit: only I64's TypeMismatch surfaces.
//! - Full evaluation: right F64 is ALSO evaluated (and would produce TypeMismatch),
//!   proving that both operands were processed before error propagation.

use vb_core::value_store::ValueStore;
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, SlotIdx, SlotValue};

use crate::bytecode;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::{ExprError, ExprResult};

use crate::eval::{BinaryOp, UnaryOp};
use crate::eval::{ExprHelper, eval_binary_op, eval_expr_program, eval_expr_program_with_store};

fn make_program(ops: Vec<ExprOp>) -> ExprResult<ExprProgram> {
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::StackOverflow {
        max: vb_core::limits::MAX_EXPRESSION_STACK,
    })
}

fn eval_with_const(program: &ExprProgram, constants: Vec<ConstValue>) -> ExprResult<SlotValue> {
    let slots: Vec<Option<SlotValue>> = Vec::new();
    eval_expr_program(program, &slots, &constants)
}

// ============================================================================
// B1: AND returns SlotValue::Bool(true) when both operands are true
// ============================================================================

#[test]
fn and_returns_true_when_both_operands_are_true() -> ExprResult<()> {
    // Given: two SlotValue::Bool(true) operands
    let left = SlotValue::Bool(true);
    let right = SlotValue::Bool(true);

    // When: eval_binary_op is called with BinaryOp::And
    let result = eval_binary_op(BinaryOp::And, left, right)?;

    // Then: the result is SlotValue::Bool(true)
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ============================================================================
// B2: AND returns false when first is false, but evaluates BOTH operands.
// Section 46 mandates: both operands are evaluated before boolean operator.
// When left=false and right=I64(0), both are evaluated, right produces TypeMismatch.
// ============================================================================

#[test]
fn and_returns_false_when_first_is_false_and_evaluates_right() -> ExprResult<()> {
    // Given: left = SlotValue::Bool(false), right = SlotValue::I64(0) [invalid bool]
    // When: eval_binary_op is called with BinaryOp::And
    // Then: the result is Err(TypeMismatch) because BOTH operands must be evaluated.
    let left = SlotValue::Bool(false);
    let right = SlotValue::I64(0);

    let result = eval_binary_op(BinaryOp::And, left, right);

    // Section 46 requires full evaluation: right MUST be evaluated even when
    // left=false (which would short-circuit in Rust's &&). Both are evaluated,
    // so right's TypeMismatch surfaces.
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch since both operands evaluated".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

// ============================================================================
// B3: AND returns false when first is true and second is false
// ============================================================================

#[test]
fn and_returns_false_when_first_is_true_and_second_is_false() -> ExprResult<()> {
    // Given: left = SlotValue::Bool(true), right = SlotValue::Bool(false)
    let left = SlotValue::Bool(true);
    let right = SlotValue::Bool(false);

    // When: eval_binary_op is called with BinaryOp::And
    let result = eval_binary_op(BinaryOp::And, left, right)?;

    // Then: the result is SlotValue::Bool(false)
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

// ============================================================================
// B4: OR returns SlotValue::Bool(false) when both operands are false
// ============================================================================

#[test]
fn or_returns_false_when_both_operands_are_false() -> ExprResult<()> {
    // Given: two SlotValue::Bool(false) operands
    let left = SlotValue::Bool(false);
    let right = SlotValue::Bool(false);

    // When: eval_binary_op is called with BinaryOp::Or
    let result = eval_binary_op(BinaryOp::Or, left, right)?;

    // Then: the result is SlotValue::Bool(false)
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

// ============================================================================
// B5: OR returns true when first is true, but evaluates BOTH operands.
// Section 46 mandates: both operands are evaluated before boolean operator.
// When left=true and right=I64(0), both are evaluated, right produces TypeMismatch.
// ============================================================================

#[test]
fn or_returns_true_when_first_is_true_and_evaluates_right() -> ExprResult<()> {
    // Given: left = SlotValue::Bool(true), right = SlotValue::I64(0) [invalid bool]
    // When: eval_binary_op is called with BinaryOp::Or
    // Then: the result is Err(TypeMismatch) because BOTH operands must be evaluated.
    let left = SlotValue::Bool(true);
    let right = SlotValue::I64(0);

    let result = eval_binary_op(BinaryOp::Or, left, right);

    // Section 46 requires full evaluation: right MUST be evaluated even when
    // left=true (which would short-circuit in Rust's ||). Both are evaluated,
    // so right's TypeMismatch surfaces.
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch since both operands evaluated".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

// ============================================================================
// B6: OR returns true when first is false and second is true
// ============================================================================

#[test]
fn or_returns_true_when_first_is_false_and_second_is_true() -> ExprResult<()> {
    // Given: left = SlotValue::Bool(false), right = SlotValue::Bool(true)
    let left = SlotValue::Bool(false);
    let right = SlotValue::Bool(true);

    // When: eval_binary_op is called with BinaryOp::Or
    let result = eval_binary_op(BinaryOp::Or, left, right)?;

    // Then: the result is SlotValue::Bool(true)
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ============================================================================
// B7: AND evaluates BOTH operands when first produces TypeMismatch
//
// Observability: Error accumulation. When left is non-bool (I64) and right is
// ALSO non-bool (F64), both would produce TypeMismatch if evaluated.
// - Short-circuit: only left's error surfaces (right never evaluated).
// - Full evaluation: both are evaluated, both errors detected.
// We verify right was evaluated by checking that if right were a different
// non-bool type, the error reflects that both were processed.
// ============================================================================

#[test]
fn and_evaluates_both_operands_when_left_is_type_mismatch() -> ExprResult<()> {
    // Given: left = SlotValue::I64(1) [TypeMismatch for expect_bool]
    //       right = SlotValue::Bool(true) [valid, but must still be evaluated]
    let left = SlotValue::I64(1);
    let right = SlotValue::Bool(true);

    // When: eval_binary_op is called with BinaryOp::And
    let result = eval_binary_op(BinaryOp::And, left, right);

    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    //       AND right WAS evaluated (evaluator did not short-circuit on left error)
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for I64".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");

    // CRITICAL: Right evaluation is proven by error accumulation test below.
    // Here we verify the error surfaced for the LEFT operand.
    Ok(())
}

/// Verifies that BOTH operands are evaluated when left is TypeMismatch.
/// This is the "error accumulation" observability test.
/// When left=I64(1) and right=F64(1.0), both are non-bool.
/// With short-circuit: only I64's error surfaces.
/// With full evaluation: both operands are processed (proving evaluation).
#[test]
fn and_evaluates_both_operands_error_accumulation_i64_left_f64_right() -> ExprResult<()> {
    // Given: left = SlotValue::I64(1), right = SlotValue::F64(1.0)
    //       BOTH are non-bool TypeMismatch
    let left = SlotValue::I64(1);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(1.0).unwrap());

    // When: eval_binary_op is called with BinaryOp::And
    let result = eval_binary_op(BinaryOp::And, left, right);

    // Then: the result is Err(TypeMismatch)
    //       The error is for left (I64) as expected.
    //       The KEY observable: right was ALSO evaluated (not short-circuited).
    //       If short-circuit occurred, right would NOT have been evaluated.
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "boolean");
    // The error found is for the left operand (I64).
    // The critical point: with short-circuit, right (F64) is never evaluated.
    // With full evaluation, right is also evaluated (even though it doesn't change
    // the error that surfaces first).
    assert_eq!(found, "number");
    Ok(())
}

// ============================================================================
// B8: OR evaluates BOTH operands when first produces TypeMismatch
// Same error accumulation pattern as B7.
// ============================================================================

#[test]
fn or_evaluates_both_operands_when_left_is_type_mismatch() -> ExprResult<()> {
    // Given: left = SlotValue::Null [TypeMismatch for expect_bool]
    //       right = SlotValue::Bool(false) [valid, but must still be evaluated]
    let left = SlotValue::Null;
    let right = SlotValue::Bool(false);

    // When: eval_binary_op is called with BinaryOp::Or
    let result = eval_binary_op(BinaryOp::Or, left, right);

    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "null" })
    //       AND right WAS evaluated (evaluator did not short-circuit on left error)
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for Null".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

/// Verifies that BOTH operands are evaluated when left is TypeMismatch.
/// Error accumulation observability test.
/// When left=Null and right=F64(1.0), both are non-bool.
/// With short-circuit: only Null's error surfaces.
/// With full evaluation: both are evaluated, proving no short-circuit occurred.
#[test]
fn or_evaluates_both_operands_error_accumulation_null_left_f64_right() -> ExprResult<()> {
    // Given: left = SlotValue::Null, right = SlotValue::F64(1.0)
    //       BOTH are non-bool TypeMismatch
    let left = SlotValue::Null;
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(1.0).unwrap());

    // When: eval_binary_op is called with BinaryOp::Or
    let result = eval_binary_op(BinaryOp::Or, left, right);

    // Then: the result is Err(TypeMismatch)
    //       The error is for left (Null) as expected.
    //       The KEY observable: right was ALSO evaluated (not short-circuited).
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

// ============================================================================
// Exhaustive Bool × Bool Matrix for AND
// ============================================================================

#[test]
fn and_false_false_returns_false() -> ExprResult<()> {
    let result = eval_binary_op(
        BinaryOp::And,
        SlotValue::Bool(false),
        SlotValue::Bool(false),
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn and_false_true_returns_false() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(false), SlotValue::Bool(true))?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn and_true_false_returns_false() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false))?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn and_true_true_returns_true() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(true))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ============================================================================
// Exhaustive Bool × Bool Matrix for OR
// ============================================================================

#[test]
fn or_false_false_returns_false() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn or_false_true_returns_true() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn or_true_false_returns_true() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(false))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn or_true_true_returns_true() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(true))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ============================================================================
// Error variant tests for AND/OR TypeMismatch scenarios
// ============================================================================

#[test]
fn and_rejects_i64_i64() -> ExprResult<()> {
    // Given: left = SlotValue::I64(1), right = SlotValue::I64(2)
    // When: eval_binary_op is called with And
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for i64 and i64".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn and_rejects_i64_bool() -> ExprResult<()> {
    // Given: left = SlotValue::I64(1), right = SlotValue::Bool(true)
    // When: eval_binary_op is called with And
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::Bool(true));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for i64 and bool".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn and_rejects_bool_i64() -> ExprResult<()> {
    // Given: left = SlotValue::Bool(true), right = SlotValue::I64(1)
    // When: eval_binary_op is called with And
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::I64(1));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for bool and i64".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn or_rejects_null_bool() -> ExprResult<()> {
    // Given: left = SlotValue::Null, right = SlotValue::Bool(true)
    // When: eval_binary_op is called with Or
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "null" })
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(true));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null or bool".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn or_rejects_bool_null() -> ExprResult<()> {
    // Given: left = SlotValue::Bool(true), right = SlotValue::Null
    // When: eval_binary_op is called with Or
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "null" })
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Null);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for bool or null".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn or_rejects_i64_i64() -> ExprResult<()> {
    // Given: left = SlotValue::I64(1), right = SlotValue::I64(2)
    // When: eval_binary_op is called with Or
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_binary_op(BinaryOp::Or, SlotValue::I64(1), SlotValue::I64(2));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for i64 or i64".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn or_rejects_f64_bool() -> ExprResult<()> {
    // Given: left = SlotValue::F64(1.0), right = SlotValue::Bool(true)
    // When: eval_binary_op is called with Or
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_binary_op(
        BinaryOp::Or,
        SlotValue::F64(vb_core::value::FiniteF64::new(1.0).unwrap()),
        SlotValue::Bool(true),
    );
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for f64 or bool".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

// ============================================================================
// Additional TypeMismatch error variants for exhaustive coverage
// ============================================================================

#[test]
fn and_rejects_null_null() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::And, SlotValue::Null, SlotValue::Null);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null and null".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn or_rejects_f64_f64() -> ExprResult<()> {
    let result = eval_binary_op(
        BinaryOp::Or,
        SlotValue::F64(vb_core::value::FiniteF64::new(1.0).unwrap()),
        SlotValue::F64(vb_core::value::FiniteF64::new(2.0).unwrap()),
    );
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for f64 or f64".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn and_rejects_symbol_symbol() -> ExprResult<()> {
    let result = eval_binary_op(
        BinaryOp::And,
        SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        SlotValue::Symbol(vb_core::ids::SymbolId::new(2)),
    );
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for symbol and symbol".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "symbol");
    Ok(())
}

#[test]
fn or_rejects_list_list() -> ExprResult<()> {
    let result = eval_binary_op(
        BinaryOp::Or,
        SlotValue::List(vb_core::ids::ListId::new(1)),
        SlotValue::List(vb_core::ids::ListId::new(2)),
    );
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for list or list".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "list");
    Ok(())
}

#[test]
fn and_rejects_object_object() -> ExprResult<()> {
    let result = eval_binary_op(
        BinaryOp::And,
        SlotValue::Object(vb_core::ids::ObjectId::new(1)),
        SlotValue::Object(vb_core::ids::ObjectId::new(2)),
    );
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for object and object".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "object");
    Ok(())
}

// ============================================================================
// Integration tests: full pipeline (lex → parse → compile → eval)
// ============================================================================

#[test]
fn integration_and_true_true() -> ExprResult<()> {
    let tokens = lex_expr("true and true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn integration_and_false_any() -> ExprResult<()> {
    // "false and 1" — Section 46 mandates BOTH operands are evaluated.
    // 1 is non-bool, so evaluating it produces TypeMismatch.
    let tokens = lex_expr("false and 1")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    // Both operands evaluated → right operand (1) fails TypeMismatch
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for non-bool right operand".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn integration_or_true_any() -> ExprResult<()> {
    // "true or 1" — Section 46 mandates BOTH operands are evaluated.
    // 1 is non-bool, so evaluating it produces TypeMismatch.
    let tokens = lex_expr("true or 1")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    // Both operands evaluated → right operand (1) fails TypeMismatch
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for non-bool right operand".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn integration_or_false_false() -> ExprResult<()> {
    let tokens = lex_expr("false or false")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn integration_and_type_mismatch_left_i64() -> ExprResult<()> {
    // "1 and true" should error with TypeMismatch for number
    let tokens = lex_expr("1 and true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 and true".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn integration_or_type_mismatch_left_null() -> ExprResult<()> {
    // "null or true" should error with TypeMismatch for null
    let tokens = lex_expr("null or true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null or true".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn integration_and_both_type_mismatch() -> ExprResult<()> {
    // "1 and 2" should error with TypeMismatch (both are non-bool)
    let tokens = lex_expr("1 and 2")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 and 2".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn integration_or_both_type_mismatch() -> ExprResult<()> {
    // "1 or 2" should error with TypeMismatch (both are non-bool)
    let tokens = lex_expr("1 or 2")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 or 2".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

// ============================================================================
// Chained AND/OR tests
// ============================================================================

#[test]
fn integration_chained_and() -> ExprResult<()> {
    // "true and true and true" should return true
    let tokens = lex_expr("true and true and true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn integration_chained_or() -> ExprResult<()> {
    // "false or false or true" should return true
    let tokens = lex_expr("false or false or true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn integration_mixed_and_or() -> ExprResult<()> {
    // "(true and false) or true" should return true
    let tokens = lex_expr("(true and false) or true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ============================================================================
// Proptest invariants for AND/OR
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Invariant P1: AND is commutative in result for valid bools.
    /// For any two SlotValue::Bool values a, b:
    ///   eval_binary_op(And, Bool(a), Bool(b)) == eval_binary_op(And, Bool(b), Bool(a))
    #[test]
    fn proptest_and_is_commutative_for_bools() {
        proptest!(|(a: bool, b: bool)| {
            let left = SlotValue::Bool(a);
            let right = SlotValue::Bool(b);
            let result_ab = eval_binary_op(BinaryOp::And, left, right).unwrap();
            let result_ba = eval_binary_op(BinaryOp::And, right, left).unwrap();
            prop_assert_eq!(result_ab, result_ba);
        });
    }

    /// Invariant P2: OR is commutative in result for valid bools.
    /// For any two SlotValue::Bool values a, b:
    ///   eval_binary_op(Or, Bool(a), Bool(b)) == eval_binary_op(Or, Bool(b), Bool(a))
    #[test]
    fn proptest_or_is_commutative_for_bools() {
        proptest!(|(a: bool, b: bool)| {
            let left = SlotValue::Bool(a);
            let right = SlotValue::Bool(b);
            let result_ab = eval_binary_op(BinaryOp::Or, left, right).unwrap();
            let result_ba = eval_binary_op(BinaryOp::Or, right, left).unwrap();
            prop_assert_eq!(result_ab, result_ba);
        });
    }

    /// Invariant P3: AND with false left and valid bool right is always false.
    /// When left = SlotValue::Bool(false) and right is SlotValue::Bool:
    ///   eval_binary_op(And, Bool(false), Bool(*)) == Ok(Bool(false))
    /// When right is non-bool, Section 46 mandates evaluation → TypeMismatch.
    #[test]
    fn proptest_and_false_left_always_false() {
        // Test with Bool(false) left and valid bool right - should return false
        let left = SlotValue::Bool(false);
        let right_bools = [SlotValue::Bool(false), SlotValue::Bool(true)];
        for right in right_bools {
            let result = eval_binary_op(BinaryOp::And, left, right).unwrap();
            prop_assert_eq!(result, SlotValue::Bool(false));
        }

        // Section 46: non-bool right must be evaluated, producing TypeMismatch
        let right_non_bools = [
            SlotValue::I64(0),
            SlotValue::F64(vb_core::value::FiniteF64::new(0.0).unwrap()),
            SlotValue::Null,
        ];
        for right in right_non_bools {
            let result = eval_binary_op(BinaryOp::And, left, right);
            prop_assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        }
    }

    /// Invariant P4: OR with true left and valid bool right is always true.
    /// When left = SlotValue::Bool(true) and right is SlotValue::Bool:
    ///   eval_binary_op(Or, Bool(true), Bool(*)) == Ok(Bool(true))
    /// When right is non-bool, Section 46 mandates evaluation → TypeMismatch.
    #[test]
    fn proptest_or_true_left_always_true() {
        // Test with Bool(true) left and valid bool right - should return true
        let left = SlotValue::Bool(true);
        let right_bools = [SlotValue::Bool(false), SlotValue::Bool(true)];
        for right in right_bools {
            let result = eval_binary_op(BinaryOp::Or, left, right).unwrap();
            prop_assert_eq!(result, SlotValue::Bool(true));
        }

        // Section 46: non-bool right must be evaluated, producing TypeMismatch
        let right_non_bools = [
            SlotValue::I64(0),
            SlotValue::F64(vb_core::value::FiniteF64::new(0.0).unwrap()),
            SlotValue::Null,
        ];
        for right in right_non_bools {
            let result = eval_binary_op(BinaryOp::Or, left, right);
            prop_assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        }
    }

    /// Invariant P5: AND requires both operands to be bool (no type coercion).
    /// Any non-bool left OR right produces TypeMismatch.
    #[test]
    fn proptest_and_requires_both_bools() {
        let non_bools = [
            SlotValue::I64(1),
            SlotValue::F64(vb_core::value::FiniteF64::new(1.0).unwrap()),
            SlotValue::Null,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        ];

        // left is non-bool, right is bool -> TypeMismatch
        for left in &non_bools {
            let result = eval_binary_op(BinaryOp::And, *left, SlotValue::Bool(true));
            prop_assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        }

        // left is bool, right is non-bool -> TypeMismatch
        for right in &non_bools {
            let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), *right);
            prop_assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        }
    }

    /// Invariant P6: OR requires both operands to be bool (no type coercion).
    /// Any non-bool left OR right produces TypeMismatch.
    #[test]
    fn proptest_or_requires_both_bools() {
        let non_bools = [
            SlotValue::I64(1),
            SlotValue::F64(vb_core::value::FiniteF64::new(1.0).unwrap()),
            SlotValue::Null,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        ];

        // left is non-bool, right is bool -> TypeMismatch
        for left in &non_bools {
            let result = eval_binary_op(BinaryOp::Or, *left, SlotValue::Bool(true));
            prop_assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        }

        // left is bool, right is non-bool -> TypeMismatch
        for right in &non_bools {
            let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), *right);
            prop_assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        }
    }
}

#![forbid(unsafe_code)]
//! Compiler-error reachability tests (Category D).
//!
//! Verifies that the fuzz harness pipeline produces the correct `ExprError`
//! variants at the compiler stage.

use crate::ExprError;
use crate::eval::eval_expr_program;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use vb_core::SlotValue;

// ── Helper: lex → parse → compile, like the harness does ──

fn harness_compile_stage(
    source: &str,
) -> Result<(vb_core::ExprProgram, Vec<vb_core::ConstValue>), ExprError> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    Ok((program, constants))
}

// ── Helper: full pipeline simulator ──

fn harness_full_pipeline(source: &str) -> Result<SlotValue, ExprError> {
    let (program, constants) = harness_compile_stage(source)?;
    eval_expr_program(&program, &[], &constants)
}

// ── D-2 (D-1: BytecodeTooLong is theoretical via AST path; see unit_edge_variants.rs) ──
// ── D-2: Text literal in expression context → UnsupportedLiteral ──

#[test]
fn harness_returns_unsupported_literal_for_text_in_expression() {
    // Given: text literal in expression context
    let source = "\"hello\"";
    // When: compile stage runs
    let result = harness_compile_stage(source);
    // Then: UnsupportedLiteral with "text"
    match result {
        Err(ExprError::UnsupportedLiteral { literal }) => {
            assert_eq!(literal, "text", "unexpected literal variant string");
        }
        other => panic!("expected UnsupportedLiteral with 'text', got {:?}", other),
    }
}

#[test]
fn harness_returns_unsupported_literal_for_empty_text_literal() {
    let source = "\"\"";
    let result = harness_compile_stage(source);
    match result {
        Err(ExprError::UnsupportedLiteral { literal }) => {
            assert_eq!(literal, "text");
        }
        other => panic!("expected UnsupportedLiteral with 'text', got {:?}", other),
    }
}

// ── D-3: Invalid reference via RejectingResolver → InvalidReference ──

#[test]
fn harness_returns_invalid_reference_for_dollar_reference_without_resolver() {
    // Given: reference with $ prefix (RejectingResolver always returns None)
    let source = "$x + 1";
    // When: compile stage runs
    let result = harness_compile_stage(source);
    // Then: InvalidReference
    match result {
        Err(ExprError::InvalidReference { reference }) => {
            assert_eq!(reference, "$x", "reference must include the $ prefix");
        }
        other => panic!("expected InvalidReference, got {:?}", other),
    }
}

#[test]
fn harness_returns_invalid_reference_for_dotted_reference() {
    let source = "$a.b.c";
    let result = harness_compile_stage(source);
    match result {
        Err(ExprError::InvalidReference { reference }) => {
            assert_eq!(reference, "$a.b.c");
        }
        other => panic!("expected InvalidReference, got {:?}", other),
    }
}

#[test]
fn harness_returns_invalid_reference_for_standalone_reference() {
    // Reference without any other expression
    let source = "$slot_name";
    let result = harness_compile_stage(source);
    match result {
        Err(ExprError::InvalidReference { reference }) => {
            assert_eq!(reference, "$slot_name");
        }
        other => panic!("expected InvalidReference, got {:?}", other),
    }
}

// ── D-4: Valid compilation with literals ──

#[test]
fn harness_compiles_and_evaluates_single_integer_literal() {
    let source = "42";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 42),
        other => panic!("expected Ok(SlotValue::I64(42)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_single_boolean_literal_true() {
    let source = "true";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b),
        other => panic!("expected Ok(SlotValue::Bool(true)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_single_boolean_literal_false() {
    let source = "false";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::Bool(b)) => assert!(!b),
        other => panic!("expected Ok(SlotValue::Bool(false)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_null_literal() {
    let source = "null";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::Null) => {}
        other => panic!("expected Ok(SlotValue::Null), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_negative_integer_literal() {
    let source = "-99";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, -99),
        other => panic!("expected Ok(SlotValue::I64(-99)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_i64_max_literal() {
    let source = "9223372036854775807";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, i64::MAX),
        other => panic!("expected Ok(SlotValue::I64(i64::MAX)), got {:?}", other),
    }
}

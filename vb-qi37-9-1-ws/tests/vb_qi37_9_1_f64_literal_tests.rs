#![forbid(unsafe_code)]
//! F64 literal tests for bead vb-qi37.9.1.
//!
//! These tests verify F64 literal lexing, parsing, type tainting, and lowering.
//! They are FAILING-FIRST: the F64 variant does not yet exist in ExpressionLiteral,
//! TokenKind::Float does not exist in the lexer, expression_literal_fact has no F64 arm,
//! and lower_literal has no F64 arm.
//!
//! These tests are in the workspace tests directory and test through the public API
//! of vb_compile and vb_core.

use vb_compile::{parse_expression, YamlCompiler};
use vb_core::{CompiledWorkflow, ConstValue, SlotIdx, StepIdx, CompiledNodeKind, FiniteF64};

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, String> {
    YamlCompiler::default()
        .compile(source)
        .map_err(|e| format!("compile unexpectedly failed: {e}"))
}

fn finish_literal_source(value: &str) -> Vec<u8> {
    format!(
        "version: velvet-ballastics/v1\nname: finish_case\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: {value}\n"
    )
    .into_bytes()
}

fn ensure_compile_and_parse_ok(source: &[u8]) -> Result<(), String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(_) => Ok(()),
        Err(errors) => Err(format!("parse_ast unexpectedly failed: {errors}")),
    }
}

fn ensure_finish_const_value(source: &[u8], expected: ConstValue) -> Result<(), String> {
    let workflow = compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::new(0))
        .ok_or_else(|| "compiled workflow did not contain step 0".to_owned())?;
    match &node.kind {
        CompiledNodeKind::SetConst { value }
            if node.output == Some(SlotIdx::new(0)) && node.next == Some(StepIdx::new(1)) =>
        {
            workflow
                .constant(*value)
                .copied()
                .ok_or_else(|| format!("finish literal referenced missing constant {value:?}"))
                .and_then(|actual| {
                    if actual == expected {
                        Ok(())
                    } else {
                        Err(format!(
                            "finish const mismatch: expected {expected:?}, got {actual:?}"
                        ))
                    }
                })
        }
        kind => Err(format!(
            "finish did not lower to SetConst -> Finish: {kind:?}"
        )),
    }
}

// ---------------------------------------------------------------------------
// F64 Literal Parsing Tests - These test that decimal literals parse as F64
// ---------------------------------------------------------------------------

#[test]
fn parse_expression_accepts_simple_positive_f64_literal() -> Result<(), String> {
    let result = vb_compile::parse_expression("3.14159");
    result.map_err(|e| format!("parse_expression rejected valid F64 literal: {e}"))
}

#[test]
fn parse_expression_accepts_negative_f64_literal() -> Result<(), String> {
    let result = vb_compile::parse_expression("-2.71828");
    result.map_err(|e| format!("parse_expression rejected valid negative F64 literal: {e}"))
}

#[test]
fn parse_expression_accepts_f64_literal_with_leading_zero() -> Result<(), String> {
    let result = vb_compile::parse_expression("0.5");
    result.map_err(|e| format!("parse_expression rejected valid F64 with leading zero: {e}"))
}

#[test]
fn parse_expression_accepts_f64_literal_with_exponent() -> Result<(), String> {
    let result = vb_compile::parse_expression("1e10");
    result.map_err(|e| format!("parse_expression rejected valid F64 with exponent: {e}"))
}

#[test]
fn parse_expression_accepts_f64_literal_with_negative_exponent() -> Result<(), String> {
    let result = vb_compile::parse_expression("1.5e-3");
    result.map_err(|e| format!("parse_expression rejected valid F64 with negative exponent: {e}"))
}

#[test]
fn parse_expression_rejects_integer_literal_as_f64() -> Result<(), String> {
    let result = vb_compile::parse_expression("42");
    result.map_err(|e| format!("parse_expression rejected integer: {e}"))?;
    let expr = result;
    match expr {
        vb_compile::ParsedExpression::Literal(vb_compile::ExpressionLiteral::I64(_)) => Ok(()),
        other => Err(format!(
            "expected I64 literal, got {:?}",
            other
        )),
    }
}

#[test]
fn parse_expression_produces_expression_literal_f64_variant() -> Result<(), String> {
    let result = vb_compile::parse_expression("3.14");
    let expr = result.map_err(|e| format!("parse_expression rejected valid F64: {e}"))?;
    match expr {
        vb_compile::ParsedExpression::Literal(vb_compile::ExpressionLiteral::F64(_)) => Ok(()),
        other => Err(format!(
            "expected ExpressionLiteral::F64, got {:?}",
            other
        )),
    }
}

#[test]
fn parse_expression_f64_preserves_value() -> Result<(), String> {
    let result = vb_compile::parse_expression("2.71828");
    let expr = result.map_err(|e| format!("parse_expression rejected valid F64: {e}"))?;
    match expr {
        vb_compile::ParsedExpression::Literal(vb_compile::ExpressionLiteral::F64(val)) => {
            if (*val - 2.71828).abs() < 1e-10 {
                Ok(())
            } else {
                Err(format!("F64 value mismatch: expected ~2.71828, got {val}"))
            }
        }
        other => Err(format!("expected ExpressionLiteral::F64, got {:?}", other)),
    }
}

// ---------------------------------------------------------------------------
// F64 Literal Workflow Compilation Tests
// ---------------------------------------------------------------------------

#[test]
fn compile_and_parse_accept_f64_finish_literal_positive() -> Result<(), String> {
    let source = finish_literal_source("3.14159");
    ensure_compile_and_parse_ok(&source)?;
    let f64_expected = ConstValue::F64(FiniteF64::new(3.14159).map_err(|e| e.to_string())?);
    ensure_finish_const_value(&source, f64_expected)
}

#[test]
fn compile_and_parse_accept_f64_finish_literal_negative() -> Result<(), String> {
    let source = finish_literal_source("-2.71828");
    ensure_compile_and_parse_ok(&source)?;
    let f64_expected = ConstValue::F64(FiniteF64::new(-2.71828).map_err(|e| e.to_string())?);
    ensure_finish_const_value(&source, f64_expected)
}

#[test]
fn compile_and_parse_accept_f64_finish_literal_zero() -> Result<(), String> {
    let source = finish_literal_source("0.0");
    ensure_compile_and_parse_ok(&source)?;
    let f64_expected = ConstValue::F64(FiniteF64::new(0.0).map_err(|e| e.to_string())?);
    ensure_finish_const_value(&source, f64_expected)
}

#[test]
fn compile_and_parse_accept_f64_finish_literal_exponent() -> Result<(), String> {
    let source = finish_literal_source("1e5");
    ensure_compile_and_parse_ok(&source)?;
    let f64_expected = ConstValue::F64(FiniteF64::new(1e5).map_err(|e| e.to_string())?);
    ensure_finish_const_value(&source, f64_expected)
}

// ---------------------------------------------------------------------------
// F64 Lowering Tests
// ---------------------------------------------------------------------------

#[test]
fn lower_literal_handles_f64_positive() -> Result<(), String> {
    use vb_compile::expression::{ExpressionLiteral, ParsedExpression};
    use vb_compile::expression_bytecode::compile_expr_to_bytecode;

    let expr = ParsedExpression::Literal(ExpressionLiteral::F64(3.14159));
    let mut constants = Vec::new();
    let result = compile_expr_to_bytecode(&expr, &mut constants);
    result.map_err(|e| format!("compile_expr_to_bytecode rejected F64: {e}"))?;

    if constants.is_empty() {
        return Err("no constants emitted for F64 literal".to_owned());
    }
    match constants.first() {
        Some(ConstValue::F64(val)) if (*val - 3.14159).abs() < 1e-10 => Ok(()),
        Some(other) => Err(format!("expected ConstValue::F64(3.14159), got {:?}", other)),
        None => Err("constants table empty after lowering F64".to_owned()),
    }
}

#[test]
fn lower_literal_handles_f64_negative() -> Result<(), String> {
    use vb_compile::expression::{ExpressionLiteral, ParsedExpression};
    use vb_compile::expression_bytecode::compile_expr_to_bytecode;

    let expr = ParsedExpression::Literal(ExpressionLiteral::F64(-2.71828));
    let mut constants = Vec::new();
    let result = compile_expr_to_bytecode(&expr, &mut constants);
    result.map_err(|e| format!("compile_expr_to_bytecode rejected negative F64: {e}"))?;

    match constants.first() {
        Some(ConstValue::F64(val)) if (*val - (-2.71828)).abs() < 1e-10 => Ok(()),
        Some(other) => Err(format!("expected ConstValue::F64(-2.71828), got {:?}", other)),
        None => Err("constants table empty after lowering F64".to_owned()),
    }
}

#[test]
fn lower_literal_handles_f64_zero() -> Result<(), String> {
    use vb_compile::expression::{ExpressionLiteral, ParsedExpression};
    use vb_compile::expression_bytecode::compile_expr_to_bytecode;

    let expr = ParsedExpression::Literal(ExpressionLiteral::F64(0.0));
    let mut constants = Vec::new();
    let result = compile_expr_to_bytecode(&expr, &mut constants);
    result.map_err(|e| format!("compile_expr_to_bytecode rejected F64 zero: {e}"))?;

    match constants.first() {
        Some(ConstValue::F64(val)) if *val == FiniteF64::new(0.0).map_err(|e| e.to_string())? => Ok(()),
        Some(other) => Err(format!("expected ConstValue::F64(0.0), got {:?}", other)),
        None => Err("constants table empty after lowering F64 zero".to_owned()),
    }
}

// ---------------------------------------------------------------------------
// TokenKind::Float Lexing Tests
// ---------------------------------------------------------------------------

#[test]
fn lexer_produces_token_kind_float_for_decimal() -> Result<(), String> {
    let result = vb_compile::parse_expression("1.5");
    let expr = result.map_err(|e| format!("parse_expression rejected decimal: {e}"))?;
    match expr {
        vb_compile::ParsedExpression::Literal(vb_compile::ExpressionLiteral::F64(_)) => Ok(()),
        other => Err(format!(
            "expected ExpressionLiteral::F64 for '1.5', got {:?}",
            other
        )),
    }
}

#[test]
fn lexer_distinguishes_integer_from_float() -> Result<(), String> {
    let int_result = vb_compile::parse_expression("42");
    let float_result = vb_compile::parse_expression("42.0");

    let int_expr = int_result.map_err(|e| format!("parse rejected integer: {e}"))?;
    let float_expr = float_result.map_err(|e| format!("parse rejected float: {e}"))?;

    match (&int_expr, &float_expr) {
        (
            vb_compile::ParsedExpression::Literal(vb_compile::ExpressionLiteral::I64(_)),
            vb_compile::ParsedExpression::Literal(vb_compile::ExpressionLiteral::F64(_)),
        ) => Ok(()),
        (int, float) => Err(format!(
            "integer vs float distinction failed: int={:?} float={:?}",
            int, float
        )),
    }
}

// ---------------------------------------------------------------------------
// F64 Variant Exhaustiveness Tests - These will fail to compile until F64 exists
// ---------------------------------------------------------------------------

#[test]
fn expression_literal_enum_has_f64_variant() -> Result<(), String> {
    use vb_compile::ExpressionLiteral;

    let _val: ExpressionLiteral = ExpressionLiteral::F64(1.0);
    Ok(())
}

#[test]
fn const_value_enum_has_f64_variant() -> Result<(), String> {
    use vb_core::ConstValue;

    let _val: ConstValue = ConstValue::F64(FiniteF64::new(1.0).map_err(|e| e.to_string())?);
    Ok(())
}

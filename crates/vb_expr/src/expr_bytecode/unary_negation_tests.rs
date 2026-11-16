#![forbid(unsafe_code)]

use super::compile_expr;
use crate::eval::{eval_expr_program, eval_unary_op};
use crate::lexer::UnaryOp;
use crate::{ExprError, ExprResult};
use vb_core::value::FiniteF64;
use vb_core::{SlotIdx, SlotValue};

const SIGN: u64 = 0x8000_0000_0000_0000;

fn evaluate(source: &str, input: SlotValue) -> ExprResult<SlotValue> {
    let resolver = |reference: &str| (reference == "$x").then_some(SlotIdx::new(0));
    let (program, constants) = compile_expr(source, &resolver)?;
    eval_expr_program(&program, &[Some(input)], &constants)
}

fn float_bits(value: SlotValue) -> Result<u64, String> {
    match value {
        SlotValue::F64(value) => Ok(value.get().to_bits()),
        other => Err(format!("expected F64, found {other:?}")),
    }
}

#[test]
fn unary_negation_literal_zero_bits() -> Result<(), String> {
    for (source, expected) in [("-0.0", SIGN), ("--0.0", 0), ("-(-0.0)", 0)] {
        let value = evaluate(source, SlotValue::Null).map_err(|error| error.to_string())?;
        assert_eq!(float_bits(value)?, expected, "{source}");
    }
    Ok(())
}

#[test]
fn unary_negation_reference_float_bits() -> Result<(), String> {
    for bits in [
        0,
        SIGN,
        1,
        0x8000_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
        0x3ff0_0000_0000_0000,
        0xbff0_0000_0000_0000,
    ] {
        let input = SlotValue::F64(
            FiniteF64::new(f64::from_bits(bits)).map_err(|error| error.to_string())?,
        );
        let ast_once = eval_unary_op(UnaryOp::Neg, input).map_err(|error| error.to_string())?;
        let ast_twice = eval_unary_op(UnaryOp::Neg, ast_once).map_err(|error| error.to_string())?;
        assert_eq!(float_bits(ast_once)?, bits ^ SIGN);
        assert_eq!(float_bits(ast_twice)?, bits);
        for (source, expected) in [("-$x", bits ^ SIGN), ("--$x", bits)] {
            let value = evaluate(source, input).map_err(|error| error.to_string())?;
            assert_eq!(float_bits(value)?, expected, "{source}, input {bits:#018x}");
        }
    }
    Ok(())
}

#[test]
fn unary_negation_integer_values_and_overflow() -> Result<(), String> {
    for (input, expected) in [
        (0, 0),
        (1, -1),
        (-1, 1),
        (i64::MAX, -i64::MAX),
        (-i64::MAX, i64::MAX),
    ] {
        assert_eq!(
            evaluate("-$x", SlotValue::I64(input)),
            Ok(SlotValue::I64(expected))
        );
        assert_eq!(
            evaluate("--$x", SlotValue::I64(input)),
            Ok(SlotValue::I64(input))
        );
        assert_eq!(
            eval_unary_op(UnaryOp::Neg, SlotValue::I64(input)),
            Ok(SlotValue::I64(expected))
        );
    }
    for source in ["-$x", "--$x"] {
        assert_eq!(
            evaluate(source, SlotValue::I64(i64::MIN)),
            Err(ExprError::IntegerOverflow)
        );
    }
    assert_eq!(
        eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN)),
        Err(ExprError::IntegerOverflow)
    );
    Ok(())
}

#[test]
fn unary_negation_rejects_nonnumeric_operand() -> Result<(), String> {
    let expected = ExprError::TypeMismatch {
        expected: "number".into(),
        found: "boolean".into(),
    };
    assert_eq!(
        evaluate("-$x", SlotValue::Bool(true)),
        Err(expected.clone())
    );
    assert_eq!(
        eval_unary_op(UnaryOp::Neg, SlotValue::Bool(true)),
        Err(expected)
    );
    Ok(())
}

#![forbid(unsafe_code)]

//! Property tests for scoped Reduce lowering boundaries (vb-eirmk.4.4.2).
//!
//! - `parse_typed_initial_roundtrip_prop`: every `i64` round-trips as
//!   `I64`; non-literal, overflow, and non-finite texts yield the canonical
//!   `aggregate.initial` rejection (field + expected text pinned).
//! - `checked_width_n_is_exact_prop`: width fits `u16` iff it is at most
//!   `u16::MAX`; oversize widths fail with the exact offending value.

use super::part_04::checked_width_n;
use super::part_04::parse_typed_initial;
use crate::{CompileError, CompileErrors};
use proptest::prelude::*;
use vb_core::ConstValue;

fn initial_shape_rejection(errors: &CompileErrors) -> bool {
    errors.0.iter().any(|e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "aggregate.initial",
                expected: "a literal constant: null, boolean, integer, or finite float",
                ..
            }
        )
    })
}

proptest! {
    #[test]
    fn parse_typed_initial_roundtrip_prop(value in any::<i64>()) {
        let text = value.to_string();
        let parsed = parse_typed_initial(&text, 0);
        prop_assert_eq!(parsed, Ok((ConstValue::I64(value), vb_expr::expr_typecheck::ExprType::I64)));
    }

    #[test]
    fn parse_typed_initial_boundary_prop(
        text in prop_oneof![
            Just("9223372036854775807".to_owned()),
            Just("-9223372036854775808".to_owned()),
            Just("9223372036854775808".to_owned()),
            Just("-9223372036854775809".to_owned()),
            Just("inf".to_owned()),
            Just("NaN".to_owned()),
            Just("-inf".to_owned()),
            Just("1e999".to_owned()),
            Just("$total".to_owned()),
            Just("1 + 2".to_owned()),
            Just("sum($total)".to_owned()),
        ]
    ) {
        let parsed = parse_typed_initial(&text, 0);
        match text.as_str() {
            "9223372036854775807" => prop_assert_eq!(
                parsed,
                Ok((ConstValue::I64(i64::MAX), vb_expr::expr_typecheck::ExprType::I64))
            ),
            "-9223372036854775808" => prop_assert_eq!(
                parsed,
                Ok((ConstValue::I64(i64::MIN), vb_expr::expr_typecheck::ExprType::I64))
            ),
            _ => {
                let errors = parsed.expect_err("overflow, non-finite, and non-literal initials must fail");
                prop_assert!(initial_shape_rejection(&errors), "initial {text} wrong diagnostic: {errors:?}");
            }
        }
    }

    #[test]
    fn checked_width_n_is_exact_prop(width in any::<usize>()) {
        let result = checked_width_n(width);
        let expected = u16::try_from(width).map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: width }]));
        prop_assert_eq!(result, expected);
    }

    #[test]
    fn checked_width_n_shrinks_at_u16_boundary(offset in 0..=4usize) {
        let Some(width) = (u16::MAX as usize).checked_add(offset) else {
            return Err(proptest::test_runner::TestCaseError::reject("width overflow"));
        };
        let result = checked_width_n(width);
        if offset == 0 {
            prop_assert_eq!(result, Ok(u16::MAX));
        } else {
            prop_assert_eq!(
                result,
                Err(CompileErrors(vec![CompileError::StepIndexOutOfRange { value: width }]))
            );
        }
    }
}

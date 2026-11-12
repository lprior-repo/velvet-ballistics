#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn eval_ops_with_constants(
        ops: &[ExprOp],
        constants: Vec<ConstValue>,
    ) -> Result<SlotValue, String> {
        let signal = super::drive_numeric_expr(ops.to_vec(), constants, vec![])
            .map_err(|e| e.to_string())?;
        match signal {
            crate::EngineSignal::Finished(value, _) => Ok(value),
            other => Err(format!("unexpected signal: {other:?}")),
        }
    }

    fn mixed_property_constants(a: i32, b: i32) -> Result<Vec<ConstValue>, TestCaseError> {
        Ok(vec![
            ConstValue::I64(i64::from(a)),
            ConstValue::F64(
                FiniteF64::new(f64::from(b)).map_err(|e| TestCaseError::fail(e.to_string()))?,
            ),
        ])
    }

    proptest! {
        #[test]
        fn add_then_sub_roundtrips_to_original(
            a in -1_000_000i64..1_000_000,
            b in -1_000_000i64..1_000_000,
        ) {
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::I64(a), ConstValue::I64(b)],
            );
            prop_assert_eq!(result, Ok(SlotValue::I64(a)));
        }

        #[test]
        fn checked_add_property_matches_independent_oracle(
            a in -1000i32..1000,
            b in -1000i32..1000,
        ) {
            let expected = a.checked_add(b).ok_or_else(|| TestCaseError::fail("bounded sum overflowed"))?;
            let ops = vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ];
            let result = eval_ops_with_constants(
                &ops,
                vec![ConstValue::I64(i64::from(a)), ConstValue::I64(i64::from(b))],
            );
            prop_assert_eq!(result, Ok(SlotValue::I64(i64::from(expected))));
        }

        #[test]
        fn mixed_add_property_matches_independent_f64_oracle(
            a in -1000i32..1000,
            b in -1000i32..1000,
        ) {
            let expected = f64::from(a.checked_add(b).ok_or_else(|| TestCaseError::fail("bounded sum overflowed"))?);
            let ops = super::binary_const_ops(ExprOp::Add);
            let result = eval_ops_with_constants(
                &ops,
                mixed_property_constants(a, b)?,
            );
            match result {
                Ok(SlotValue::F64(v)) => {
                    prop_assert_eq!(v.get(), expected);
                }
                other => prop_assert!(false, "unexpected: {other:?}"),
            }
        }

        #[test]
        fn mixed_lt_property_matches_independent_comparison(
            a in -1000i32..1000,
            b in -1000i32..1000,
        ) {
            let expected = f64::from(a) < f64::from(b);
            let ops = super::binary_const_ops(ExprOp::Lt);
            let result = eval_ops_with_constants(
                &ops,
                mixed_property_constants(a, b)?,
            );
            prop_assert_eq!(result, Ok(SlotValue::Bool(expected)));
        }
    }
}

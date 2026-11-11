const ROUNDING_CASES: [(&str, i64, f64); 4] = [
    ("9007199254740993", 9007199254740993_i64, 9007199254740992.0),
    ("9007199254740995", 9007199254740995_i64, 9007199254740996.0),
    ("i64_max", i64::MAX, 9223372036854775808.0),
    ("i64_min", i64::MIN, -9223372036854775808.0),
];

fn check_promotion_case((name, i64_value, expected_f64): (&str, i64, f64)) -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Add),
        vec![ConstValue::I64(i64_value), f64_const(0.0)?],
        vec![],
    )
    .map_err(|e| format!("{name}: {e}"))?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::F64(result), Taint::Clean) => {
            if result.get() != expected_f64 {
                return Err(format!(
                    "{name}: expected {expected_f64}, got {}",
                    result.get()
                ));
            }
        }
        other => {
            return Err(format!("{name}: unexpected {other:?}"));
        }
    }
    Ok(())
}

fn check_rounded_gt(two_53: &ConstValue) -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Gt),
        vec![ConstValue::I64(9007199254740993), two_53.clone()],
        vec![],
    )
    .map_err(|e| format!("gt_2to53: {e}"))?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::Bool(false), Taint::Clean) => {}
        other => {
            return Err(format!("gt_2to53: expected false, got {other:?}"));
        }
    }
    Ok(())
}

fn check_rounded_gte(two_53: ConstValue) -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Gte),
        vec![ConstValue::I64(9007199254740993), two_53],
        vec![],
    )
    .map_err(|e| format!("gte_2to53: {e}"))?;
    match signal {
        crate::EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean) => {}
        other => {
            return Err(format!("gte_2to53: expected true, got {other:?}"));
        }
    }
    Ok(())
}

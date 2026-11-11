const COMPARISON_CASES: [ComparisonCase; 12] = [
    // (name, op, left, right, expected)
    // Gt: 6>2 true, 2>6 false, 2>2 false
    (
        "gt_6_2",
        ExprOp::Gt,
        SlotValue::I64(6),
        SlotValue::I64(2),
        true,
    ),
    (
        "gt_2_6",
        ExprOp::Gt,
        SlotValue::I64(2),
        SlotValue::I64(6),
        false,
    ),
    (
        "gt_2_2",
        ExprOp::Gt,
        SlotValue::I64(2),
        SlotValue::I64(2),
        false,
    ),
    // Gte: 6>=2 true, 2>=6 false, 2>=2 true
    (
        "gte_6_2",
        ExprOp::Gte,
        SlotValue::I64(6),
        SlotValue::I64(2),
        true,
    ),
    (
        "gte_2_6",
        ExprOp::Gte,
        SlotValue::I64(2),
        SlotValue::I64(6),
        false,
    ),
    (
        "gte_2_2",
        ExprOp::Gte,
        SlotValue::I64(2),
        SlotValue::I64(2),
        true,
    ),
    // Lt: 6<2 false, 2<6 true, 2<2 false
    (
        "lt_6_2",
        ExprOp::Lt,
        SlotValue::I64(6),
        SlotValue::I64(2),
        false,
    ),
    (
        "lt_2_6",
        ExprOp::Lt,
        SlotValue::I64(2),
        SlotValue::I64(6),
        true,
    ),
    (
        "lt_2_2",
        ExprOp::Lt,
        SlotValue::I64(2),
        SlotValue::I64(2),
        false,
    ),
    // Lte: 6<=2 false, 2<=6 true, 2<=2 true
    (
        "lte_6_2",
        ExprOp::Lte,
        SlotValue::I64(6),
        SlotValue::I64(2),
        false,
    ),
    (
        "lte_2_6",
        ExprOp::Lte,
        SlotValue::I64(2),
        SlotValue::I64(6),
        true,
    ),
    (
        "lte_2_2",
        ExprOp::Lte,
        SlotValue::I64(2),
        SlotValue::I64(2),
        true,
    ),
];

const COMPARISON_PAIRINGS: [ComparisonPairing; 4] = [
    ("i64_i64", true, true),
    ("i64_f64", true, false),
    ("f64_i64", false, true),
    ("f64_f64", false, false),
];

fn comparison_constants(
    (_, _, left, right, _): ComparisonCase,
    (_, left_is_i64, right_is_i64): ComparisonPairing,
) -> Result<Vec<ConstValue>, String> {
    let left_val = if left_is_i64 {
        to_const(left)?
    } else {
        f64_const(left_as_f64(&left)?)?
    };
    let right_val = if right_is_i64 {
        to_const(right)?
    } else {
        f64_const(right_as_f64(&right)?)?
    };
    Ok(vec![left_val, right_val])
}

fn check_comparison_case(
    case: ComparisonCase,
    configuration: ComparisonPairing,
) -> Result<(), String> {
    let (name, op, _, _, expected) = case;
    let (pairing, _, _) = configuration;
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(op),
        comparison_constants(case, configuration)?,
        vec![],
    )
    .map_err(|e| format!("{name}_{pairing}: {e}"))?;
    assert_comparison_signal(signal, name, pairing, expected)
}

fn assert_comparison_signal(
    signal: crate::EngineSignal,
    name: &str,
    pairing: &str,
    expected: bool,
) -> Result<(), String> {
    match signal {
        crate::EngineSignal::Finished(SlotValue::Bool(result), Taint::Clean) => {
            if result != expected {
                return Err(format!(
                    "{name}_{pairing}: expected {expected}, got {result}"
                ));
            }
        }
        other => {
            return Err(format!("{name}_{pairing}: unexpected {other:?}"));
        }
    }
    Ok(())
}

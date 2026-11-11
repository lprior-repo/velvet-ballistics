type NumericTaintCase = (&'static str, ExprOp, SlotValue, f64);
type TaintPairing = (&'static str, SlotValue, SlotValue, bool);
type TaintSide = (&'static str, Taint, Taint);

const NUMERIC_TAINT_CASES: [NumericTaintCase; 8] = [
    ("add", ExprOp::Add, SlotValue::I64(8), 8.0),
    ("sub", ExprOp::Sub, SlotValue::I64(4), 4.0),
    ("mul", ExprOp::Mul, SlotValue::I64(12), 12.0),
    ("div", ExprOp::Div, SlotValue::I64(3), 3.0),
    ("gt", ExprOp::Gt, SlotValue::Bool(true), 0.0),
    ("gte", ExprOp::Gte, SlotValue::Bool(true), 0.0),
    ("lt", ExprOp::Lt, SlotValue::Bool(false), 0.0),
    ("lte", ExprOp::Lte, SlotValue::Bool(false), 0.0),
];

const TAINT_SIDES: [TaintSide; 2] = [
    ("left", Taint::Secret, Taint::Clean),
    ("right", Taint::Clean, Taint::Secret),
];

fn taint_type_pairings() -> Result<[TaintPairing; 4], String> {
    Ok([
        ("i64_i64", SlotValue::I64(6), SlotValue::I64(2), true),
        ("i64_f64", SlotValue::I64(6), slot_f64(2.0)?, false),
        ("f64_i64", slot_f64(6.0)?, SlotValue::I64(2), false),
        ("f64_f64", slot_f64(6.0)?, slot_f64(2.0)?, false),
    ])
}

fn check_taint_case(
    (op_name, op, expected_i64, expected_f64): NumericTaintCase,
    (pairing, left, right, result_is_i64): TaintPairing,
    (secret_side, left_taint, right_taint): TaintSide,
) -> Result<(), String> {
    let expected = if result_is_i64 || matches!(expected_i64, SlotValue::Bool(_)) {
        expected_i64
    } else {
        slot_f64(expected_f64)?
    };
    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadSlot(crate::ids::SlotIdx::new(0)),
            ExprOp::LoadSlot(crate::ids::SlotIdx::new(1)),
            op,
        ],
        vec![],
        vec![(left, left_taint), (right, right_taint)],
    )
    .map_err(|e| format!("{op_name}_{pairing}_{secret_side}: {e}"))?;
    assert_taint_signal(signal, expected, op_name, pairing, secret_side)
}

fn assert_taint_signal(
    signal: crate::EngineSignal,
    expected: SlotValue,
    op_name: &str,
    pairing: &str,
    secret_side: &str,
) -> Result<(), String> {
    match signal {
        crate::EngineSignal::Finished(value, Taint::Secret) => {
            if value != expected {
                return Err(format!(
                    "{op_name}_{pairing}_{secret_side}: expected {expected:?}, got {value:?}"
                ));
            }
        }
        other => {
            return Err(format!(
                "{op_name}_{pairing}_{secret_side}: unexpected {other:?}"
            ));
        }
    }
    Ok(())
}

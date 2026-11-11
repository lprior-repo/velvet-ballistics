// Numeric operator boundary tests: comparisons, typed errors, promotion, taint.
// Uses validated EvalExpr->Finish drive through real workflow.

use crate::errors::EngineError;
use crate::ids::ConstIdx;
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::workflow::ExprOp;

use super::f64_const;

fn slot_f64(value: f64) -> Result<SlotValue, String> {
    FiniteF64::new(value)
        .map(SlotValue::F64)
        .map_err(|error| error.to_string())
}

fn to_const(value: SlotValue) -> Result<ConstValue, String> {
    match value {
        SlotValue::I64(n) => Ok(ConstValue::I64(n)),
        SlotValue::F64(f) => Ok(ConstValue::F64(f)),
        other => Err(format!("expected numeric fixture, got {other:?}")),
    }
}

#[test]
fn comparison_outcomes_all_pairings() -> Result<(), String> {
    let cases = [
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

    let pairings = [
        ("i64_i64", true, true),
        ("i64_f64", true, false),
        ("f64_i64", false, true),
        ("f64_f64", false, false),
    ];

    for (name, op, left, right, expected) in cases {
        for (pairing, left_is_i64, right_is_i64) in pairings {
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
            let signal = super::drive_numeric_expr(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    op,
                ],
                vec![left_val, right_val],
                vec![],
            )
            .map_err(|e| format!("{name}_{pairing}: {e}"))?;
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
        }
    }
    Ok(())
}

fn left_as_f64(value: &SlotValue) -> Result<f64, String> {
    match value {
        SlotValue::I64(n) => {
            let n_i32 = i32::try_from(*n).map_err(|_| "i64 too large for test")?;
            Ok(f64::from(n_i32))
        }
        SlotValue::F64(f) => Ok(f.get()),
        other => Err(format!("unexpected {other:?}")),
    }
}

fn right_as_f64(value: &SlotValue) -> Result<f64, String> {
    match value {
        SlotValue::I64(n) => {
            let n_i32 = i32::try_from(*n).map_err(|_| "i64 too large for test")?;
            Ok(f64::from(n_i32))
        }
        SlotValue::F64(f) => Ok(f.get()),
        other => Err(format!("unexpected {other:?}")),
    }
}

#[test]
fn exact_typed_errors() -> Result<(), String> {
    // Integer arithmetic overflow
    for (name, op, left, right) in [
        ("add", ExprOp::Add, i64::MAX, 1),
        ("sub", ExprOp::Sub, i64::MIN, 1),
        ("mul", ExprOp::Mul, i64::MAX, 2),
    ] {
        let signal = super::drive_numeric_expr(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                op,
            ],
            vec![ConstValue::I64(left), ConstValue::I64(right)],
            vec![],
        );
        match signal {
            Err(EngineError::InvalidCompiledWorkflow { reason }) => {
                if reason != "integer arithmetic overflow" {
                    return Err(format!("{name}: got reason '{reason}'"));
                }
            }
            other => {
                return Err(format!("{name}: expected overflow, got {other:?}"));
            }
        }
    }

    // Integer division overflow
    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)],
        vec![],
    );
    match signal {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            if reason != "integer division overflow" {
                return Err(format!("div: got reason '{reason}'"));
            }
        }
        other => {
            return Err(format!("div: expected overflow, got {other:?}"));
        }
    }

    // Bool type mismatch on all 8 numeric ops, both operand positions
    let numeric_ops = [
        ExprOp::Add,
        ExprOp::Sub,
        ExprOp::Mul,
        ExprOp::Div,
        ExprOp::Gt,
        ExprOp::Gte,
        ExprOp::Lt,
        ExprOp::Lte,
    ];
    for op in numeric_ops {
        for (name, left, right) in [
            ("left", ConstValue::Bool(true), ConstValue::I64(5)),
            ("right", ConstValue::I64(5), ConstValue::Bool(true)),
        ] {
            let signal = super::drive_numeric_expr(
                vec![
                    ExprOp::LoadConst(ConstIdx::new(0)),
                    ExprOp::LoadConst(ConstIdx::new(1)),
                    op,
                ],
                vec![left, right],
                vec![],
            );
            match signal {
                Err(EngineError::TypeMismatch { expected, found }) => {
                    if expected != "number" || found != "boolean" {
                        return Err(format!("{name}: TypeMismatch({expected},{found})"));
                    }
                }
                other => {
                    return Err(format!("{name}: expected TypeMismatch, got {other:?}"));
                }
            }
        }
    }

    // Integer division by zero
    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ],
        vec![ConstValue::I64(10), ConstValue::I64(0)],
        vec![],
    );
    match signal {
        Err(EngineError::DivisionByZero) => {}
        other => {
            return Err(format!("div_zero: expected DivisionByZero, got {other:?}"));
        }
    }

    // Mixed/F64 division by zero -> NonFiniteNumber
    let f64_zero = f64_const(0.0)?;
    for (name, left, right) in [
        ("f64_div_i64_zero", f64_const(1.0)?, ConstValue::I64(0)),
        ("i64_div_f64_zero", ConstValue::I64(1), f64_const(0.0)?),
        ("f64_div_f64_zero", f64_const(1.0)?, f64_zero),
    ] {
        let signal = super::drive_numeric_expr(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![left, right],
            vec![],
        );
        match signal {
            Err(EngineError::NonFiniteNumber) => {}
            other => {
                return Err(format!("{name}: expected NonFiniteNumber, got {other:?}"));
            }
        }
    }

    // f64 MAX+MAX -> NonFiniteNumber
    let f64_max = f64_const(f64::MAX)?;
    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ],
        vec![f64_max.clone(), f64_max.clone()],
        vec![],
    );
    match signal {
        Err(EngineError::NonFiniteNumber) => {}
        other => {
            return Err(format!(
                "f64_max_add: expected NonFiniteNumber, got {other:?}"
            ));
        }
    }

    // f64 MAX*2 -> NonFiniteNumber
    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Mul,
        ],
        vec![f64_max, f64_const(2.0)?],
        vec![],
    );
    match signal {
        Err(EngineError::NonFiniteNumber) => {}
        other => {
            return Err(format!(
                "f64_max_mul: expected NonFiniteNumber, got {other:?}"
            ));
        }
    }

    Ok(())
}

#[test]
fn exact_i64_promotion_rounding() -> Result<(), String> {
    // 2^53+1 rounds down, 2^53+3 rounds up, MAX/MIN promote exactly
    let cases = [
        ("9007199254740993", 9007199254740993_i64, 9007199254740992.0),
        ("9007199254740995", 9007199254740995_i64, 9007199254740996.0),
        ("i64_max", i64::MAX, 9223372036854775808.0),
        ("i64_min", i64::MIN, -9223372036854775808.0),
    ];

    for (name, i64_value, expected_f64) in cases {
        let signal = super::drive_numeric_expr(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
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
    }

    // Promoted 2^53+1 vs F64(2^53): Gt false, Gte true
    let two_53 = f64_const(9007199254740992.0)?;
    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Gt,
        ],
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

    let signal = super::drive_numeric_expr(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Gte,
        ],
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

#[test]
fn taint_all_ops_pairings() -> Result<(), String> {
    let numeric_ops = [
        ("add", ExprOp::Add, SlotValue::I64(8), 8.0),
        ("sub", ExprOp::Sub, SlotValue::I64(4), 4.0),
        ("mul", ExprOp::Mul, SlotValue::I64(12), 12.0),
        ("div", ExprOp::Div, SlotValue::I64(3), 3.0),
        ("gt", ExprOp::Gt, SlotValue::Bool(true), 0.0),
        ("gte", ExprOp::Gte, SlotValue::Bool(true), 0.0),
        ("lt", ExprOp::Lt, SlotValue::Bool(false), 0.0),
        ("lte", ExprOp::Lte, SlotValue::Bool(false), 0.0),
    ];

    let pairings = [
        ("i64_i64", SlotValue::I64(6), SlotValue::I64(2), true),
        ("i64_f64", SlotValue::I64(6), slot_f64(2.0)?, false),
        ("f64_i64", slot_f64(6.0)?, SlotValue::I64(2), false),
        ("f64_f64", slot_f64(6.0)?, slot_f64(2.0)?, false),
    ];

    for (op_name, op, expected_i64, expected_f64) in numeric_ops {
        for (pairing, left, right, result_is_i64) in pairings {
            for (secret_side, left_taint, right_taint) in [
                ("left", Taint::Secret, Taint::Clean),
                ("right", Taint::Clean, Taint::Secret),
            ] {
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
            }
        }
    }
    Ok(())
}

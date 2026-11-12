fn check_arithmetic_overflows() -> Result<(), String> {
    for (name, op, left, right) in [
        ("add", ExprOp::Add, i64::MAX, 1),
        ("sub", ExprOp::Sub, i64::MIN, 1),
        ("mul", ExprOp::Mul, i64::MAX, 2),
    ] {
        let signal = super::drive_numeric_expr(
            super::binary_const_ops(op),
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
    Ok(())
}

fn check_division_overflow() -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Div),
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
    Ok(())
}

const MISMATCH_OPS: [ExprOp; 8] = [
    ExprOp::Add,
    ExprOp::Sub,
    ExprOp::Mul,
    ExprOp::Div,
    ExprOp::Gt,
    ExprOp::Gte,
    ExprOp::Lt,
    ExprOp::Lte,
];
const MISMATCH_POSITIONS: [(&str, ConstValue, ConstValue); 2] = [
    ("left", ConstValue::Bool(true), ConstValue::I64(5)),
    ("right", ConstValue::I64(5), ConstValue::Bool(true)),
];

fn check_type_mismatches() -> Result<(), String> {
    for op in MISMATCH_OPS {
        for (name, left, right) in MISMATCH_POSITIONS {
            check_type_mismatch(op, name, left.clone(), right.clone())?;
        }
    }
    Ok(())
}

fn check_type_mismatch(
    op: ExprOp,
    name: &str,
    left: ConstValue,
    right: ConstValue,
) -> Result<(), String> {
    let signal = super::drive_numeric_expr(super::binary_const_ops(op), vec![left, right], vec![]);
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
    Ok(())
}

fn check_integer_division_zero() -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Div),
        vec![ConstValue::I64(10), ConstValue::I64(0)],
        vec![],
    );
    match signal {
        Err(EngineError::DivisionByZero) => {}
        other => {
            return Err(format!("div_zero: expected DivisionByZero, got {other:?}"));
        }
    }
    Ok(())
}

fn float_division_zero_cases() -> Result<[(&'static str, ConstValue, ConstValue); 3], String> {
    let f64_zero = f64_const(0.0)?;
    Ok([
        ("f64_div_i64_zero", f64_const(1.0)?, ConstValue::I64(0)),
        ("i64_div_f64_zero", ConstValue::I64(1), f64_const(0.0)?),
        ("f64_div_f64_zero", f64_const(1.0)?, f64_zero),
    ])
}

fn check_float_division_zero() -> Result<(), String> {
    for (name, left, right) in float_division_zero_cases()? {
        let signal = super::drive_numeric_expr(
            super::binary_const_ops(ExprOp::Div),
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
    Ok(())
}

fn check_f64_max_add(f64_max: &ConstValue) -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Add),
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
    Ok(())
}

fn check_f64_max_mul(f64_max: ConstValue) -> Result<(), String> {
    let signal = super::drive_numeric_expr(
        super::binary_const_ops(ExprOp::Mul),
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

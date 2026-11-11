const ARITHMETIC_SCENARIOS: [fn() -> Result<NumericScenario, String>; 16] = [
    || {
        Ok((
            "add_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(8),
        ))
    },
    || {
        Ok((
            "add_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(8.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "add_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(8.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "add_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Add,
            ],
            vec![f64_const(2.0)?, ConstValue::I64(6)],
            SlotValue::F64(FiniteF64::new(8.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "sub_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(4),
        ))
    },
    || {
        Ok((
            "sub_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(4.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "sub_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(4.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "sub_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Sub,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::F64(FiniteF64::new(4.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "mul_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(12),
        ))
    },
    || {
        Ok((
            "mul_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(12.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "mul_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(12.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "mul_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Mul,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::F64(FiniteF64::new(12.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "div_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::I64(3),
        ))
    },
    || {
        Ok((
            "div_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(3.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "div_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::F64(FiniteF64::new(3.0).map_err(|e| e.to_string())?),
        ))
    },
    || {
        Ok((
            "div_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::F64(FiniteF64::new(3.0).map_err(|e| e.to_string())?),
        ))
    },
];

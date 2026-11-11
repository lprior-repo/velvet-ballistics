const COMPARISON_SCENARIOS: [fn() -> Result<NumericScenario, String>; 16] = [
    || {
        Ok((
            "gt_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![ConstValue::I64(6), ConstValue::I64(2)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gt_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![f64_const(6.0)?, f64_const(2.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gt_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![ConstValue::I64(6), f64_const(2.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gt_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gt,
            ],
            vec![f64_const(6.0)?, ConstValue::I64(2)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gte_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![ConstValue::I64(4), ConstValue::I64(4)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gte_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![f64_const(4.0)?, f64_const(4.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gte_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![ConstValue::I64(4), f64_const(4.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "gte_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Gte,
            ],
            vec![f64_const(4.0)?, ConstValue::I64(4)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lt_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![ConstValue::I64(2), ConstValue::I64(6)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lt_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![f64_const(2.0)?, f64_const(6.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lt_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![ConstValue::I64(2), f64_const(6.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lt_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lt,
            ],
            vec![f64_const(2.0)?, ConstValue::I64(6)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lte_i64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![ConstValue::I64(4), ConstValue::I64(4)],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lte_f64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![f64_const(4.0)?, f64_const(4.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lte_i64_f64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![ConstValue::I64(4), f64_const(4.0)?],
            SlotValue::Bool(true),
        ))
    },
    || {
        Ok((
            "lte_f64_i64",
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Lte,
            ],
            vec![f64_const(4.0)?, ConstValue::I64(4)],
            SlotValue::Bool(true),
        ))
    },
];

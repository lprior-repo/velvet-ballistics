// ── Edge-case: operator precedence boundary conditions ──────────────────

#[test]
fn mul_has_higher_precedence_than_add() -> Result<(), String> {
    // 2 + 3 * 4 => 2, 3, 4, Mul, Add
    let (ops, constants, _max_stack) = lower("2 + 3 * 4")?;
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    // Mul should come before Add
    let mul_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Mul))
        .ok_or("no Mul")?;
    let add_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Add))
        .ok_or("no Add")?;
    adv_ensure(mul_pos < add_pos, "Mul should come before Add in postfix")?;
    Ok(())
}

#[test]
fn div_has_higher_precedence_than_sub() -> Result<(), String> {
    // 10 - 6 / 2 => 10, 6, 2, Div, Sub
    let (ops, _constants, _max_stack) = lower("10 - 6 / 2")?;
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    let div_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Div))
        .ok_or("no Div")?;
    let sub_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Sub))
        .ok_or("no Sub")?;
    adv_ensure(div_pos < sub_pos, "Div should come before Sub in postfix")?;
    Ok(())
}

#[test]
fn and_has_higher_precedence_than_or() -> Result<(), String> {
    // true or false and true => true, false, true, And, Or
    let (ops, _constants, _max_stack) = lower("true or false and true")?;
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    let and_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::And))
        .ok_or("no And")?;
    let or_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Or))
        .ok_or("no Or")?;
    adv_ensure(and_pos < or_pos, "And should come before Or in postfix")?;
    Ok(())
}

#[test]
fn comparison_has_higher_precedence_than_and() -> Result<(), String> {
    // 1 < 2 and 3 > 0 => 1, 2, Lt, 3, 0, Gt, And
    let (ops, _constants, _max_stack) = lower("1 < 2 and 3 > 0")?;
    let lt_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Lt))
        .ok_or("no Lt")?;
    let gt_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Gt))
        .ok_or("no Gt")?;
    let and_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::And))
        .ok_or("no And")?;
    adv_ensure(lt_pos < and_pos, "Lt should come before And")?;
    adv_ensure(gt_pos < and_pos, "Gt should come before And")?;
    Ok(())
}

#[test]
fn equality_has_higher_precedence_than_and() -> Result<(), String> {
    // a == b and c != d => a, b, Eq, c, d, NotEq, And
    let (ops, _constants, _max_stack) = lower("1 == 2 and 3 != 4")?;
    adv_ensure(ops.len() == 7, "should have 7 ops")?;
    let eq_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Eq))
        .ok_or("no Eq")?;
    let noteq_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::NotEq))
        .ok_or("no NotEq")?;
    let and_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::And))
        .ok_or("no And")?;
    adv_ensure(eq_pos < and_pos, "Eq should come before And")?;
    adv_ensure(noteq_pos < and_pos, "NotEq should come before And")?;
    Ok(())
}

#[test]
fn parentheses_override_precedence() -> Result<(), String> {
    // (1 + 2) * 3 => 1, 2, Add, 3, Mul (vs without parens: 1, 2, 3, Mul, Add)
    let (ops, constants, _max_stack) = lower("(1 + 2) * 3")?;
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    let add_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Add))
        .ok_or("no Add")?;
    let mul_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Mul))
        .ok_or("no Mul")?;
    adv_ensure(add_pos < mul_pos, "Add should come before Mul with parens")?;
    Ok(())
}

#[test]
fn nested_parens_override_all_precedence() -> Result<(), String> {
    // ((1 + 2) * (3 - 4)) / 5
    let (ops, constants, _max_stack) = lower("((1 + 2) * (3 - 4)) / 5")?;
    adv_ensure(constants.len() == 5, "should have 5 constants")?;
    // Add, Sub should come before Mul, Mul before Div
    let add_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Add))
        .ok_or("no Add")?;
    let sub_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Sub))
        .ok_or("no Sub")?;
    let mul_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Mul))
        .ok_or("no Mul")?;
    let div_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Div))
        .ok_or("no Div")?;
    adv_ensure(add_pos < mul_pos, "Add should come before Mul")?;
    adv_ensure(sub_pos < mul_pos, "Sub should come before Mul")?;
    adv_ensure(mul_pos < div_pos, "Mul should come before Div")?;
    Ok(())
}

#[test]
fn unary_negation_has_highest_precedence() -> Result<(), String> {
    // -1 + 2 => the negation is applied to 1 first
    let (ops, constants, _max_stack) = lower("-1 + 2")?;
    // Const 0, Const 1, Sub, Const 2, Add
    adv_ensure(
        constants == vec![ConstValue::I64(0), ConstValue::I64(1), ConstValue::I64(2)],
        "negation constants should be 0, 1, 2",
    )?;
    let sub_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Sub))
        .ok_or("no Sub")?;
    let add_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Add))
        .ok_or("no Add")?;
    adv_ensure(sub_pos < add_pos, "Sub (negation) should come before Add")?;
    Ok(())
}

#[test]
fn not_has_higher_precedence_than_comparison() -> Result<(), String> {
    // not true == false => (not true) == false
    let (ops, _constants, _max_stack) = lower("not true == false")?;
    let not_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Not))
        .ok_or("no Not")?;
    let eq_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Eq))
        .ok_or("no Eq")?;
    adv_ensure(not_pos < eq_pos, "Not should come before Eq")?;
    Ok(())
}

// ── Edge-case: helper function boundary conditions ──────────────────────

#[test]
fn helper_with_negated_argument() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("exists(-1)")?;
    // Const 0, Const 1, Sub, Exists
    adv_ensure(constants.len() == 2, "should have 2 constants (0 and 1)")?;
    let has_sub = ops.iter().any(|op| matches!(op, ExprOp::Sub));
    let has_exists = ops.iter().any(|op| matches!(op, ExprOp::Exists));
    adv_ensure(has_sub, "should contain Sub for negation")?;
    adv_ensure(has_exists, "should contain Exists")?;
    let exists_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Exists))
        .ok_or("no Exists")?;
    let sub_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Sub))
        .ok_or("no Sub")?;
    adv_ensure(sub_pos < exists_pos, "Sub should come before Exists")?;
    Ok(())
}

#[test]
fn helper_with_binary_expression_argument() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("sum(1 + 2)")?;
    // Const 1, Const 2, Add, Sum
    adv_ensure(
        constants == vec![ConstValue::I64(1), ConstValue::I64(2)],
        "should have constants 1 and 2",
    )?;
    adv_ensure(ops.len() == 4, "should have 4 ops (2 loads, add, sum)")?;
    let add_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Add))
        .ok_or("no Add")?;
    let sum_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Sum))
        .ok_or("no Sum")?;
    adv_ensure(add_pos < sum_pos, "Add should come before Sum")?;
    Ok(())
}

#[test]
fn helper_with_parenthesized_complex_argument() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("length((1 + 2) * 3)")?;
    // LoadConst, LoadConst, Add, LoadConst, Mul, Length
    adv_ensure(ops.len() == 6, "should have 6 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Length), "should end with Length")?;
    Ok(())
}

#[test]
fn nested_helpers_in_binary_expression() -> Result<(), String> {
    // exists(1) == empty(0)
    let (ops, _constants, _max_stack) = lower("exists(1) == empty(0)")?;
    let has_exists = ops.iter().any(|op| matches!(op, ExprOp::Exists));
    let has_empty = ops.iter().any(|op| matches!(op, ExprOp::Empty));
    let has_eq = ops.iter().any(|op| matches!(op, ExprOp::Eq));
    adv_ensure(has_exists, "should contain Exists")?;
    adv_ensure(has_empty, "should contain Empty")?;
    adv_ensure(has_eq, "should contain Eq")?;
    let eq_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Eq))
        .ok_or("no Eq")?;
    let exists_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Exists))
        .ok_or("no Exists")?;
    let empty_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Empty))
        .ok_or("no Empty")?;
    adv_ensure(exists_pos < eq_pos, "Exists should come before Eq")?;
    adv_ensure(empty_pos < eq_pos, "Empty should come before Eq")?;
    Ok(())
}

#[test]
fn double_negation_in_helper() -> Result<(), String> {
    // exists(--5) => exists evaluated on (-(- 5))
    let (ops, constants, _max_stack) = lower("exists(--5)")?;
    // Const 0, Const 0, Const 5, Sub, Sub, Exists
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    let sub_count = ops.iter().filter(|op| matches!(op, ExprOp::Sub)).count();
    adv_ensure(sub_count == 2, "should have 2 Sub ops for double negation")?;
    let has_exists = ops.iter().any(|op| matches!(op, ExprOp::Exists));
    adv_ensure(has_exists, "should contain Exists")?;
    Ok(())
}

#[test]
fn helper_not_negated() -> Result<(), String> {
    // not empty(1)
    let (ops, _constants, _max_stack) = lower("not empty(1)")?;
    adv_ensure(ops.len() == 3, "should have 3 ops (Load, Empty, Not)")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Not), "should end with Not")?;
    let empty_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Empty))
        .ok_or("no Empty")?;
    let not_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Not))
        .ok_or("no Not")?;
    adv_ensure(empty_pos < not_pos, "Empty should come before Not")?;
    Ok(())
}

#[test]
fn ternary_helper_append_if_lowers_correctly() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("append_if(1, 2, 3)")?;
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    adv_ensure(ops.len() == 4, "should have 4 ops (3 loads + AppendIf)")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::AppendIf), "should end with AppendIf")?;
    Ok(())
}

#[test]
fn helper_with_null_argument() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("exists(null)")?;
    adv_ensure(
        constants == vec![ConstValue::Null],
        "should have Null constant",
    )?;
    adv_ensure(ops.len() == 2, "should be 2 ops (Load + Exists)")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Exists), "should end with Exists")?;
    Ok(())
}

#[test]
fn helper_with_boolean_argument() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("length(true)")?;
    adv_ensure(
        constants == vec![ConstValue::Bool(true)],
        "should have Bool(true)",
    )?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Length), "should end with Length")?;
    Ok(())
}

#[test]
fn helper_with_zero_argument() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("sum(0)")?;
    adv_ensure(constants == vec![ConstValue::I64(0)], "should have I64(0)")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Sum), "should end with Sum")?;
    Ok(())
}

// ── Edge-case: reference lowering through accessor path ─────────────────

#[test]
fn accessor_with_many_segments_produces_multi_path() -> Result<(), String> {
    let (ops, _constants, accessors) = lower_with_accessors("$slots.3.0.1.2.3.4.5")?;
    adv_ensure(ops.len() == 1, "should be single LoadAccessor op")?;
    let accessor = accessors.first().ok_or("missing accessor")?;
    adv_ensure(accessor.root == SlotIdx::new(3), "root should be slot 3")?;
    adv_ensure(accessor.path.len() == 6, "should have 6 path segments")?;
    Ok(())
}

#[test]
fn multiple_accessors_in_expression_produce_separate_entries() -> Result<(), String> {
    // $slot.0 == $slots.1.2
    let (ops, _constants, accessors) = lower_with_accessors("$slot.0 == $slots.1.2")?;
    // LoadSlot(0), LoadAccessor(0), Eq
    adv_ensure(ops.len() == 3, "should have 3 ops")?;
    adv_ensure(accessors.len() == 1, "should have 1 accessor entry")?;
    let accessor = accessors.first().ok_or("missing accessor")?;
    adv_ensure(
        accessor.root == SlotIdx::new(1),
        "accessor root should be slot 1",
    )?;
    adv_ensure(accessor.path.len() == 1, "should have 1 path segment")?;
    Ok(())
}

#[test]
fn accessor_with_single_segment_creates_one_path_entry() -> Result<(), String> {
    let (ops, _constants, accessors) = lower_with_accessors("$slots.5.0")?;
    adv_ensure(ops.len() == 1, "should be single LoadAccessor")?;
    let accessor = accessors.first().ok_or("missing accessor")?;
    adv_ensure(accessor.root == SlotIdx::new(5), "root should be slot 5")?;
    adv_ensure(accessor.path.len() == 1, "should have 1 segment")?;
    match accessor.path.first() {
        Some(PathSegment::Index(0)) => Ok(()),
        other => Err(format!("expected Index(0), got {other:?}")),
    }
}

// ── Edge-case: multiple expressions sharing a constant pool ──────────────

#[test]
fn two_expressions_share_constant_pool_independently() -> Result<(), String> {
    // Lower two separate expressions into the same constants vec
    let expr1 = parse_expression("1 + 2").map_err(|e| e.to_string())?;
    let expr2 = parse_expression("3 + 4").map_err(|e| e.to_string())?;
    let mut constants = Vec::new();
    let _prog1 = compile_expr_to_bytecode(&expr1, &mut constants).map_err(|e| e.to_string())?;
    adv_ensure(
        constants.len() == 2,
        "first expression should add 2 constants",
    )?;
    let _prog2 = compile_expr_to_bytecode(&expr2, &mut constants).map_err(|e| e.to_string())?;
    adv_ensure(
        constants.len() == 4,
        "second expression should add 2 more constants",
    )?;
    adv_ensure(
        constants
            == vec![
                ConstValue::I64(1),
                ConstValue::I64(2),
                ConstValue::I64(3),
                ConstValue::I64(4),
            ],
        "constants should be [1, 2, 3, 4]",
    )?;
    Ok(())
}

#[test]
fn expression_with_max_constants_near_overflow_boundary() -> Result<(), String> {
    // Fill constants to u16::MAX - 1 and verify the expression still compiles
    let expr = parse_expression("1").map_err(|e| e.to_string())?;
    let fill_count = usize::from(u16::MAX) - 1;
    let mut constants = Vec::with_capacity(fill_count + 1);
    for i in 0..fill_count {
        let value = i64::try_from(i).map_err(|error| error.to_string())?;
        constants.push(ConstValue::I64(value));
    }
    // constants has 65534 entries; pushing one more should succeed (65535 < 65536)
    let result = compile_expr_to_bytecode(&expr, &mut constants);
    adv_ensure(
        result.is_ok(),
        "should succeed with u16::MAX - 1 existing constants",
    )?;
    adv_ensure(
        constants.len() == fill_count + 1,
        "should have one more constant",
    )?;
    Ok(())
}

// ── Edge-case: chained comparisons left-associativity ───────────────────

#[test]
fn chained_lt_operators_left_associative() -> Result<(), String> {
    // 1 < 2 < 3 => (1 < 2) < 3
    let (ops, constants, _max_stack) = lower("1 < 2 < 3")?;
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    adv_ensure(ops.len() == 5, "should have 5 ops (3 loads + 2 Lt)")?;
    let lt_count = ops.iter().filter(|op| matches!(op, ExprOp::Lt)).count();
    adv_ensure(lt_count == 2, "should have 2 Lt ops")?;
    Ok(())
}

#[test]
fn chained_gte_operators_left_associative() -> Result<(), String> {
    // 5 >= 3 >= 1 => (5 >= 3) >= 1
    let (ops, constants, _max_stack) = lower("5 >= 3 >= 1")?;
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    let gte_count = ops.iter().filter(|op| matches!(op, ExprOp::Gte)).count();
    adv_ensure(gte_count == 2, "should have 2 Gte ops")?;
    Ok(())
}

#[test]
fn mixed_add_mul_left_assoc_same_precedence() -> Result<(), String> {
    // 1 + 2 - 3 => left-assoc: (1 + 2) - 3
    let (ops, _constants, _max_stack) = lower("1 + 2 - 3")?;
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    // First binary op should be Add, second should be Sub
    let first_bin = ops
        .iter()
        .find(|op| matches!(op, ExprOp::Add | ExprOp::Sub))
        .ok_or("missing first binary op")?;
    adv_ensure(
        matches!(first_bin, ExprOp::Add),
        "first binary op should be Add",
    )?;
    Ok(())
}

// ── Edge-case: negation of helper result ────────────────────────────────

#[test]
fn negation_of_helper_result_produces_sub() -> Result<(), String> {
    // -length(1) => Const 0, Const 1, Length, Sub
    let (ops, constants, _max_stack) = lower("-length(1)")?;
    adv_ensure(
        constants.first() == Some(&ConstValue::I64(0)),
        "first constant should be 0 for negation",
    )?;
    let has_length = ops.iter().any(|op| matches!(op, ExprOp::Length));
    let has_sub = ops.iter().any(|op| matches!(op, ExprOp::Sub));
    adv_ensure(has_length, "should contain Length")?;
    adv_ensure(has_sub, "should contain Sub for negation")?;
    let sub_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Sub))
        .ok_or("no Sub")?;
    let length_pos = ops
        .iter()
        .position(|op| matches!(op, ExprOp::Length))
        .ok_or("no Length")?;
    adv_ensure(length_pos < sub_pos, "Length should come before Sub")?;
    Ok(())
}

#[test]
fn not_of_not_of_boolean() -> Result<(), String> {
    // not not false => LoadConst(false), Not, Not
    let (ops, constants, _max_stack) = lower("not not false")?;
    adv_ensure(
        constants == vec![ConstValue::Bool(false)],
        "should have Bool(false)",
    )?;
    adv_ensure(ops.len() == 3, "should have 3 ops")?;
    let not_count = ops.iter().filter(|op| matches!(op, ExprOp::Not)).count();
    adv_ensure(not_count == 2, "should have 2 Not ops")?;
    Ok(())
}

// ── Edge-case: max stack tracking ───────────────────────────────────────

#[test]
fn max_stack_one_for_simple_load() -> Result<(), String> {
    let (_, _, max_stack) = lower("42")?;
    adv_ensure(max_stack == 1, "single load should have max_stack 1")
}

#[test]
fn max_stack_two_for_binary_op() -> Result<(), String> {
    let (_, _, max_stack) = lower("1 + 2")?;
    adv_ensure(max_stack >= 2, "binary op should have max_stack >= 2")
}

#[test]
fn max_stack_increases_with_complexity() -> Result<(), String> {
    let (_, _, ms_simple) = lower("1 + 2")?;
    let (_, _, ms_complex) = lower("1 + 2 * 3")?;
    adv_ensure(
        ms_complex >= ms_simple,
        "more complex expression should have >= max_stack",
    )
}

// ── Step reference lowering tests ────────────────────────────────────────

fn lower_with_step_slots(
    source: &str,
    step_slots: &[(Box<str>, SlotIdx)],
) -> Result<(Vec<ExprOp>, Vec<ConstValue>, Vec<AccessorProgram>), String> {
    let expr = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    let program =
        compile_expr_to_bytecode_with_step_slots(&expr, &mut constants, &mut accessors, step_slots)
            .map_err(|error| error.to_string())?;
    Ok((program.ops.into_vec(), constants, accessors))
}

#[test]
fn lowers_bare_step_reference_to_load_slot() -> Result<(), String> {
    let step_slots: [(Box<str>, SlotIdx); 2] = [
        (Box::from("build"), SlotIdx::new(3)),
        (Box::from("test"), SlotIdx::new(5)),
    ];
    let (ops, constants, accessors) = lower_with_step_slots("$steps.build", &step_slots)?;
    let expected_ops = vec![ExprOp::LoadSlot(SlotIdx::new(3))];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if !constants.is_empty() {
        return Err(format!("bare step ref created constants: {constants:?}"));
    }
    if !accessors.is_empty() {
        return Err(format!("bare step ref created accessors: {accessors:?}"));
    }
    Ok(())
}

#[test]
fn lowers_step_reference_with_result_field_to_accessor() -> Result<(), String> {
    let step_slots: [(Box<str>, SlotIdx); 1] = [(Box::from("build"), SlotIdx::new(3))];
    let (ops, constants, accessors) = lower_with_step_slots("$steps.build.result", &step_slots)?;
    let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
    let expected_accessors = vec![AccessorProgram {
        root: SlotIdx::new(3),
        path: vec![PathSegment::Index(0)].into_boxed_slice(),
    }];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if !constants.is_empty() {
        return Err(format!("step accessor created constants: {constants:?}"));
    }
    if accessors != expected_accessors {
        return Err(format!(
            "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
        ));
    }
    Ok(())
}

#[test]
fn rejects_unknown_step_reference() -> Result<(), String> {
    let step_slots: [(Box<str>, SlotIdx); 1] = [(Box::from("build"), SlotIdx::new(3))];
    let expr = parse_expression("$steps.unknown").map_err(|e| e.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    match compile_expr_to_bytecode_with_step_slots(
        &expr,
        &mut constants,
        &mut accessors,
        &step_slots,
    ) {
        Err(CompileError::UnknownReferenceName { kind, name, .. })
            if &*kind == "step" && &*name == "unknown" =>
        {
            Ok(())
        }
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

#[test]
fn rejects_step_reference_with_wrong_root() -> Result<(), String> {
    let step_slots: [(Box<str>, SlotIdx); 0] = [];
    let expr = parse_expression("$unknown.build").map_err(|e| e.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    match compile_expr_to_bytecode_with_step_slots(
        &expr,
        &mut constants,
        &mut accessors,
        &step_slots,
    ) {
        Err(CompileError::UnknownReferenceRoot { root, .. }) if &*root == "unknown" => Ok(()),
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

#[test]
fn step_reference_in_binary_expression() -> Result<(), String> {
    let step_slots: [(Box<str>, SlotIdx); 1] = [(Box::from("build"), SlotIdx::new(3))];
    let (ops, constants, accessors) = lower_with_step_slots("$steps.build == 42", &step_slots)?;
    let expected_ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(3)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Eq,
    ];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != vec![ConstValue::I64(42)] {
        return Err(format!("constants mismatch: {constants:?}"));
    }
    if !accessors.is_empty() {
        return Err(format!("unexpected accessors: {accessors:?}"));
    }
    Ok(())
}

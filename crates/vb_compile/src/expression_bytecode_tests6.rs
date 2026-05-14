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

mod tests6b;

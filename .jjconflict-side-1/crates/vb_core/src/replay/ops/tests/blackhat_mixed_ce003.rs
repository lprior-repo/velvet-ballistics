// =========================================================================
// BLACKHAT security regression tests -- replay expression evaluation
// =========================================================================

// --- FINDING BH-OPS-01: Division of i64::MIN by -1 must overflow ---
//
// checked_div correctly returns None for i64::MIN / -1, but we verify
// the replay path handles this case.

#[test]
fn blackhat_eval_div_min_by_neg_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(-1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-01: i64::MIN / -1 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-02: Subtraction of i64::MIN by 1 must underflow ---

#[test]
fn blackhat_eval_sub_min_by_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-02: i64::MIN - 1 must underflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-03: Addition of i64::MAX + 1 must overflow ---

#[test]
fn blackhat_eval_add_max_plus_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-03: i64::MAX + 1 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-04: Multiplication overflow at boundary values ---

#[test]
fn blackhat_eval_mul_boundary_overflow() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-04: i64::MIN * 2 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-05: Taint joins across LoadSlot operations ---
//
// When two slots are loaded (one Clean, one Secret) and added,
// the accumulated taint must be Secret.

#[test]
fn blackhat_taint_joins_across_expression() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![expr],
        vec![],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean)?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret)?;
    let mut store = ValueStore::new();

    let (value, taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    assert_eq!(
        value,
        SlotValue::I64(30),
        "BLACKHAT BH-OPS-05: value must be 30"
    );
    assert_eq!(
        taint,
        Taint::Secret,
        "BLACKHAT BH-OPS-05: taint must be Secret when one operand is Secret"
    );
    Ok(())
}

// --- FINDING BH-OPS-06: Expression stack overflow is caught ---
//
// Pushing more than capacity allows must fail, not corrupt memory.

#[test]
fn blackhat_expression_stack_overflow_is_safe() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(2).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = stack.push(SlotValue::I64(3));
    assert!(
        result.is_err(),
        "BLACKHAT BH-OPS-06: stack overflow must return error, not corrupt"
    );

    // Verify the stack is still consistent after the failed push
    let first = stack.pop().map_err(replay_err_to_core)?;
    assert_eq!(
        first,
        SlotValue::I64(2),
        "BLACKHAT BH-OPS-06: stack must remain consistent after overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-07: Helper ops evaluate like the engine (vb-cofe1) ---
//
// Replay previously rejected every helper opcode. The bead extends replay to
// cover them with engine-matching semantics, so these tests now pin the
// supported behavior. Wrong operand types must still fail evaluation
// (ExpressionEvalFailed), never panic and never surface as Internal.

fn eval_stack_helper(
    op: ExprOp,
    store: &mut ValueStore,
    values: &[SlotValue],
) -> Result<SlotValue, CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    for value in values {
        stack.push(*value).map_err(replay_err_to_core)?;
    }
    eval_replay_op(&plan, &run, store, op, &mut stack, &mut taint).map_err(replay_err_to_core)?;
    stack.pop().map_err(replay_err_to_core)
}

fn check_helper_type_error(op: ExprOp, values: &[SlotValue]) -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    for value in values {
        stack.push(*value).map_err(replay_err_to_core)?;
    }
    let result = eval_replay_op(&plan, &run, &mut store, op, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-07: {op:?} on wrong types must fail evaluation, not panic"
    );
    Ok(())
}

fn insert_text(store: &mut ValueStore, text: &str) -> Result<SymbolId, CoreError> {
    store.insert_symbol(text)
}

fn insert_numbers(store: &mut ValueStore, values: &[i64]) -> Result<ListId, CoreError> {
    let items: Vec<SlotValue> = values.iter().map(|&v| SlotValue::I64(v)).collect();
    store.insert_list(items.into_boxed_slice())
}

#[test]
fn blackhat_helper_contains_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let haystack = insert_text(&mut store, "hello world")?;
    let needle = insert_text(&mut store, "world")?;
    let missing = insert_text(&mut store, "xyz")?;
    let found = eval_stack_helper(
        ExprOp::Contains,
        &mut store,
        &[SlotValue::Symbol(haystack), SlotValue::Symbol(needle)],
    )?;
    if found != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: contains should find substring".into(),
        });
    }
    let absent = eval_stack_helper(
        ExprOp::Contains,
        &mut store,
        &[SlotValue::Symbol(haystack), SlotValue::Symbol(missing)],
    )?;
    if absent != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: contains should miss absent substring".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_starts_with_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello world")?;
    let prefix = insert_text(&mut store, "hello")?;
    let result = eval_stack_helper(
        ExprOp::StartsWith,
        &mut store,
        &[SlotValue::Symbol(text), SlotValue::Symbol(prefix)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: starts_with should match prefix".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_ends_with_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello world")?;
    let suffix = insert_text(&mut store, "world")?;
    let result = eval_stack_helper(
        ExprOp::EndsWith,
        &mut store,
        &[SlotValue::Symbol(text), SlotValue::Symbol(suffix)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: ends_with should match suffix".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_has_matches_engine_list_membership() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let present = eval_stack_helper(
        ExprOp::Has,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(2)],
    )?;
    if present != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: has should find list member".into(),
        });
    }
    let absent = eval_stack_helper(
        ExprOp::Has,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(9)],
    )?;
    if absent != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: has should miss absent member".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_exists_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key = insert_text(&mut store, "k")?;
    let object = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key,
                SlotValue::I64(1),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed".into(),
        })?;
    let present = eval_stack_helper(ExprOp::Exists, &mut store, &[SlotValue::Object(object)])?;
    if present != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: exists should hold for non-empty object".into(),
        });
    }
    let null = eval_stack_helper(ExprOp::Exists, &mut store, &[SlotValue::Null])?;
    if null != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: exists should fail for null".into(),
        });
    }
    check_helper_type_error(ExprOp::Exists, &[SlotValue::I64(1)])
}

#[test]
fn blackhat_helper_length_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello")?;
    let text_len = eval_stack_helper(ExprOp::Length, &mut store, &[SlotValue::Symbol(text)])?;
    if text_len != SlotValue::I64(5) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: length of text should be 5".into(),
        });
    }
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let list_len = eval_stack_helper(ExprOp::Length, &mut store, &[SlotValue::List(list)])?;
    if list_len != SlotValue::I64(3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: length of list should be 3".into(),
        });
    }
    check_helper_type_error(ExprOp::Length, &[SlotValue::Bool(true)])
}

#[test]
fn blackhat_helper_empty_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let null_empty = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::Null])?;
    if null_empty != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: empty null should be true".into(),
        });
    }
    let empty_list = insert_numbers(&mut store, &[])?;
    let list_empty = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::List(empty_list)])?;
    if list_empty != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: empty list should be true".into(),
        });
    }
    let full_list = insert_numbers(&mut store, &[1])?;
    let list_full = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::List(full_list)])?;
    if list_full != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: non-empty list should be false".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_append_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2])?;
    let result = eval_stack_helper(
        ExprOp::Append,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(3)],
    )?;
    let appended_id = match result {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append must return a list".into(),
            });
        }
    };
    let items = store
        .list(appended_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "appended list must resolve".into(),
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append should extend the list".into(),
        });
    }
    check_helper_type_error(ExprOp::Append, &[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn blackhat_helper_append_if_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1])?;
    let appended = eval_stack_helper(
        ExprOp::AppendIf,
        &mut store,
        &[
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(true),
        ],
    )?;
    let appended_id = match appended {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append_if must return a list".into(),
            });
        }
    };
    let items = store
        .list(appended_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "append_if list must resolve".into(),
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append_if true should extend the list".into(),
        });
    }
    let kept = eval_stack_helper(
        ExprOp::AppendIf,
        &mut store,
        &[
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(false),
        ],
    )?;
    let kept_id = match kept {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append_if must return a list".into(),
            });
        }
    };
    let kept_items = store
        .list(kept_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "append_if list must resolve".into(),
        })?;
    if kept_items != [SlotValue::I64(1)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append_if false should copy the list".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_merge_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key_a = insert_text(&mut store, "a")?;
    let key_b = insert_text(&mut store, "b")?;
    let left = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key_a,
                SlotValue::I64(1),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed".into(),
        })?;
    let right = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key_b,
                SlotValue::I64(2),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed".into(),
        })?;
    let result = eval_stack_helper(
        ExprOp::Merge,
        &mut store,
        &[SlotValue::Object(left), SlotValue::Object(right)],
    )?;
    let merged_id = match result {
        SlotValue::Object(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: merge must return an object".into(),
            });
        }
    };
    let fields = store
        .object(merged_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "merged object must resolve".into(),
        })?;
    if fields.len() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: merge should combine fields".into(),
        });
    }
    check_helper_type_error(ExprOp::Merge, &[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn blackhat_helper_sum_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let result = eval_stack_helper(ExprOp::Sum, &mut store, &[SlotValue::List(list)])?;
    if result != SlotValue::I64(6) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: sum should total list items".into(),
        });
    }
    check_helper_type_error(ExprOp::Sum, &[SlotValue::I64(1)])
}

#[test]
fn blackhat_helper_count_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let result = eval_stack_helper(ExprOp::Count, &mut store, &[SlotValue::List(list)])?;
    if result != SlotValue::I64(3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: count should report list length".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_unique_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 1, 3])?;
    let result = eval_stack_helper(ExprOp::Unique, &mut store, &[SlotValue::List(list)])?;
    let unique_id = match result {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: unique must return a list".into(),
            });
        }
    };
    let items = store
        .list(unique_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "unique list must resolve".into(),
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: unique should deduplicate preserving order".into(),
        });
    }
    Ok(())
}

// ---- Mixed I64/F64 arithmetic (vb-cofe1) ----

fn f64_value(raw: f64) -> Result<SlotValue, CoreError> {
    FiniteF64::new(raw).map(SlotValue::F64)
}

#[test]
fn replay_mixed_add_promotes_both_directions() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(
        ExprOp::Add,
        &mut store,
        &[SlotValue::I64(3), f64_value(0.5)?],
    )?;
    if result != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(3) + F64(0.5) should be F64(3.5)".into(),
        });
    }
    let flipped = eval_stack_helper(
        ExprOp::Add,
        &mut store,
        &[f64_value(1.5)?, SlotValue::I64(2)],
    )?;
    if flipped != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.5) + I64(2) should be F64(3.5)".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_mixed_sub_mul_div_promote() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(
        ExprOp::Sub,
        &mut store,
        &[SlotValue::I64(10), f64_value(0.5)?],
    )?;
    if result != f64_value(9.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) - F64(0.5) should be F64(9.5)".into(),
        });
    }
    let result = eval_stack_helper(
        ExprOp::Mul,
        &mut store,
        &[f64_value(2.5)?, SlotValue::I64(4)],
    )?;
    if result != f64_value(10.0)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.5) * I64(4) should be F64(10.0)".into(),
        });
    }
    let result = eval_stack_helper(
        ExprOp::Div,
        &mut store,
        &[SlotValue::I64(1), f64_value(2.0)?],
    )?;
    if result != f64_value(0.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(1) / F64(2.0) should be F64(0.5)".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_mixed_div_by_zero_fails() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Div, &[SlotValue::I64(1), f64_value(0.0)?])?;
    check_helper_type_error(ExprOp::Div, &[f64_value(1.0)?, SlotValue::I64(0)])
}

#[test]
fn replay_f64_add_and_div_by_zero() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(ExprOp::Add, &mut store, &[f64_value(1.5)?, f64_value(2.5)?])?;
    if result != f64_value(4.0)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.5) + F64(2.5) should be F64(4.0)".into(),
        });
    }
    check_helper_type_error(ExprOp::Div, &[f64_value(1.0)?, f64_value(0.0)?])
}

#[test]
fn replay_mixed_comparisons_promote_operands() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(
        ExprOp::Lt,
        &mut store,
        &[SlotValue::I64(3), f64_value(3.5)?],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(3) < F64(3.5) should be true".into(),
        });
    }
    let result = eval_stack_helper(
        ExprOp::Gte,
        &mut store,
        &[f64_value(2.0)?, SlotValue::I64(2)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.0) >= I64(2) should be true".into(),
        });
    }
    let result = eval_stack_helper(
        ExprOp::Gt,
        &mut store,
        &[SlotValue::I64(10), f64_value(9.5)?],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) > F64(9.5) should be true".into(),
        });
    }
    Ok(())
}

#[test]
fn replay_integer_error_contract_preserved() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Add, &[SlotValue::I64(i64::MAX), SlotValue::I64(1)])?;
    check_helper_type_error(ExprOp::Div, &[SlotValue::I64(10), SlotValue::I64(0)])?;
    check_helper_type_error(ExprOp::Div, &[SlotValue::I64(i64::MIN), SlotValue::I64(-1)])?;
    check_helper_type_error(ExprOp::Add, &[SlotValue::Bool(true), SlotValue::I64(1)])?;
    check_helper_type_error(ExprOp::Mul, &[f64_value(2.0)?, SlotValue::Null])
}

#[test]
fn replay_mixed_add_via_expr_program() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ])?;
    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![
            ConstValue::I64(3),
            ConstValue::F64(FiniteF64::new(2.5).map_err(|_| {
                CoreError::InternalInvariantViolation {
                    reason: "test constant must be finite".into(),
                }
            })?),
        ],
        vec![expr],
        vec![],
    )?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let (value, _) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;
    if value != f64_value(5.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "replay of I64(3) + F64(2.5) should be F64(5.5)".into(),
        });
    }
    Ok(())
}

// --- CE-003: Replay accessor must propagate field/list-item taint ---
//
// Engine evaluation joins segment taint from every object_field and list_item
// traversal. The replay path must do the same, otherwise accessor-derived
// values can land on the replay stack with weaker taint than the engine
// produced, silently downgrading secret propagation.

#[test]
fn ce003_replay_object_field_taint_joins_segment() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();
    let fields = vec![crate::value_store::ObjectField::with_taint(
        field_sym,
        SlotValue::I64(42),
        Taint::Secret,
    )]
    .into_boxed_slice();
    let obj_handle = store.insert_object(fields)?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(field_sym)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    // Root slot is Clean — segment taint is Secret. Combined result must be Secret.
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj_handle), Taint::Clean)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: accessor should resolve object field to I64(42)".into(),
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join field taint into accumulator".into(),
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_list_item_taint_joins_segment() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let values =
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice();
    let taints = vec![Taint::Clean, Taint::Secret, Taint::Clean].into_boxed_slice();
    let list_handle = store.insert_list_with_taint(values, taints)?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(1)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list_handle), Taint::Clean)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: accessor should resolve list[1] to I64(20)".into(),
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join list-item taint into accumulator".into(),
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_multi_segment_path_joins_every_segment_taint() -> Result<(), CoreError> {
    // Build: list[0] -> list_inner -> list_inner_field
    // list_inner contains one object; list_inner_field is Secret.
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();

    let inner_obj = store.insert_object(
        vec![crate::value_store::ObjectField::with_taint(
            field_sym,
            SlotValue::I64(7),
            Taint::Secret,
        )]
        .into_boxed_slice(),
    )?;
    let inner_list = store.insert_list_with_taint(
        vec![SlotValue::Object(inner_obj)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;
    let outer_list = store.insert_list_with_taint(
        vec![SlotValue::List(inner_list)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![
                PathSegment::Index(0),
                PathSegment::Index(0),
                PathSegment::Field(field_sym),
            ]
            .into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(outer_list), Taint::Clean)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: deep accessor should resolve to I64(7)".into(),
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay must join every segment's taint along the path".into(),
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_root_taint_joins_into_accumulator() -> Result<(), CoreError> {
    // Regression for vb-a68zo follow-up vb-yt5wq: the original CE-003 tests
    // only exercised segment taint propagation (root slot was always Clean).
    // This test sets the root slot taint to Secret while the field it points
    // at is Clean. A correct replay accessor must join the root taint into
    // the accumulator so the final taint is Secret. Mutation resistance:
    // deleting the `accumulated_taint = run.read_taint(program.root)` line
    // (or any other "drop root taint" mutation in `eval_accessor_for_replay`)
    // would leave the accumulator at Clean and this test would fail.
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();
    let fields = vec![crate::value_store::ObjectField::with_taint(
        field_sym,
        SlotValue::I64(99),
        Taint::Clean,
    )]
    .into_boxed_slice();
    let obj_handle = store.insert_object(fields)?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(field_sym)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    // Root slot carries Secret taint; field itself is Clean. The accessor
    // must surface Secret as the joined accumulator value.
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(obj_handle),
        Taint::Secret,
    )?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(99) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: accessor should resolve object field to I64(99)".into(),
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join root slot taint into accumulator".into(),
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_lattice_random_respected() -> Result<(), CoreError> {
    // Regression for vb-a68zo follow-up vb-yt5wq: the original CE-003 tests
    // only ever produced Taint::Secret. The taint lattice orders
    //   Clean(0) < DerivedFromSecret(1) < Secret(2) < Random(3) < TimeDependent(4)
    // via `join_taint` (max-discriminant wins). This test puts a Random
    // taint on the deepest segment and a Secret taint on the root slot,
    // then asserts the accumulator is Random -- proving the join uses the
    // lattice's max-rule rather than a hard-coded "Secret" shortcut.
    // Mutation resistance: replacing `join_taint(accumulated_taint, segment_taint)`
    // with identity (e.g. just `accumulated_taint`) would leave the result
    // at Secret, and this test would fail.
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();

    let inner_obj = store.insert_object(
        vec![crate::value_store::ObjectField::with_taint(
            field_sym,
            SlotValue::I64(11),
            Taint::Random,
        )]
        .into_boxed_slice(),
    )?;
    let inner_list = store.insert_list_with_taint(
        vec![SlotValue::Object(inner_obj)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;
    let outer_list = store.insert_list_with_taint(
        vec![SlotValue::List(inner_list)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![
                PathSegment::Index(0),
                PathSegment::Index(0),
                PathSegment::Field(field_sym),
            ]
            .into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    // Root slot is Secret -- must lose to the deeper Random segment under
    // the lattice join. Intermediate lists/objects stay Clean.
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(outer_list), Taint::Secret)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(11) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: deep accessor should resolve to I64(11)".into(),
        });
    }
    if taint != Taint::Random {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay must respect lattice -- Random(3) outranks Secret(2)".into(),
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_unique_rejects_non_list() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Unique, &[SlotValue::I64(7)])
}

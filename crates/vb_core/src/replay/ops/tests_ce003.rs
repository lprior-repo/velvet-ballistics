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
            reason: "I64(3) + F64(0.5) should be F64(3.5)",
        });
    }
    let flipped = eval_stack_helper(
        ExprOp::Add,
        &mut store,
        &[f64_value(1.5)?, SlotValue::I64(2)],
    )?;
    if flipped != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.5) + I64(2) should be F64(3.5)",
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
            reason: "I64(10) - F64(0.5) should be F64(9.5)",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Mul,
        &mut store,
        &[f64_value(2.5)?, SlotValue::I64(4)],
    )?;
    if result != f64_value(10.0)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.5) * I64(4) should be F64(10.0)",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Div,
        &mut store,
        &[SlotValue::I64(1), f64_value(2.0)?],
    )?;
    if result != f64_value(0.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(1) / F64(2.0) should be F64(0.5)",
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
            reason: "F64(1.5) + F64(2.5) should be F64(4.0)",
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
            reason: "I64(3) < F64(3.5) should be true",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Gte,
        &mut store,
        &[f64_value(2.0)?, SlotValue::I64(2)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.0) >= I64(2) should be true",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Gt,
        &mut store,
        &[SlotValue::I64(10), f64_value(9.5)?],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) > F64(9.5) should be true",
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
                    reason: "test constant must be finite",
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
            reason: "replay of I64(3) + F64(2.5) should be F64(5.5)",
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
            reason: "CE-003: accessor should resolve object field to I64(42)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join field taint into accumulator",
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
            reason: "CE-003: accessor should resolve list[1] to I64(20)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join list-item taint into accumulator",
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
            reason: "CE-003: deep accessor should resolve to I64(7)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay must join every segment's taint along the path",
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
            reason: "CE-003: accessor should resolve object field to I64(99)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join root slot taint into accumulator",
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
            reason: "CE-003: deep accessor should resolve to I64(11)",
        });
    }
    if taint != Taint::Random {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay must respect lattice -- Random(3) outranks Secret(2)",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_unique_rejects_non_list() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Unique, &[SlotValue::I64(7)])
}

include!("context_tests.rs");

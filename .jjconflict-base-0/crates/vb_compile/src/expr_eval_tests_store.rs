    // ===== Security regression tests =====

    #[test]
    fn eval_binary_op_i64_min_div_neg_one_is_integer_overflow_not_division_by_zero()
    -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MIN / -1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_div_by_zero_still_returns_division_by_zero() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero for 10 / 0".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_i64_min_div_neg_one_is_integer_overflow() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MIN / -1 end-to-end".into(),
            });
        };
        Ok(())
    }

    // ===== Store-aware helper tests =====

    #[test]
    fn eval_helper_with_store_empty_returns_true_for_null() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let args = [SlotValue::Null];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_empty_returns_true_for_empty_list() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_empty_returns_false_for_nonempty_list() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_empty_returns_true_for_empty_symbol() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let sym = store
            .insert_symbol(Box::<str>::from(""))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(sym)];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_empty_returns_false_for_nonempty_symbol() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let sym = store
            .insert_symbol(Box::<str>::from("hello"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(sym)];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_empty_returns_true_for_empty_object() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let obj = store
            .insert_object(vec![].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Object(obj)];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_empty_returns_type_mismatch_for_i64() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let args = [SlotValue::I64(42)];
        let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for empty(42) with store".into(),
            });
        };
        assert_eq!(expected, "text, list, object, or null");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_unique_deduplicates_list() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(1)].into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
        let SlotValue::List(unique_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from unique".into(),
            });
        };
        let items = store
            .list(unique_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], SlotValue::I64(1));
        assert_eq!(items[1], SlotValue::I64(2));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_unique_preserves_order() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(3),
                    SlotValue::I64(1),
                    SlotValue::I64(3),
                    SlotValue::I64(2),
                    SlotValue::I64(1),
                ]
                .into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
        let SlotValue::List(unique_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from unique".into(),
            });
        };
        let items = store
            .list(unique_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], SlotValue::I64(3));
        assert_eq!(items[1], SlotValue::I64(1));
        assert_eq!(items[2], SlotValue::I64(2));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_unique_returns_empty_list_for_empty_input() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
        let SlotValue::List(unique_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from unique".into(),
            });
        };
        let items = store
            .list(unique_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert!(items.is_empty());
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_unique_rejects_non_list() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let args = [SlotValue::I64(42)];
        let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for unique(42) with store".into(),
            });
        };
        assert_eq!(expected, "list");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_length_returns_list_length() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
        assert_eq!(result, SlotValue::I64(3));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_length_returns_symbol_length() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let sym = store
            .insert_symbol(Box::<str>::from("hello"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(sym)];
        let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
        assert_eq!(result, SlotValue::I64(5));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_length_returns_object_field_count() -> ExprResult<()> {
        use vb_core::value_store::ObjectField;
        let mut store = ValueStore::new();
        let obj = store
            .insert_object(
                vec![
                    ObjectField {
                        key: vb_core::ids::SymbolId::new(0),
                        value: SlotValue::I64(1),
                        taint: Taint::Clean,
                    },
                    ObjectField {
                        key: vb_core::ids::SymbolId::new(1),
                        value: SlotValue::I64(2),
                        taint: Taint::Clean,
                    },
                ]
                .into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Object(obj)];
        let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
        assert_eq!(result, SlotValue::I64(2));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_sum_sums_list_elements() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store)?;
        assert_eq!(result, SlotValue::I64(60));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_sum_returns_integer_overflow_on_overflow() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(i64::MAX), SlotValue::I64(1)].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store);
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for sum overflow".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_count_returns_list_length() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list)];
        let result = eval_helper_with_store(ExprHelper::Count, &args, &mut store)?;
        assert_eq!(result, SlotValue::I64(2));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_contains_checks_substring() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let haystack = store
            .insert_symbol(Box::<str>::from("hello world"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let needle = store
            .insert_symbol(Box::<str>::from("world"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(haystack), SlotValue::Symbol(needle)];
        let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_contains_returns_false_for_absent() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let haystack = store
            .insert_symbol(Box::<str>::from("hello world"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let needle = store
            .insert_symbol(Box::<str>::from("xyz"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(haystack), SlotValue::Symbol(needle)];
        let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_starts_with_checks_prefix() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let text = store
            .insert_symbol(Box::<str>::from("hello world"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let prefix = store
            .insert_symbol(Box::<str>::from("hello"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(text), SlotValue::Symbol(prefix)];
        let result = eval_helper_with_store(ExprHelper::StartsWith, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_ends_with_checks_suffix() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let text = store
            .insert_symbol(Box::<str>::from("hello world"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let suffix = store
            .insert_symbol(Box::<str>::from("world"))
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Symbol(text), SlotValue::Symbol(suffix)];
        let result = eval_helper_with_store(ExprHelper::EndsWith, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_has_checks_object_key() -> ExprResult<()> {
        use vb_core::value_store::ObjectField;
        let mut store = ValueStore::new();
        let key = vb_core::ids::SymbolId::new(42);
        let obj = store
            .insert_object(
                vec![ObjectField {
                    key,
                    value: SlotValue::I64(100),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Object(obj), SlotValue::Symbol(key)];
        let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_has_returns_false_for_missing_key() -> ExprResult<()> {
        use vb_core::value_store::ObjectField;
        let mut store = ValueStore::new();
        let key_present = vb_core::ids::SymbolId::new(1);
        let key_absent = vb_core::ids::SymbolId::new(99);
        let obj = store
            .insert_object(
                vec![ObjectField {
                    key: key_present,
                    value: SlotValue::I64(1),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Object(obj), SlotValue::Symbol(key_absent)];
        let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_append_adds_item_to_list() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::List(list), SlotValue::I64(2)];
        let result = eval_helper_with_store(ExprHelper::Append, &args, &mut store)?;
        let SlotValue::List(new_list_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from append".into(),
            });
        };
        let items = store
            .list(new_list_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], SlotValue::I64(1));
        assert_eq!(items[1], SlotValue::I64(2));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_append_if_adds_when_true() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(true),
        ];
        let result = eval_helper_with_store(ExprHelper::AppendIf, &args, &mut store)?;
        let SlotValue::List(new_list_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from append_if".into(),
            });
        };
        let items = store
            .list(new_list_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(items.len(), 2);
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_append_if_skips_when_false() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(false),
        ];
        let result = eval_helper_with_store(ExprHelper::AppendIf, &args, &mut store)?;
        let SlotValue::List(new_list_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from append_if".into(),
            });
        };
        let items = store
            .list(new_list_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(items.len(), 1);
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_merge_combines_objects() -> ExprResult<()> {
        use vb_core::value_store::ObjectField;
        let mut store = ValueStore::new();
        let key_a = vb_core::ids::SymbolId::new(1);
        let key_b = vb_core::ids::SymbolId::new(2);
        let left = store
            .insert_object(
                vec![ObjectField {
                    key: key_a,
                    value: SlotValue::I64(10),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let right = store
            .insert_object(
                vec![ObjectField {
                    key: key_b,
                    value: SlotValue::I64(20),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let args = [SlotValue::Object(left), SlotValue::Object(right)];
        let result = eval_helper_with_store(ExprHelper::Merge, &args, &mut store)?;
        let SlotValue::Object(merged_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected Object from merge".into(),
            });
        };
        let fields = store
            .object(merged_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(fields.len(), 2);
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_store_empty_list_returns_true() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|_| ExprError::UnexpectedEof)?;
        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty].into_boxed_slice(),
            max_stack: 1,
        };
        let slots = vec![Some(SlotValue::List(list))];
        let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_store_unique_deduplicates() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique].into_boxed_slice(),
            max_stack: 1,
        };
        let slots = vec![Some(SlotValue::List(list))];
        let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
        let SlotValue::List(unique_id) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected List from unique".into(),
            });
        };
        let items = store
            .list(unique_id)
            .map_err(|_| ExprError::UnexpectedEof)?;
        assert_eq!(items.len(), 2);
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_store_length_returns_correct_count() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length].into_boxed_slice(),
            max_stack: 1,
        };
        let slots = vec![Some(SlotValue::List(list))];
        let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
        assert_eq!(result, SlotValue::I64(3));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_store_sum_computes_total() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
            )
            .map_err(|_| ExprError::UnexpectedEof)?;
        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum].into_boxed_slice(),
            max_stack: 1,
        };
        let slots = vec![Some(SlotValue::List(list))];
        let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
        assert_eq!(result, SlotValue::I64(60));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_exists_returns_false_for_null() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let args = [SlotValue::Null];
        let result = eval_helper_with_store(ExprHelper::Exists, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_helper_with_store_exists_returns_true_for_non_null() -> ExprResult<()> {
        let mut store = ValueStore::new();
        let args = [SlotValue::I64(1)];
        let result = eval_helper_with_store(ExprHelper::Exists, &args, &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

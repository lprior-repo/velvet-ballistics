#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Tests for the expression bytecode evaluator.
//! Extracted from eval.rs to satisfy the 300-line file limit.

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use crate::eval::{
        eval_binary_op, eval_expr_program, eval_expr_program_with_accessors_and_store,
        eval_expr_program_with_store, eval_helper, eval_helper_with_store, eval_unary_op,
    };
    use crate::lexer::{BinaryOp, UnaryOp};
    use crate::parser::ExprHelper;
    use crate::{AccessorContextAbsence, ExprError, ExprResult};
    use proptest;
    use proptest::prelude::*;
    use vb_core::limits::MAX_EXPRESSION_STACK;
    use vb_core::value::FiniteF64;
    use vb_core::value::Taint;
    use vb_core::value_store::ValueStore;
    use vb_core::{
        AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, ExprProgram, PathSegment,
        SlotIdx, SlotValue,
    };

    fn make_f64(value: f64) -> FiniteF64 {
        FiniteF64::new(value).expect("expected finite f64")
    }

    fn make_program(ops: Vec<ExprOp>) -> ExprResult<ExprProgram> {
        ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::StackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
    }

    fn make_accessor(root: SlotIdx, path: Vec<PathSegment>) -> AccessorProgram {
        AccessorProgram {
            root,
            path: path.into_boxed_slice(),
        }
    }

    fn eval_with_const(program: &ExprProgram, constants: Vec<ConstValue>) -> ExprResult<SlotValue> {
        let slots: Vec<Option<SlotValue>> = Vec::new();
        eval_expr_program(program, &slots, &constants)
    }

    #[test]
    fn evaluates_addition() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(19), ConstValue::I64(23)])?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn evaluates_subtraction() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(10), ConstValue::I64(3)])?;
        assert_eq!(result, SlotValue::I64(7));
        Ok(())
    }

    #[test]
    fn evaluates_multiplication() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Mul,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(6), ConstValue::I64(7)])?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn evaluates_division() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(42), ConstValue::I64(6)])?;
        assert_eq!(result, SlotValue::I64(7));
        Ok(())
    }

    #[test]
    fn rejects_division_by_zero() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(1), ConstValue::I64(0)]);
        assert!(matches!(result, Err(ExprError::DivisionByZero)));
        Ok(())
    }

    #[test]
    fn evaluates_equality() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Eq,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(5), ConstValue::I64(5)])?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn evaluates_inequality() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::NotEq,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(5), ConstValue::I64(3)])?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn evaluates_comparison_ops() -> ExprResult<()> {
        let constants = vec![ConstValue::I64(3), ConstValue::I64(5)];
        for (op, expected) in [
            (ExprOp::Lt, true),
            (ExprOp::Lte, true),
            (ExprOp::Gt, false),
            (ExprOp::Gte, false),
        ] {
            let program = make_program(vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                op,
            ])?;
            let result = eval_with_const(&program, constants.clone())?;
            assert_eq!(result, SlotValue::Bool(expected), "failed for {op:?}");
        }
        Ok(())
    }

    #[test]
    fn evaluates_boolean_not() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not])?;
        let result = eval_with_const(&program, vec![ConstValue::Bool(true)])?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn evaluates_boolean_and_or() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::And,
        ])?;
        let result = eval_with_const(
            &program,
            vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        )?;
        assert_eq!(result, SlotValue::Bool(false));

        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Or,
        ])?;
        let result = eval_with_const(
            &program,
            vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn evaluates_load_slot() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadSlot(SlotIdx::new(0))])?;
        let slots = vec![Some(SlotValue::I64(99))];
        let result = eval_expr_program(&program, &slots, &[])?;
        assert_eq!(result, SlotValue::I64(99));
        Ok(())
    }

    #[test]
    fn legacy_eval_load_accessor_returns_missing_context_not_unknown_operator() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let slots = vec![Some(SlotValue::I64(99))];
        let mut store = ValueStore::new();
        let result = eval_expr_program_with_store(&program, &slots, &[], &mut store);
        assert!(matches!(
            result,
            Err(ExprError::MissingAccessorContext {
                absence: AccessorContextAbsence::LegacyApiNoAccessorTable
            })
        ));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_loads_empty_path_root() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let accessors = vec![make_accessor(SlotIdx::new(0), Vec::new())];
        let slots = vec![Some(SlotValue::I64(42))];
        let mut store = ValueStore::new();
        let result = eval_expr_program_with_accessors_and_store(
            &program,
            &slots,
            &[],
            &accessors,
            &mut store,
        )?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_traverses_object_field_and_list_index() -> ExprResult<()> {
        use vb_core::value_store::ObjectField;

        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let mut store = ValueStore::new();
        let field = vb_core::ids::SymbolId::new(7);
        let list = store
            .insert_list(vec![SlotValue::I64(11), SlotValue::I64(42)].into_boxed_slice())
            .map_err(ExprError::from)?;
        let object = store
            .insert_object(
                vec![ObjectField {
                    key: field,
                    value: SlotValue::List(list),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(ExprError::from)?;
        let accessors = vec![make_accessor(
            SlotIdx::new(0),
            vec![PathSegment::Field(field), PathSegment::Index(1)],
        )];
        let slots = vec![Some(SlotValue::Object(object))];
        let result = eval_expr_program_with_accessors_and_store(
            &program,
            &slots,
            &[],
            &accessors,
            &mut store,
        )?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_rejects_accessor_out_of_bounds() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(1))])?;
        let accessors = vec![make_accessor(SlotIdx::new(0), Vec::new())];
        let slots = vec![Some(SlotValue::I64(42))];
        let mut store = ValueStore::new();
        let result = eval_expr_program_with_accessors_and_store(
            &program,
            &slots,
            &[],
            &accessors,
            &mut store,
        );
        assert!(matches!(
            result,
            Err(ExprError::AccessorOutOfBounds { accessor }) if accessor == AccessorIdx::new(1)
        ));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_distinguishes_root_slot_errors() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let accessors = vec![make_accessor(SlotIdx::new(1), Vec::new())];
        let mut store = ValueStore::new();
        let out_of_bounds = eval_expr_program_with_accessors_and_store(
            &program,
            &[Some(SlotValue::I64(42))],
            &[],
            &accessors,
            &mut store,
        );
        assert!(matches!(
            out_of_bounds,
            Err(ExprError::AccessorRootOutOfBounds { root }) if root == SlotIdx::new(1)
        ));

        let accessors = vec![make_accessor(SlotIdx::new(0), Vec::new())];
        let uninitialized = eval_expr_program_with_accessors_and_store(
            &program,
            &[None],
            &[],
            &accessors,
            &mut store,
        );
        assert!(matches!(
            uninitialized,
            Err(ExprError::AccessorRootUninitialized { root }) if root == SlotIdx::new(0)
        ));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_preserves_path_error_family() -> ExprResult<()> {
        use vb_core::value_store::ObjectField;

        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let mut store = ValueStore::new();
        let wrong_shape = vec![make_accessor(
            SlotIdx::new(0),
            vec![PathSegment::Field(vb_core::ids::SymbolId::new(1))],
        )];
        let wrong_shape_result = eval_expr_program_with_accessors_and_store(
            &program,
            &[Some(SlotValue::I64(42))],
            &[],
            &wrong_shape,
            &mut store,
        );
        assert!(matches!(
            wrong_shape_result,
            Err(ExprError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "number"
            })
        ));

        let present_field = vb_core::ids::SymbolId::new(2);
        let missing_field = vb_core::ids::SymbolId::new(3);
        let object = store
            .insert_object(
                vec![ObjectField {
                    key: present_field,
                    value: SlotValue::I64(1),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(ExprError::from)?;
        let missing_field_accessor = vec![make_accessor(
            SlotIdx::new(0),
            vec![PathSegment::Field(missing_field)],
        )];
        let missing_field_result = eval_expr_program_with_accessors_and_store(
            &program,
            &[Some(SlotValue::Object(object))],
            &[],
            &missing_field_accessor,
            &mut store,
        );
        assert!(matches!(
            missing_field_result,
            Err(ExprError::ObjectFieldNotFound { field }) if field == missing_field
        ));

        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(ExprError::from)?;
        let list_oob_accessor = vec![make_accessor(SlotIdx::new(0), vec![PathSegment::Index(9)])];
        let list_oob_result = eval_expr_program_with_accessors_and_store(
            &program,
            &[Some(SlotValue::List(list))],
            &[],
            &list_oob_accessor,
            &mut store,
        );
        assert!(matches!(
            list_oob_result,
            Err(ExprError::ListIndexOutOfBounds { index }) if index == 9
        ));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_rejects_invalid_object_handle() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let field = vb_core::ids::SymbolId::new(5);
        let forged_object = vb_core::ids::ObjectId::new(99);
        let accessors = vec![make_accessor(
            SlotIdx::new(0),
            vec![PathSegment::Field(field)],
        )];
        let slots = vec![Some(SlotValue::Object(forged_object))];
        let mut store = ValueStore::new();
        let result = eval_expr_program_with_accessors_and_store(
            &program,
            &slots,
            &[],
            &accessors,
            &mut store,
        );
        assert!(matches!(
            result,
            Err(ExprError::ObjectHandleOutOfBounds { object }) if object == forged_object
        ));
        Ok(())
    }

    #[test]
    fn eval_expr_program_with_accessors_rejects_invalid_list_handle() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadAccessor(AccessorIdx::new(0))])?;
        let forged_list = vb_core::ids::ListId::new(88);
        let accessors = vec![make_accessor(SlotIdx::new(0), vec![PathSegment::Index(0)])];
        let slots = vec![Some(SlotValue::List(forged_list))];
        let mut store = ValueStore::new();
        let result = eval_expr_program_with_accessors_and_store(
            &program,
            &slots,
            &[],
            &accessors,
            &mut store,
        );
        assert!(matches!(
            result,
            Err(ExprError::ListHandleOutOfBounds { list }) if list == forged_list
        ));
        Ok(())
    }

    #[test]
    fn rejects_type_mismatch_for_arithmetic() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::Bool(true), ConstValue::I64(1)]);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for Bool + I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn public_binary_eval_matches_stack_arithmetic() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(20), SlotValue::I64(22))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn public_unary_eval_rejects_wrong_type() -> crate::ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Not, SlotValue::I64(1));
        if let Err(ExprError::TypeMismatch { expected, found }) = result {
            assert_eq!(expected, "boolean");
            assert_eq!(found, "number");
        } else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for not(1)".into(),
            });
        }
        Ok(())
    }

    #[test]
    fn public_helper_eval_supports_scalar_exists() -> ExprResult<()> {
        let args = [SlotValue::Null];
        let result = eval_helper(ExprHelper::Exists, &args)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn end_to_end_lex_parse_compile_eval() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("3 + 4 * 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let _program = crate::bytecode::compile_expr_to_bytecode(&ast)?;
        let mut constants = Vec::new();
        let p2 = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&p2, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(11));
        Ok(())
    }

    // --- BDD evaluator tests ---

    #[test]
    fn eval_binary_op_adds_two_numbers() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(10), SlotValue::I64(32))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_subtracts_two_numbers() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(100), SlotValue::I64(58))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_multiplies_two_numbers() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(6), SlotValue::I64(7))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_divides_two_numbers() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(84), SlotValue::I64(2))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_compares_equality() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(7))?;
        assert_eq!(result, SlotValue::Bool(true));

        let result_ne = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(8))?;
        assert_eq!(result_ne, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_compares_less_than() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(3), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(true));

        let result_false = eval_binary_op(BinaryOp::Lt, SlotValue::I64(5), SlotValue::I64(3))?;
        assert_eq!(result_false, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_unary_op_negates_number() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(42))?;
        assert_eq!(result, SlotValue::I64(-42));
        Ok(())
    }

    #[test]
    fn eval_unary_op_not_negates_boolean() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Not, SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(false));

        let result_false = eval_unary_op(UnaryOp::Not, SlotValue::Bool(false))?;
        assert_eq!(result_false, SlotValue::Bool(true));
        Ok(())
    }

    // --- Section 46 no-short-circuit coverage for And/Or ---
    //
    // Production `eval_binary_op` (eval/ops.rs) must evaluate BOTH operands
    // for `And` and `Or` even when the left operand alone determines the
    // result. A short-circuit implementation (Rust `&&` / `||`) would skip
    // the right operand's type enforcement and silently return the wrong
    // variant. The four mismatch cases below catch any short-circuit
    // regression by forcing a `TypeMismatch` from the right operand to be
    // observed regardless of the left operand's value.

    #[test]
    fn eval_binary_op_and_evaluates_right_even_when_left_is_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(false), SlotValue::I64(7));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch (right must be type-checked even when left is false)".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_and_rejects_two_non_boolean_operands() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_or_evaluates_right_even_when_left_is_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::I64(7));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch (right must be type-checked even when left is true)".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_or_rejects_two_non_boolean_operands() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_and_accepts_two_boolean_operands() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_or_accepts_two_boolean_operands() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_and_returns_false_when_left_true_right_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_or_returns_true_when_left_false_right_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_applies_known_helper_exists() -> ExprResult<()> {
        let args = [SlotValue::Null];
        let result = eval_helper(ExprHelper::Exists, &args)?;
        assert_eq!(result, SlotValue::Bool(false));

        let args_non_null = [SlotValue::I64(1)];
        let result_non_null = eval_helper(ExprHelper::Exists, &args_non_null)?;
        assert_eq!(result_non_null, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_evaluates_simple_expression() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("2 * 3 + 4")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(10));
        Ok(())
    }

    #[test]
    fn eval_binary_op_returns_type_mismatch_for_string_in_arithmetic() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::Bool(true), SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn eval_expr_program_returns_stack_overflow_for_deep_nesting() -> ExprResult<()> {
        let mut ops = Vec::new();
        for i in 0..65u16 {
            ops.push(ExprOp::LoadConst(ConstIdx::new(i)));
        }
        let result = make_program(ops);
        let Err(ExprError::StackOverflow { max }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected StackOverflow".into(),
            });
        };
        assert_eq!(max, vb_core::limits::MAX_EXPRESSION_STACK);
        Ok(())
    }

    #[test]
    fn eval_expr_program_returns_stack_underflow_for_empty_stack_op() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![ExprOp::Add].into_boxed_slice(),
            max_stack: 0,
        };
        let result = eval_expr_program(&program, &[], &[]);
        let Err(ExprError::StackUnderflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected StackUnderflow".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_returns_division_by_zero() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero".into(),
            });
        };
        Ok(())
    }

    // --- Adversarial BDD evaluator tests ---

    #[test]
    fn eval_binary_op_i64_max_plus_one_is_error() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MAX + 1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_i64_min_minus_one_is_error() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MIN - 1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_i64_max_times_two_is_error() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX), SlotValue::I64(2));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MAX * 2".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_negation_of_i64_min_is_error() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for -i64::MIN".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_null_in_addition() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::Null, SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null + 1".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_bool_in_multiplication() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::Bool(true), SlotValue::I64(3));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for bool * int".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_number_in_and() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 and i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_null_in_or() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null or true".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn eval_unary_op_not_rejects_i64() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Not, SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for not 1".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_unary_op_neg_rejects_bool() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for -true".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn eval_expr_program_end_to_end_division_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("10 / 0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero for 10 / 0".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_end_to_end_overflow() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("9223372036854775807 + 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MAX + 1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_equality_null_vs_null() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("null == null")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_inequality_null_vs_i64() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("null != 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_boolean_and_type_mismatch() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("true and 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        assert!(
            matches!(result, Err(ExprError::TypeMismatch { .. })),
            "true and 1 should fail with TypeMismatch at eval"
        );
        Ok(())
    }

    #[test]
    fn eval_expr_program_chained_not_true() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("not not true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_double_negation() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("--5")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(5));
        Ok(())
    }

    #[test]
    fn eval_load_const_out_of_bounds_returns_error() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![ExprOp::LoadConst(ConstIdx::new(99))].into_boxed_slice(),
            max_stack: 1,
        };
        let result = eval_expr_program(&program, &[], &[]);
        assert!(
            matches!(result, Err(ExprError::UnexpectedEof)),
            "LoadConst with out-of-bounds index should return UnexpectedEof"
        );
        Ok(())
    }

    #[test]
    fn eval_load_slot_out_of_bounds_returns_error() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(99))].into_boxed_slice(),
            max_stack: 1,
        };
        let slots: Vec<Option<SlotValue>> = vec![];
        let result = eval_expr_program(&program, &slots, &[]);
        assert!(
            matches!(result, Err(ExprError::StackUnderflow)),
            "LoadSlot with out-of-bounds index should return StackUnderflow"
        );
        Ok(())
    }

    #[test]
    fn eval_helper_unique_rejects_non_list() -> ExprResult<()> {
        let args = [SlotValue::I64(42)];
        let result = eval_helper(ExprHelper::Unique, &args);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for unique(42)".into(),
            });
        };
        assert_eq!(expected, "list");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_length_returns_type_mismatch_for_non_list() -> ExprResult<()> {
        let args = [SlotValue::I64(42)];
        let result = eval_helper(ExprHelper::Length, &args);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for length(42)".into(),
            });
        };
        assert_eq!(expected, "list");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_empty_returns_true_for_null() -> ExprResult<()> {
        let args = [SlotValue::Null];
        let result = eval_helper(ExprHelper::Empty, &args)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_empty_returns_type_mismatch_for_i64() -> ExprResult<()> {
        let args = [SlotValue::I64(42)];
        let result = eval_helper(ExprHelper::Empty, &args);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for empty(42)".into(),
            });
        };
        assert_eq!(expected, "list or null");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_contains_returns_type_mismatch_for_i64_args() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, .. }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for Contains with I64 args".into(),
            });
        };
        assert!(
            expected.contains("text"),
            "expected should mention text, got: {expected}"
        );
        Ok(())
    }

    #[test]
    fn eval_helper_append_returns_type_mismatch_for_i64_args() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Append,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, .. }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for Append with I64 args".into(),
            });
        };
        assert!(
            expected.contains("list"),
            "expected should mention list, got: {expected}"
        );
        Ok(())
    }

    #[test]
    fn eval_helper_merge_returns_type_mismatch_for_i64_args() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Merge,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, .. }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for Merge with I64 args".into(),
            });
        };
        assert!(
            expected.contains("object"),
            "expected should mention object, got: {expected}"
        );
        Ok(())
    }

    #[test]
    fn eval_program_with_only_load_const_no_ops_returns_stack_overflow() -> ExprResult<()> {
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::StackOverflow { max }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected StackOverflow for extra values".into(),
            });
        };
        assert_eq!(max, vb_core::limits::MAX_EXPRESSION_STACK);
        Ok(())
    }

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

    // ===== Edge-case tests added for comprehensive coverage =====

    /// Edge case: F64 values are rejected by integer arithmetic (type mismatch).
    ///
    /// The expression evaluator only supports I64 arithmetic. Supplying an F64
    /// value to an Add operation must produce TypeMismatch, not a panic or
    /// silent coercion.
    #[test]
    fn edge_f64_rejected_by_integer_addition() -> ExprResult<()> {
        let f64_val = vb_core::value::FiniteF64::new(3.14).map_err(|_| ExprError::UnexpectedEof)?;
        let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(f64_val), SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 + I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    /// Edge case: F64 values rejected by comparison operators (type mismatch).
    ///
    /// Comparison operators expect I64 operands. Supplying F64 must fail.
    #[test]
    fn edge_f64_rejected_by_comparison() -> ExprResult<()> {
        let f64_val = vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?;
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::F64(f64_val), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 < I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    /// Edge case: Parenthesized groups override operator precedence.
    ///
    /// `(1 + 2) * (3 + 4)` should evaluate to `3 * 7 = 21`, not `1 + (2 * 3) + 4`.
    /// This tests that parentheses correctly override the default precedence
    /// where multiplication binds tighter than addition.
    #[test]
    fn edge_parenthesized_groups_override_precedence() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("(1 + 2) * (3 + 4)")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(21));
        Ok(())
    }

    /// Edge case: Comparison operators mixed with arithmetic in precedence.
    ///
    /// `1 + 2 < 3 + 4` should be parsed as `(1 + 2) < (3 + 4)` which evaluates
    /// to `3 < 7` = true. This verifies that comparison operators have lower
    /// precedence than arithmetic operators.
    #[test]
    fn edge_comparison_lower_precedence_than_arithmetic() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1 + 2 < 3 + 4")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    /// Edge case: Boolean equality and inequality across same and different values.
    ///
    /// `true == true` should be true, `true == false` should be false,
    /// `false != false` should be false, `true != false` should be true.
    #[test]
    fn edge_boolean_equality_and_inequality() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("true == true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));

        let tokens = crate::lexer::lex_expr("true != false")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    /// Edge case: All four comparison operators on equal values.
    ///
    /// For `5 < 5` -> false, `5 <= 5` -> true, `5 > 5` -> false, `5 >= 5` -> true.
    /// This verifies the boundary behavior of comparison operators.
    #[test]
    fn edge_comparison_boundary_on_equal_values() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(false), "5 < 5 should be false");

        let result = eval_binary_op(BinaryOp::Lte, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(true), "5 <= 5 should be true");

        let result = eval_binary_op(BinaryOp::Gt, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(false), "5 > 5 should be false");

        let result = eval_binary_op(BinaryOp::Gte, SlotValue::I64(5), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(true), "5 >= 5 should be true");
        Ok(())
    }

    /// Edge case: Negation of a negative number returns the positive.
    ///
    /// `-(-42)` should evaluate to `42`. This exercises the unary negation
    /// path with a non-zero, non-MIN negative operand.
    #[test]
    fn edge_negation_of_negative_returns_positive() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(-42))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    /// Edge case: Logical AND/OR with all boolean combinations.
    ///
    /// Verifies `false and false` -> false, `true or true` -> true,
    /// `false or false` -> false. Existing tests cover `true and false`
    /// and `true or false`, but these three combinations are untested.
    #[test]
    fn edge_logical_and_or_all_combinations() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Bool(false),
            SlotValue::Bool(false),
        )?;
        assert_eq!(result, SlotValue::Bool(false), "false and false");

        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true), "true or true");

        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false), "false or false");
        Ok(())
    }

    /// Edge case: Chained left-associative subtraction via end-to-end pipeline.
    ///
    /// `100 - 50 - 25` should parse as `(100 - 50) - 25 = 25`, not `100 - (50 - 25) = 75`.
    /// This verifies that subtraction is left-associative through the full
    /// lex -> parse -> compile -> eval pipeline.
    #[test]
    fn edge_left_associative_subtraction_e2e() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("100 - 50 - 25")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(25));
        Ok(())
    }

    /// Edge case: Division of negative by negative produces correct positive result.
    ///
    /// `-100 / -10` should yield `10`. Exercises signed division through the
    /// end-to-end pipeline.
    #[test]
    fn edge_negative_division_e2e() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("0 - 100")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program_neg100 = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;

        let tokens = crate::lexer::lex_expr("0 - 10")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants2 = Vec::new();
        let program_neg10 = crate::bytecode::compile_expr_with_pool(&ast, &mut constants2)?;

        // Use direct binary op since we cannot parse negative literals
        let neg100_result = eval_expr_program(&program_neg100, &[], &constants)?;
        let neg10_result = eval_expr_program(&program_neg10, &[], &constants2)?;

        let result = eval_binary_op(BinaryOp::Div, neg100_result, neg10_result)?;
        assert_eq!(result, SlotValue::I64(10));
        Ok(())
    }

    /// Edge case: `not true or true` precedence -- NOT binds tighter than OR.
    ///
    /// This should parse as `(not true) or true` = `false or true` = `true`.
    /// If OR bound tighter, it would be `not (true or true)` = `not true` = `false`.
    #[test]
    fn edge_not_binds_tighter_than_or() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("not true or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(
            result,
            SlotValue::Bool(true),
            "not true or true should be true"
        );
        Ok(())
    }

    /// Edge case: Whitespace-only input through full pipeline returns error.
    ///
    /// The lexer produces only an End token for whitespace-only input.
    /// The parser should reject this with UnexpectedToken.
    #[test]
    fn edge_whitespace_only_rejected_by_parser() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("   \t  \n  ")?;
        let result = crate::parser::parse_expr(&tokens);
        let Err(ExprError::UnexpectedToken { token }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnexpectedToken for whitespace-only".into(),
            });
        };
        assert!(
            token.contains("End"),
            "whitespace-only should produce End token error, got: {token}"
        );
        Ok(())
    }

    // ===== F64 arithmetic integration tests =====

    #[test]
    fn eval_binary_op_f64_adds_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(make_f64(1.5)),
            SlotValue::F64(make_f64(2.5)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(4.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_subtracts_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(make_f64(10.0)),
            SlotValue::F64(make_f64(3.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(7.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_multiplies_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(6.0)),
            SlotValue::F64(make_f64(7.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(42.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_divides_two_finite_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(84.0)),
            SlotValue::F64(make_f64(2.0)),
        )?;
        assert_eq!(result, SlotValue::F64(make_f64(42.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_division_by_zero_returns_nonfinite_float_not_division_by_zero()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::F64(make_f64(0.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for F64/0, not DivisionByZero".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_zero_divided_by_zero_returns_nonfinite_float() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(0.0)),
            SlotValue::F64(make_f64(0.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for 0.0/0.0 (NaN)".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_produces_nonfinite_float_when_result_is_infinity() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Div,
            SlotValue::F64(make_f64(f64::MAX)),
            SlotValue::F64(make_f64(f64::MIN_POSITIVE)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat when result is Inf".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_addition_produces_nonfinite_float_when_result_is_infinity()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(make_f64(f64::MAX)),
            SlotValue::F64(make_f64(f64::MAX)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for MAX + MAX".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_subtraction_produces_nonfinite_float_when_result_is_negative_infinity()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(make_f64(f64::MIN)),
            SlotValue::F64(make_f64(f64::MAX)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for MIN - MAX".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_multiplication_produces_nonfinite_float_when_result_is_infinity()
    -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(f64::MAX)),
            SlotValue::F64(make_f64(2.0)),
        );
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for MAX * 2.0".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_negation_returns_finite_value() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(make_f64(42.0)))?;
        assert_eq!(result, SlotValue::F64(make_f64(-42.0)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_negation_of_min_produces_max() -> ExprResult<()> {
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(make_f64(f64::MIN)))?;
        assert_eq!(result, SlotValue::F64(make_f64(f64::MAX)));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_greater_than() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Gt,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(3.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_greater_than_returns_false_when_less() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Gt,
            SlotValue::F64(make_f64(3.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_greater_than_or_equal_equal_case() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Gte,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_less_than() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lt,
            SlotValue::F64(make_f64(3.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_less_than_returns_false_when_greater() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lt,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(3.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_compares_less_than_or_equal_equal_case() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Lte,
            SlotValue::F64(make_f64(5.0)),
            SlotValue::F64(make_f64(5.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_equality_with_equal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Eq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(7.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_equality_with_unequal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Eq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(8.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_inequality_with_unequal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::NotEq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(8.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_inequality_with_equal_values() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::NotEq,
            SlotValue::F64(make_f64(7.0)),
            SlotValue::F64(make_f64(7.0)),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_rejects_type_mismatch_with_i64_in_add() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Add,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::I64(2),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 + I64".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_rejects_type_mismatch_with_bool_in_mul() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Mul,
            SlotValue::F64(make_f64(3.0)),
            SlotValue::Bool(true),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 * Bool".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_f64_rejects_null_in_subtraction() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Sub,
            SlotValue::F64(make_f64(1.0)),
            SlotValue::Null,
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for F64 - Null".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_end_to_end_division_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("3.14 / 0.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::NonFiniteFloat) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected NonFiniteFloat for F64 / 0.0 end-to-end".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_end_to_end_addition() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1.5 + 2.5")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::F64(make_f64(4.0)));
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_end_to_end_multiplication() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("6.0 * 7.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::F64(make_f64(42.0)));
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_complex_expression() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("2.0 + 3.0 * 4.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::F64(make_f64(14.0)));
        Ok(())
    }

    #[test]
    fn eval_expr_program_f64_division_yields_nonfinite_when_dividing_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1.0 / 0.0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        assert!(
            matches!(result, Err(ExprError::NonFiniteFloat)),
            "F64/0.0 should return NonFiniteFloat, not DivisionByZero"
        );
        Ok(())
    }

    #[test]
    fn i64_division_by_zero_still_returns_division_by_zero_not_nonfinite_float() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "I64/0 must return DivisionByZero, not NonFiniteFloat".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_i64_division_by_zero_returns_division_by_zero() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("10 / 0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero for I64/0".into(),
            });
        };
        Ok(())
    }

    /// NaN comparison test — POST-006 (IEEE 754 compliance).
    ///
    /// Per IEEE 754: NaN < x, NaN > x, NaN == x yield false for any x.
    /// NaN != NaN is true (NaN is not equal to itself).
    ///
    /// NOTE: NaN cannot enter the vb_expr system via FiniteF64::new() which
    /// rejects all NaN/Inf bit patterns at construction time. Therefore, the
    /// comparison operators eval_lt_op, eval_gt_op, eval_gte_op, eval_lte_op
    /// can NEVER receive NaN inputs through the public API by construction.
    ///
    /// This test verifies the IEEE 754 NaN comparison semantics DIRECTLY using
    /// raw f64::NAN, demonstrating that the behavior would be correct if NaN
    /// could somehow reach the comparison operators (which it cannot).
    ///
    /// The eval_*_op functions extract the inner f64 via .get() and perform
    /// standard Rust f64 comparisons, which follow IEEE 754 semantics.
    #[test]
    fn f64_comparison_nan_yields_false() {
        let nan = f64::NAN;
        let x = 1.0;

        assert!(!(nan < x), "NaN < x must be false per IEEE 754");
        assert!(!(nan > x), "NaN > x must be false per IEEE 754");
        assert!(!(nan == x), "NaN == x must be false per IEEE 754");
        assert!(!(nan <= x), "NaN <= x must be false per IEEE 754");
        assert!(!(nan >= x), "NaN >= x must be false per IEEE 754");
        assert!(
            nan != nan,
            "NaN != NaN must be true (NaN is not equal to itself)"
        );

        assert!(!(nan < 0.0), "NaN < 0.0 must be false");
        assert!(!(nan > 0.0), "NaN > 0.0 must be false");
        assert!(!(nan < f64::INFINITY), "NaN < Inf must be false");
        assert!(!(nan > f64::NEG_INFINITY), "NaN > -Inf must be false");
        assert!(!(nan == f64::MAX), "NaN == MAX must be false");
        assert!(!(nan == f64::MIN), "NaN == MIN must be false");
    }

    // ============================================================================
    // AND/OR Short-Circuit Tests (LETHAL-2)
    // ============================================================================

    // --- B1: AND returns SlotValue::Bool(true) when both operands are true ---

    #[test]
    fn and_returns_true_when_both_operands_are_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // --- B2: AND returns false when first is false but evaluates BOTH operands ---
    // Section 46: both operands must be evaluated before boolean operator applies.

    #[test]
    fn and_returns_false_when_first_is_false_and_evaluates_right() -> ExprResult<()> {
        // Section 46: left=false, right=non-bool → both evaluated → TypeMismatch
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(false), SlotValue::I64(0));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch since both operands evaluated".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // --- B3: AND returns false when first is true and second is false ---

    #[test]
    fn and_returns_false_when_first_is_true_and_second_is_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    // --- B4: OR returns SlotValue::Bool(false) when both operands are false ---

    #[test]
    fn or_returns_false_when_both_operands_are_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    // --- B5: OR returns true when first is true but evaluates BOTH operands ---
    // Section 46: both operands must be evaluated before boolean operator applies.

    #[test]
    fn or_returns_true_when_first_is_true_and_evaluates_right() -> ExprResult<()> {
        // Section 46: left=true, right=non-bool → both evaluated → TypeMismatch
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::I64(0));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch since both operands evaluated".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // --- B6: OR returns true when first is false and second is true ---

    #[test]
    fn or_returns_true_when_first_is_false_and_second_is_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // --- B7: AND evaluates BOTH operands when first produces TypeMismatch ---

    #[test]
    fn and_evaluates_both_operands_when_left_is_type_mismatch() -> ExprResult<()> {
        // left = I64 (TypeMismatch), right = Bool (valid)
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for I64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    /// B7 error accumulation test: left=I64, right=F64 (both non-bool)
    #[test]
    fn and_evaluates_both_operands_error_accumulation_i64_left_f64_right() -> ExprResult<()> {
        let left = SlotValue::I64(1);
        let right = SlotValue::F64(FiniteF64::new(1.0).expect("1.0 is finite"));
        let result = eval_binary_op(BinaryOp::And, left, right);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // --- B8: OR evaluates BOTH operands when first produces TypeMismatch ---

    #[test]
    fn or_evaluates_both_operands_when_left_is_type_mismatch() -> ExprResult<()> {
        // left = Null (TypeMismatch), right = Bool (valid)
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(false));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for Null".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    /// B8 error accumulation test: left=Null, right=F64 (both non-bool)
    #[test]
    fn or_evaluates_both_operands_error_accumulation_null_left_f64_right() -> ExprResult<()> {
        let left = SlotValue::Null;
        let right = SlotValue::F64(FiniteF64::new(1.0).expect("1.0 is finite"));
        let result = eval_binary_op(BinaryOp::Or, left, right);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    // ============================================================================
    // Exhaustive Bool × Bool Matrix for AND
    // ============================================================================

    #[test]
    fn and_false_false_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Bool(false),
            SlotValue::Bool(false),
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn and_false_true_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn and_true_false_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn and_true_true_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // ============================================================================
    // Exhaustive Bool × Bool Matrix for OR
    // ============================================================================

    #[test]
    fn or_false_false_returns_false() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn or_false_true_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn or_true_false_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(false))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn or_true_true_returns_true() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(true), SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // ============================================================================
    // Error variant tests for AND/OR TypeMismatch scenarios
    // ============================================================================

    #[test]
    fn and_rejects_i64_i64() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 and i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_i64_bool() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 and bool".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_bool_i64() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for bool and i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn or_rejects_null_bool() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null or bool".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn or_rejects_bool_null() -> ExprResult<()> {
        // left=false requires evaluating right, which is Null -> TypeMismatch
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), SlotValue::Null);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for false or null".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn or_rejects_i64_i64() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 or i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn or_rejects_f64_bool() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::F64(FiniteF64::new(1.0).expect("1.0 is finite")),
            SlotValue::Bool(true),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for f64 or bool".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_null_null() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Null, SlotValue::Null);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null and null".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn or_rejects_f64_f64() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::F64(FiniteF64::new(1.0).expect("1.0 is finite")),
            SlotValue::F64(FiniteF64::new(2.0).expect("2.0 is finite")),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for f64 or f64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn and_rejects_symbol_symbol() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
            SlotValue::Symbol(vb_core::ids::SymbolId::new(2)),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for symbol and symbol".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "symbol");
        Ok(())
    }

    #[test]
    fn or_rejects_list_list() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::Or,
            SlotValue::List(vb_core::ids::ListId::new(1)),
            SlotValue::List(vb_core::ids::ListId::new(2)),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for list or list".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "list");
        Ok(())
    }

    #[test]
    fn and_rejects_object_object() -> ExprResult<()> {
        let result = eval_binary_op(
            BinaryOp::And,
            SlotValue::Object(vb_core::ids::ObjectId::new(1)),
            SlotValue::Object(vb_core::ids::ObjectId::new(2)),
        );
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for object and object".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "object");
        Ok(())
    }

    // ============================================================================
    // Integration tests: full pipeline (lex → parse → compile → eval)
    // ============================================================================

    #[test]
    fn integration_and_true_true() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("true and true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn integration_and_false_any() -> ExprResult<()> {
        // "false and 1" — Section 46 mandates BOTH operands are evaluated.
        // 1 is non-bool, so evaluating it produces TypeMismatch.
        let tokens = crate::lexer::lex_expr("false and 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for non-bool right operand".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_true_any() -> ExprResult<()> {
        // "true or 1" — Section 46 mandates BOTH operands are evaluated.
        // 1 is non-bool, so evaluating it produces TypeMismatch.
        let tokens = crate::lexer::lex_expr("true or 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for non-bool right operand".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_false_false() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("false or false")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn integration_and_type_mismatch_left_i64() -> ExprResult<()> {
        // "1 and true" should error with TypeMismatch for number
        let tokens = crate::lexer::lex_expr("1 and true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for 1 and true".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_type_mismatch_left_null() -> ExprResult<()> {
        // "null or true" should error with TypeMismatch for null
        let tokens = crate::lexer::lex_expr("null or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null or true".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn integration_and_both_type_mismatch() -> ExprResult<()> {
        // "1 and 2" should error with TypeMismatch (both are non-bool)
        let tokens = crate::lexer::lex_expr("1 and 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for 1 and 2".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn integration_or_both_type_mismatch() -> ExprResult<()> {
        // "1 or 2" should error with TypeMismatch (both are non-bool)
        let tokens = crate::lexer::lex_expr("1 or 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for 1 or 2".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    // ============================================================================
    // Chained AND/OR tests
    // ============================================================================

    #[test]
    fn integration_chained_and() -> ExprResult<()> {
        // "true and true and true" should return true
        let tokens = crate::lexer::lex_expr("true and true and true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn integration_chained_or() -> ExprResult<()> {
        // "false or false or true" should return true
        let tokens = crate::lexer::lex_expr("false or false or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn integration_mixed_and_or() -> ExprResult<()> {
        // "(true and false) or true" should return true
        let tokens = crate::lexer::lex_expr("(true and false) or true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    // ============================================================================
    // Proptest invariants for AND/OR
    // ============================================================================

    proptest! {
        #[test]
        fn proptest_and_is_commutative_for_bools(a: bool, b: bool) {
            // For any two SlotValue::Bool values a, b:
            // eval_binary_op(And, Bool(a), Bool(b)) == eval_binary_op(And, Bool(b), Bool(a))
            let left = SlotValue::Bool(a);
            let right = SlotValue::Bool(b);
            let result_ab = eval_binary_op(BinaryOp::And, left, right).expect("And with bools must succeed");
            let result_ba = eval_binary_op(BinaryOp::And, right, left).expect("And with bools must succeed");
            prop_assert_eq!(result_ab, result_ba);
        }
    }

    proptest! {
        #[test]
        fn proptest_or_is_commutative_for_bools(a: bool, b: bool) {
            // For any two SlotValue::Bool values a, b:
            // eval_binary_op(Or, Bool(a), Bool(b)) == eval_binary_op(Or, Bool(b), Bool(a))
            let left = SlotValue::Bool(a);
            let right = SlotValue::Bool(b);
            let result_ab = eval_binary_op(BinaryOp::Or, left, right).expect("Or with bools must succeed");
            let result_ba = eval_binary_op(BinaryOp::Or, right, left).expect("Or with bools must succeed");
            prop_assert_eq!(result_ab, result_ba);
        }
    }

    #[test]
    fn proptest_and_false_left_always_false() {
        // Section 46: AND with false left and valid bool right is always false.
        // With non-bool right, Section 46 mandates evaluation → TypeMismatch.
        let left = SlotValue::Bool(false);
        // Test with valid bool right - should return false
        let result = eval_binary_op(BinaryOp::And, left, SlotValue::Bool(true))
            .unwrap_or_else(|e| panic!("And with bools must succeed, got error: {e:?}"));
        assert_eq!(result, SlotValue::Bool(false));

        // Section 46: non-bool right must be evaluated → TypeMismatch
        let result2 = eval_binary_op(BinaryOp::And, left, SlotValue::I64(0));
        let Err(ExprError::TypeMismatch { expected, found }) = result2 else {
            panic!("expected TypeMismatch for bool and i64");
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");

        let result3 = eval_binary_op(BinaryOp::And, left, SlotValue::Null);
        let Err(ExprError::TypeMismatch { expected, found }) = result3 else {
            panic!("expected TypeMismatch for bool and null");
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
    }

    #[test]
    fn proptest_or_true_left_always_true() {
        // Section 46: OR with true left and valid bool right is always true.
        // With non-bool right, Section 46 mandates evaluation → TypeMismatch.
        let left = SlotValue::Bool(true);
        // Test with valid bool right - should return true
        let result = eval_binary_op(BinaryOp::Or, left, SlotValue::Bool(false))
            .unwrap_or_else(|e| panic!("Or with bools must succeed, got error: {e:?}"));
        assert_eq!(result, SlotValue::Bool(true));

        // Section 46: non-bool right must be evaluated → TypeMismatch
        let result2 = eval_binary_op(BinaryOp::Or, left, SlotValue::I64(0));
        let Err(ExprError::TypeMismatch { expected, found }) = result2 else {
            panic!("expected TypeMismatch for bool or i64");
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");

        let result3 = eval_binary_op(BinaryOp::Or, left, SlotValue::Null);
        let Err(ExprError::TypeMismatch { expected, found }) = result3 else {
            panic!("expected TypeMismatch for bool or null");
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
    }

    #[test]
    fn proptest_and_requires_both_bools() {
        // Any non-bool left OR right produces TypeMismatch
        let non_bools = [
            (SlotValue::I64(1), "number"),
            (
                SlotValue::F64(FiniteF64::new(1.0).expect("1.0 is finite")),
                "number",
            ),
            (SlotValue::Null, "null"),
            (SlotValue::Symbol(vb_core::ids::SymbolId::new(1)), "symbol"),
        ];

        // left is non-bool, right is bool -> TypeMismatch
        for (left, expected_found) in &non_bools {
            let result = eval_binary_op(BinaryOp::And, *left, SlotValue::Bool(true));
            let Err(ExprError::TypeMismatch { expected, found }) = result else {
                panic!("AND with non-bool left should be TypeMismatch");
            };
            assert_eq!(expected, "boolean", "AND non-bool left type mismatch");
            assert_eq!(found, *expected_found, "AND non-bool left found type");
        }

        // left is bool, right is non-bool -> TypeMismatch
        for (right, expected_found) in &non_bools {
            let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(true), *right);
            let Err(ExprError::TypeMismatch { expected, found }) = result else {
                panic!("AND with non-bool right should be TypeMismatch");
            };
            assert_eq!(expected, "boolean", "AND non-bool right type mismatch");
            assert_eq!(found, *expected_found, "AND non-bool right found type");
        }
    }

    #[test]
    fn proptest_or_requires_both_bools() {
        // Any non-bool left OR right produces TypeMismatch
        let non_bools = [
            (SlotValue::I64(1), "number"),
            (
                SlotValue::F64(FiniteF64::new(1.0).expect("1.0 is finite")),
                "number",
            ),
            (SlotValue::Null, "null"),
            (SlotValue::Symbol(vb_core::ids::SymbolId::new(1)), "symbol"),
        ];

        // left is non-bool, right is bool -> TypeMismatch
        for (left, expected_found) in &non_bools {
            let result = eval_binary_op(BinaryOp::Or, *left, SlotValue::Bool(true));
            let Err(ExprError::TypeMismatch { expected, found }) = result else {
                panic!("OR with non-bool left should be TypeMismatch");
            };
            assert_eq!(expected, "boolean", "OR non-bool left type mismatch");
            assert_eq!(found, *expected_found, "OR non-bool left found type");
        }

        // left is bool, right is non-bool -> TypeMismatch
        for (right, expected_found) in &non_bools {
            let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(false), *right);
            let Err(ExprError::TypeMismatch { expected, found }) = result else {
                panic!("OR with non-bool right should be TypeMismatch");
            };
            assert_eq!(expected, "boolean", "OR non-bool right type mismatch");
            assert_eq!(found, *expected_found, "OR non-bool right found type");
        }
    }

    // ============================================================================
    // vb-bc33k: Proptest property tests for type_enforcers.
    //
    // Each property mirrors a Verus spec in `crates/vb_expr/src/eval/verus.rs`:
    //   - spec_expect_bool   -> expect_bool   (LEMMA-TYPE-001)
    //   - spec_expect_i64    -> expect_i64    (LEMMA-TYPE-002)
    //   - spec_expect_symbol -> expect_symbol (LEMMA-TYPE-003)
    //   - spec_expect_list   -> expect_list   (LEMMA-TYPE-004)
    //   - spec_expect_object -> expect_object (LEMMA-TYPE-005)
    //
    // Each expect_* must accept exactly one SlotValue variant and reject all
    // others with TypeMismatch { expected, found: type_name() }.
    // ============================================================================

    use crate::eval::type_enforcers::{
        expect_bool, expect_i64, expect_list, expect_object, expect_symbol,
    };
    use vb_core::ids::{BlobId, ListId, ObjectId, SymbolId};

    fn arb_slot_value() -> impl Strategy<Value = SlotValue> {
        prop_oneof![
            Just(SlotValue::Null),
            any::<bool>().prop_map(SlotValue::Bool),
            any::<i64>().prop_map(SlotValue::I64),
            (0u32..1024).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
            (0u32..1024).prop_map(|id| SlotValue::List(ListId::new(id))),
            (0u32..1024).prop_map(|id| SlotValue::Object(ObjectId::new(id))),
            (0u64..1024).prop_map(|id| SlotValue::Blob(BlobId::new(id))),
        ]
    }

    // -- LEMMA-TYPE-001: expect_bool iff value is Bool. -----------------------

    proptest! {
        #[test]
        fn type_enforcer_expect_bool_roundtrips_any_bool(input in any::<bool>()) {
            let value = SlotValue::Bool(input);
            match expect_bool(value) {
                Ok(recovered) => prop_assert_eq!(recovered, input),
                Err(err) => prop_assert!(false, "expect_bool must accept Bool: got {:?}", err),
            }
        }

        #[test]
        fn type_enforcer_expect_bool_rejects_non_bool(value in arb_slot_value()) {
            if matches!(value, SlotValue::Bool(_)) {
                return Ok(());
            }
            match expect_bool(value) {
                Err(ExprError::TypeMismatch { expected, found }) => {
                    prop_assert_eq!(expected, "boolean");
                    prop_assert_eq!(found, value.type_name());
                }
                other => prop_assert!(false, "expected TypeMismatch, got {:?}", other),
            }
        }
    }

    // -- LEMMA-TYPE-002: expect_i64 iff value is I64 (NOT F64). --------------

    proptest! {
        #[test]
        fn type_enforcer_expect_i64_roundtrips_any_i64(input in any::<i64>()) {
            let value = SlotValue::I64(input);
            match expect_i64(value) {
                Ok(recovered) => prop_assert_eq!(recovered, input),
                Err(err) => prop_assert!(false, "expect_i64 must accept I64: got {:?}", err),
            }
        }

        #[test]
        fn type_enforcer_expect_i64_rejects_non_i64(value in arb_slot_value()) {
            if matches!(value, SlotValue::I64(_)) {
                return Ok(());
            }
            match expect_i64(value) {
                Err(ExprError::TypeMismatch { expected, found }) => {
                    prop_assert_eq!(expected, "number");
                    prop_assert_eq!(found, value.type_name());
                }
                other => prop_assert!(false, "expected TypeMismatch, got {:?}", other),
            }
        }
    }

    // -- LEMMA-TYPE-003: expect_symbol iff value is Symbol. -------------------

    proptest! {
        #[test]
        fn type_enforcer_expect_symbol_roundtrips_any_id(id in any::<u32>()) {
            let symbol_id = SymbolId::new(id);
            let value = SlotValue::Symbol(symbol_id);
            let result: ExprResult<SymbolId> = expect_symbol(value);
            match result {
                Ok(recovered) => prop_assert_eq!(recovered, symbol_id),
                Err(err) => prop_assert!(false, "expect_symbol must accept Symbol: got {:?}", err),
            }
        }

        #[test]
        fn type_enforcer_expect_symbol_rejects_non_symbol(value in arb_slot_value()) {
            if matches!(value, SlotValue::Symbol(_)) {
                return Ok(());
            }
            match expect_symbol(value) {
                Err(ExprError::TypeMismatch { expected, found }) => {
                    prop_assert_eq!(expected, "text");
                    prop_assert_eq!(found, value.type_name());
                }
                other => prop_assert!(false, "expected TypeMismatch, got {:?}", other),
            }
        }
    }

    // -- LEMMA-TYPE-004: expect_list iff value is List. ----------------------

    proptest! {
        #[test]
        fn type_enforcer_expect_list_roundtrips_any_id(id in any::<u32>()) {
            let list_id = ListId::new(id);
            let value = SlotValue::List(list_id);
            let result: ExprResult<ListId> = expect_list(value);
            match result {
                Ok(recovered) => prop_assert_eq!(recovered, list_id),
                Err(err) => prop_assert!(false, "expect_list must accept List: got {:?}", err),
            }
        }

        #[test]
        fn type_enforcer_expect_list_rejects_non_list(value in arb_slot_value()) {
            if matches!(value, SlotValue::List(_)) {
                return Ok(());
            }
            match expect_list(value) {
                Err(ExprError::TypeMismatch { expected, found }) => {
                    prop_assert_eq!(expected, "list");
                    prop_assert_eq!(found, value.type_name());
                }
                other => prop_assert!(false, "expected TypeMismatch, got {:?}", other),
            }
        }
    }

    // -- LEMMA-TYPE-005: expect_object iff value is Object. ------------------

    proptest! {
        #[test]
        fn type_enforcer_expect_object_roundtrips_any_id(id in any::<u32>()) {
            let object_id = ObjectId::new(id);
            let value = SlotValue::Object(object_id);
            let result: ExprResult<ObjectId> = expect_object(value);
            match result {
                Ok(recovered) => prop_assert_eq!(recovered, object_id),
                Err(err) => prop_assert!(false, "expect_object must accept Object: got {:?}", err),
            }
        }

        #[test]
        fn type_enforcer_expect_object_rejects_non_object(value in arb_slot_value()) {
            if matches!(value, SlotValue::Object(_)) {
                return Ok(());
            }
            match expect_object(value) {
                Err(ExprError::TypeMismatch { expected, found }) => {
                    prop_assert_eq!(expected, "object");
                    prop_assert_eq!(found, value.type_name());
                }
                other => prop_assert!(false, "expected TypeMismatch, got {:?}", other),
            }
        }
    }

    // -- LEMMA-TYPE-006: SlotValue is a partition of variants. ---------------

    proptest! {
        #[test]
        fn type_enforcer_null_rejected_by_all_enforcers(_unit in Just(())) {
            let v = SlotValue::Null;
            prop_assert!(expect_bool(v).is_err());
            prop_assert!(expect_i64(v).is_err());
            prop_assert!(expect_symbol(v).is_err());
            prop_assert!(expect_list(v).is_err());
            prop_assert!(expect_object(v).is_err());
        }

        #[test]
        fn type_enforcer_exactly_zero_or_one_accepts(value in arb_slot_value()) {
            let ok_bool = expect_bool(value).is_ok();
            let ok_i64 = expect_i64(value).is_ok();
            let ok_symbol = expect_symbol(value).is_ok();
            let ok_list = expect_list(value).is_ok();
            let ok_object = expect_object(value).is_ok();

            let ok_count = [ok_bool, ok_i64, ok_symbol, ok_list, ok_object]
                .iter()
                .filter(|b| **b)
                .count();

            // The iff-correctness invariant: at most one type_enforcer accepts
            // a given value (Null is rejected by all; every other variant is
            // accepted by exactly one).
            prop_assert!(
                ok_count <= 1,
                "at most one type_enforcer accepts a value (got {})",
                ok_count
            );
        }
    }
}

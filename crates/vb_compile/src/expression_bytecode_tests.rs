#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::map_clone,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
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
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
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
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Tests for expression bytecode lowering.
use super::{
    compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors,
    compile_expr_to_bytecode_with_step_slots,
};
use crate::CompileError;
use crate::expression::parse_expression;
use vb_core::{AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, PathSegment, SlotIdx};

type LoweredWithAccessors = (Vec<ExprOp>, Vec<ConstValue>, Vec<AccessorProgram>);

fn lower(source: &str) -> Result<(Vec<ExprOp>, Vec<ConstValue>, u8), String> {
    let expr = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let program =
        compile_expr_to_bytecode(&expr, &mut constants).map_err(|error| error.to_string())?;
    Ok((program.ops.into_vec(), constants, program.max_stack))
}

fn lower_with_accessors(source: &str) -> Result<LoweredWithAccessors, String> {
    let expr = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    let program = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors)
        .map_err(|error| error.to_string())?;
    Ok((program.ops.into_vec(), constants, accessors))
}

#[test]
fn lowers_binary_expression_to_postfix_bytecode() -> Result<(), String> {
    let (ops, constants, max_stack) = lower("1 + 2 * 3")?;

    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Mul,
        ExprOp::Add,
    ];
    let expected_constants = vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != expected_constants {
        return Err(format!(
            "constants mismatch: expected {expected_constants:?}, got {constants:?}"
        ));
    }
    if max_stack != 3 {
        return Err(format!("max_stack mismatch: expected 3, got {max_stack}"));
    }
    Ok(())
}

#[test]
fn lowers_unary_not_and_numeric_negation() -> Result<(), String> {
    let (ops, constants, max_stack) = lower("not -1")?;

    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
        ExprOp::Not,
    ];
    let expected_constants = vec![ConstValue::I64(0), ConstValue::I64(1)];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != expected_constants {
        return Err(format!(
            "constants mismatch: expected {expected_constants:?}, got {constants:?}"
        ));
    }
    if max_stack != 2 {
        return Err(format!("max_stack mismatch: expected 2, got {max_stack}"));
    }
    Ok(())
}

#[test]
fn lowers_numeric_negation_of_f64_literal_emits_f64_zero_constant() -> Result<(), String> {
    // Regression: lower_numeric_negation used to emit ConstValue::I64(0)
    // unconditionally, so `Neg(0.0)` lowered to a Sub over mixed
    // I64/F64 operands and failed parity against the AST oracle. The
    // fix inspects the inner expression's static type and emits
    // ConstValue::F64(0.0) for F64 operands, leaving I64(0) for the
    // default case.
    let (ops, constants, _max_stack) = lower("-1.5")?;

    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ];
    let expected_constants = vec![
        ConstValue::F64(finite_f64(0.0)?),
        ConstValue::F64(finite_f64(1.5)?),
    ];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != expected_constants {
        return Err(format!(
            "constants mismatch: expected {expected_constants:?}, got {constants:?}"
        ));
    }
    Ok(())
}

#[test]
fn lowers_numeric_negation_of_nested_f64_emits_f64_zero_constants() -> Result<(), String> {
    // `--2.5` is `Neg(Neg(Literal(F64(2.5))))`. The inner Neg lowers
    // to F64(0.0) - F64(2.5), and the outer Neg must also lower to
    // F64(0.0) - <inner>, so the constants table should contain three
    // F64 values: outer 0.0, inner 0.0, and 2.5.
    let (ops, constants, _max_stack) = lower("--2.5")?;

    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Sub,
        ExprOp::Sub,
    ];
    let expected_constants = vec![
        ConstValue::F64(finite_f64(0.0)?),
        ConstValue::F64(finite_f64(0.0)?),
        ConstValue::F64(finite_f64(2.5)?),
    ];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != expected_constants {
        return Err(format!(
            "constants mismatch: expected {expected_constants:?}, got {constants:?}"
        ));
    }
    Ok(())
}

#[test]
fn lowers_numeric_negation_of_i64_literal_still_emits_i64_zero_constant() -> Result<(), String> {
    // The I64 path must be preserved: `Neg(1)` continues to lower to
    // I64(0) - I64(1) so existing test surfaces keep their constant
    // tables intact.
    let (ops, constants, _max_stack) = lower("-1")?;

    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ];
    let expected_constants = vec![ConstValue::I64(0), ConstValue::I64(1)];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != expected_constants {
        return Err(format!(
            "constants mismatch: expected {expected_constants:?}, got {constants:?}"
        ));
    }
    Ok(())
}

/// Wraps `FiniteF64::new` so a finiteness rejection surfaces as a
/// test failure with a readable message instead of a `Result::Err`
/// type error in the test body.
fn finite_f64(value: f64) -> Result<vb_core::FiniteF64, String> {
    vb_core::FiniteF64::new(value).map_err(|_| format!("expected {value} to be finite"))
}

#[test]
fn validates_helper_arity_before_stack_validation() -> Result<(), String> {
    let expr = parse_expression("contains(1)").map_err(|error| error.to_string())?;
    let mut constants = Vec::new();

    match compile_expr_to_bytecode(&expr, &mut constants) {
        Err(CompileError::ExpressionHelperArity {
            helper: "contains",
            actual: 1,
            ..
        }) => Ok(()),
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

#[test]
fn rejects_references_until_accessor_table_exists() -> Result<(), String> {
    let expr = parse_expression("$input.value").map_err(|error| error.to_string())?;
    let mut constants = Vec::new();

    match compile_expr_to_bytecode(&expr, &mut constants) {
        Err(CompileError::ExpressionLoweringUnsupported { ref feature })
            if feature.as_ref() == "accessor references" =>
        {
            Ok(())
        }
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

#[test]
fn lowers_direct_slot_reference_to_load_slot() -> Result<(), String> {
    let (ops, constants, accessors) = lower_with_accessors("$slot.7 == true")?;
    let expected_ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(7)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Eq,
    ];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if constants != vec![ConstValue::Bool(true)] {
        return Err(format!("unexpected constants: {constants:?}"));
    }
    if !accessors.is_empty() {
        return Err(format!("direct slot ref created accessors: {accessors:?}"));
    }
    Ok(())
}

#[test]
fn lowers_numeric_nested_slot_reference_to_accessor_table() -> Result<(), String> {
    let (ops, constants, accessors) = lower_with_accessors("$slots.2.0.3")?;
    let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
    let expected_accessors = vec![AccessorProgram {
        root: SlotIdx::new(2),
        path: vec![PathSegment::Index(0), PathSegment::Index(3)].into_boxed_slice(),
    }];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if !constants.is_empty() {
        return Err(format!("nested accessor created constants: {constants:?}"));
    }
    if accessors != expected_accessors {
        return Err(format!(
            "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
        ));
    }
    Ok(())
}

#[test]
fn lowers_single_list_index_accessor_to_table() -> Result<(), String> {
    let (ops, constants, accessors) = lower_with_accessors("$slot.4.12")?;
    let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
    let expected_accessors = vec![AccessorProgram {
        root: SlotIdx::new(4),
        path: vec![PathSegment::Index(12)].into_boxed_slice(),
    }];
    if ops != expected_ops {
        return Err(format!(
            "ops mismatch: expected {expected_ops:?}, got {ops:?}"
        ));
    }
    if !constants.is_empty() {
        return Err(format!("list accessor created constants: {constants:?}"));
    }
    if accessors != expected_accessors {
        return Err(format!(
            "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
        ));
    }
    Ok(())
}

#[test]
fn rejects_field_accessor_without_symbol_table() -> Result<(), String> {
    let expr = parse_expression("$slot.1.name").map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();

    match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
        Err(CompileError::UnsupportedAccessorReference { root, path, .. })
            if root.as_ref() == "slot.1" && path.as_ref() == "name" =>
        {
            Ok(())
        }
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

#[test]
fn rejects_field_accessor_after_list_index_without_mutating_table() -> Result<(), String> {
    let expr = parse_expression("$slots.1.0.name").map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();

    match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
        Err(CompileError::UnsupportedAccessorReference { root, path, .. })
            if root.as_ref() == "slots.1" && path.as_ref() == "0.name" =>
        {
            if !accessors.is_empty() {
                return Err(format!("unsupported accessor mutated table: {accessors:?}"));
            }
            Ok(())
        }
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

#[test]
fn rejects_empty_accessor_segment_with_exact_diagnostic_code() -> Result<(), String> {
    let expr = parse_expression("$slot.1..0").map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();

    match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
        Err(error @ CompileError::UnsupportedAccessorReference { .. }) => match error {
            CompileError::UnsupportedAccessorReference {
                ref root, ref path, ..
            } if root.as_ref() == "slot.1"
                && path.as_ref() == ".0"
                && error.diagnostic_code().as_str() == "UNSUPPORTED_ACCESSOR_REFERENCE" =>
            {
                Ok(())
            }
            other => Err(format!("unexpected lowering result: {other:?}")),
        },
        other => Err(format!("unexpected lowering result: {other:?}")),
    }
}

// ── Adversarial expression bytecode tests ────────────────────────────────

fn adv_lower_error(source: &str) -> Result<CompileError, String> {
    let expr = parse_expression(source).map_err(|error| error.to_string())?;
    let mut constants = Vec::new();
    match compile_expr_to_bytecode(&expr, &mut constants) {
        Err(error) => Ok(error),
        Ok(program) => Err(format!("lowering unexpectedly succeeded: {program:?}")),
    }
}

fn adv_ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

#[test]
fn text_literal_rejected_with_expression_lowering_unsupported() -> Result<(), String> {
    let expr = parse_expression("\"hello\"").map_err(|e| e.to_string())?;
    let mut constants = Vec::new();
    let result = compile_expr_to_bytecode(&expr, &mut constants);
    adv_ensure(
        matches!(
            result,
            Err(CompileError::ExpressionLoweringUnsupported {
                ref feature
            }) if feature.as_ref() == "text constants"
        ),
        "text literal did not produce exact text constants diagnostic",
    )
}

#[test]
fn accessor_reference_without_table_rejected_with_unsupported_feature() -> Result<(), String> {
    let error = adv_lower_error("$slot.5")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionLoweringUnsupported {
                ref feature
            } if feature.as_ref() == "accessor references"
        ),
        "accessor without table did not produce accessor references diagnostic",
    )
}

#[test]
fn constant_pool_overflow_in_expression_rejected() -> Result<(), String> {
    let expr = parse_expression("1").map_err(|e| e.to_string())?;
    // Pre-fill constants to u16::MAX + 1 (65536) so the next push fails
    let count = usize::from(u16::MAX) + 1;
    let mut constants = Vec::with_capacity(count);
    for i in 0..count {
        let value = i64::try_from(i).map_err(|error| error.to_string())?;
        constants.push(ConstValue::I64(value));
    }
    let result = compile_expr_to_bytecode(&expr, &mut constants);
    adv_ensure(
        matches!(result, Err(_)),
        "constant pool overflow (65536 existing + 1 new) should produce an error",
    )
}

#[test]
fn helper_zero_args_rejected_with_arity_mismatch() -> Result<(), String> {
    let error = adv_lower_error("contains()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "contains",
                expected: 2,
                actual: 0
            }
        ),
        "contains() did not produce arity mismatch",
    )
}

#[test]
fn helper_too_many_args_rejected_with_arity_mismatch() -> Result<(), String> {
    let error = adv_lower_error("append_if(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append_if",
                expected: 3,
                actual: 2
            }
        ),
        "append_if(1, 2) did not produce arity mismatch",
    )
}

#[test]
fn slot_accessor_with_non_numeric_root_rejected() -> Result<(), String> {
    let expr = parse_expression("$slot.abc").map_err(|e| e.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    let result = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors);
    adv_ensure(
        matches!(
            result,
            Err(CompileError::UnknownReferenceName { kind: "slot", .. })
        ),
        "non-numeric slot index did not produce slot reference error",
    )
}

#[test]
fn unknown_reference_root_rejected_in_slot_accessor_path() -> Result<(), String> {
    let expr = parse_expression("$unknown.5").map_err(|e| e.to_string())?;
    let mut constants = Vec::new();
    let mut accessors = Vec::new();
    let result = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors);
    adv_ensure(
        matches!(result, Err(CompileError::UnknownReferenceRoot { root, .. }) if root.as_ref() == "unknown"),
        "unknown root did not produce UnknownReferenceRoot",
    )
}

#[test]
fn deeply_nested_arithmetic_produces_valid_bytecode() -> Result<(), String> {
    // Build a left-associative chain: 1 + 2 + 3 + 4 + 5
    let (ops, constants, max_stack) = lower("1 + 2 + 3 + 4 + 5")?;
    adv_ensure(constants.len() == 5, "should have 5 constants")?;
    adv_ensure(ops.len() == 9, "should have 5 loads + 4 adds = 9 ops")?;
    adv_ensure(
        max_stack >= 2,
        "max_stack should be at least 2 for left-assoc chain",
    )?;
    Ok(())
}

#[test]
fn nested_negation_produces_correct_bytecode() -> Result<(), String> {
    // --5 should produce: LoadConst(0), LoadConst(0), LoadConst(5), Sub, Sub
    let (ops, _constants, _max_stack) = lower("--5")?;
    adv_ensure(ops.len() == 5, "nested negation should produce 5 ops")?;
    // Check last two ops are Sub
    let fourth = ops.get(3).ok_or("missing 4th op")?;
    let fifth = ops.get(4).ok_or("missing 5th op")?;
    adv_ensure(matches!(fourth, ExprOp::Sub), "4th op should be Sub")?;
    adv_ensure(matches!(fifth, ExprOp::Sub), "5th op should be Sub")?;
    Ok(())
}

// ── Edge-case expression bytecode tests ──────────────────────────────────

// 1. Empty string constant expressions

#[test]
fn empty_string_literal_rejected_as_unsupported() -> Result<(), String> {
    let error = adv_lower_error("\"\"")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionLoweringUnsupported { ref feature }
                if feature.as_ref() == "text constants"
        ),
        "empty string literal did not produce text constants diagnostic",
    )
}

// 2. Non-empty string constants (rejected as ExpressionLoweringUnsupported)

#[test]
fn nonempty_string_literal_rejected_as_unsupported() -> Result<(), String> {
    let error = adv_lower_error("\"hello world\"")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionLoweringUnsupported { ref feature }
                if feature.as_ref() == "text constants"
        ),
        "non-empty string literal did not produce text constants diagnostic",
    )
}

#[test]
fn string_in_helper_call_rejected_as_unsupported() -> Result<(), String> {
    // contains($slot.0, "needle") - first arg is a reference (rejected),
    // but the string arg alone would also fail if lowered first
    let error = adv_lower_error("contains(\"a\", \"b\")")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionLoweringUnsupported { ref feature }
                if feature.as_ref() == "text constants"
        ),
        "string arguments in helper should be rejected as text constants",
    )
}

// 3. Large integer constants (near i64::MAX, i64::MIN, zero)

#[test]
fn zero_integer_constant_lowers_correctly() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("0")?;
    adv_ensure(
        constants == vec![ConstValue::I64(0)],
        "zero should produce I64(0) constant",
    )?;
    adv_ensure(ops.len() == 1, "single zero should produce one op")?;
    adv_ensure(
        matches!(ops.first(), Some(ExprOp::LoadConst(_))),
        "zero should produce LoadConst op",
    )
}

#[test]
fn near_max_integer_constant_lowers_correctly() -> Result<(), String> {
    let source = i64::MAX.to_string();
    let (ops, constants, _max_stack) = lower(&source)?;
    adv_ensure(
        constants == vec![ConstValue::I64(i64::MAX)],
        "i64::MAX should produce correct constant",
    )?;
    adv_ensure(
        matches!(ops.first(), Some(ExprOp::LoadConst(_))),
        "i64::MAX should produce LoadConst op",
    )
}

#[test]
fn near_min_integer_constant_lowers_correctly() -> Result<(), String> {
    // i64::MIN = -9223372036854775808 cannot be parsed as a literal because
    // the lexer treats the minus as unary negation and 9223372036854775808
    // overflows i64::MAX. Instead verify a large negative constant is lowered
    // through the negation path correctly.
    let (ops, constants, _max_stack) = lower("-9999999999")?;
    adv_ensure(
        constants == vec![ConstValue::I64(0), ConstValue::I64(9999999999)],
        "large negative should produce 0 and absolute value constants",
    )?;
    adv_ensure(ops.len() == 3, "negation should produce 3 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Sub), "should end with Sub")?;
    Ok(())
}

#[test]
fn negative_one_integer_constant_lowers_correctly() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("-1")?;
    adv_ensure(
        constants == vec![ConstValue::I64(0), ConstValue::I64(1)],
        "negation of 1 should produce 0 and 1 constants",
    )?;
    adv_ensure(ops.len() == 3, "negation should produce 3 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Sub), "negation should end with Sub")?;
    Ok(())
}

#[test]
fn large_integer_in_binary_expression() -> Result<(), String> {
    let source = format!("{} + {}", i64::MAX - 1, 1);
    let (ops, constants, _max_stack) = lower(&source)?;
    adv_ensure(constants.len() == 2, "should have 2 constants")?;
    adv_ensure(
        constants.first() == Some(&ConstValue::I64(i64::MAX - 1)),
        "first constant should be i64::MAX - 1",
    )?;
    adv_ensure(
        constants.get(1) == Some(&ConstValue::I64(1)),
        "second constant should be 1",
    )?;
    adv_ensure(ops.len() == 3, "should have 2 loads + 1 add = 3 ops")?;
    Ok(())
}

// 4. Boolean constant expressions (true, false)

#[test]
fn true_boolean_lowers_to_const() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("true")?;
    adv_ensure(
        constants == vec![ConstValue::Bool(true)],
        "true should produce Bool(true) constant",
    )?;
    adv_ensure(ops.len() == 1, "true should produce one op")?;
    adv_ensure(
        matches!(ops.first(), Some(ExprOp::LoadConst(_))),
        "true should produce LoadConst op",
    )
}

#[test]
fn false_boolean_lowers_to_const() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("false")?;
    adv_ensure(
        constants == vec![ConstValue::Bool(false)],
        "false should produce Bool(false) constant",
    )?;
    adv_ensure(ops.len() == 1, "false should produce one op")?;
    adv_ensure(
        matches!(ops.first(), Some(ExprOp::LoadConst(_))),
        "false should produce LoadConst op",
    )
}

#[test]
fn boolean_equality_expression_lowers_correctly() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("true == false")?;
    adv_ensure(
        constants == vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        "true == false should produce two boolean constants",
    )?;
    adv_ensure(
        ops.len() == 3,
        "should have 3 ops (LoadConst, LoadConst, Eq)",
    )?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Eq), "should end with Eq op")?;
    Ok(())
}

// 5. Null constant expressions

#[test]
fn null_constant_lowers_to_const() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("null")?;
    adv_ensure(
        constants == vec![ConstValue::Null],
        "null should produce Null constant",
    )?;
    adv_ensure(ops.len() == 1, "null should produce one op")?;
    adv_ensure(
        matches!(ops.first(), Some(ExprOp::LoadConst(_))),
        "null should produce LoadConst op",
    )
}

#[test]
fn null_equality_expression_lowers_correctly() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("null == null")?;
    adv_ensure(
        constants == vec![ConstValue::Null, ConstValue::Null],
        "null == null should produce two null constants",
    )?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Eq), "should end with Eq")?;
    Ok(())
}

#[test]
fn null_inequality_expression_lowers_correctly() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("null != 0")?;
    adv_ensure(
        constants == vec![ConstValue::Null, ConstValue::I64(0)],
        "null != 0 should produce null and zero constants",
    )?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::NotEq), "should end with NotEq")?;
    Ok(())
}

// 6. Nested expressions (parenthesized, deeply nested)

#[test]
fn parenthesized_expression_lowers_identically() -> Result<(), String> {
    let (ops_unparen, constants_unparen, max_unparen) = lower("1 + 2")?;
    let (ops_paren, constants_paren, max_paren) = lower("(1 + 2)")?;
    adv_ensure(
        ops_unparen == ops_paren,
        "parenthesized ops should match unparenthesized",
    )?;
    adv_ensure(
        constants_unparen == constants_paren,
        "parenthesized constants should match unparenthesized",
    )?;
    adv_ensure(
        max_unparen == max_paren,
        "parenthesized max_stack should match unparenthesized",
    )?;
    Ok(())
}

#[test]
fn nested_parentheses_preserve_precedence() -> Result<(), String> {
    // ((1 + 2)) should be the same as 1 + 2
    let (ops, _constants, _max_stack) = lower("((1 + 2))")?;
    adv_ensure(ops.len() == 3, "double-parenthesized 1+2 should be 3 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Add), "should end with Add")?;
    Ok(())
}

#[test]
fn deeply_nested_unary_not_lowers_correctly() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("not not true")?;
    adv_ensure(ops.len() == 3, "not not true should be 3 ops")?;
    adv_ensure(
        matches!(ops.get(1), Some(ExprOp::Not)),
        "second op should be Not",
    )?;
    adv_ensure(
        matches!(ops.get(2), Some(ExprOp::Not)),
        "third op should be Not",
    )?;
    Ok(())
}

#[test]
fn deeply_nested_mixed_arithmetic_and_negation() -> Result<(), String> {
    let (ops, _constants, max_stack) = lower("-(1 + -(2 * 3))")?;
    adv_ensure(
        max_stack >= 3,
        "nested negation and arithmetic should need stack >= 3",
    )?;
    // Verify the expression compiles without error and has reasonable ops
    adv_ensure(
        ops.len() >= 7,
        "complex nested expression should have many ops",
    )?;
    Ok(())
}

// 7. BinaryOp edge cases (division, multiplication, subtraction)

#[test]
fn division_lowers_to_div_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("10 / 2")?;
    adv_ensure(
        ops.len() == 3,
        "division should be 3 ops (LoadConst, LoadConst, Div)",
    )?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Div), "should end with Div op")?;
    Ok(())
}

#[test]
fn multiplication_lowers_to_mul_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("3 * 4")?;
    adv_ensure(ops.len() == 3, "multiplication should be 3 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Mul), "should end with Mul op")?;
    Ok(())
}

#[test]
fn subtraction_lowers_to_sub_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("10 - 3")?;
    adv_ensure(ops.len() == 3, "subtraction should be 3 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Sub), "should end with Sub op")?;
    Ok(())
}

#[test]
fn subtraction_with_addition_left_associative() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("10 - 3 + 2")?;
    // left assoc: (10 - 3) + 2 => LoadConst, LoadConst, Sub, LoadConst, Add = 5 ops
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    adv_ensure(
        matches!(ops.get(2), Some(ExprOp::Sub)),
        "3rd op should be Sub",
    )?;
    adv_ensure(
        matches!(ops.get(4), Some(ExprOp::Add)),
        "5th op should be Add",
    )?;
    Ok(())
}

#[test]
fn division_and_multiplication_left_associative() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("12 / 3 * 2")?;
    adv_ensure(constants.len() == 3, "should have 3 constants")?;
    // (12 / 3) * 2 => LoadConst, LoadConst, Div, LoadConst, Mul = 5 ops
    adv_ensure(ops.len() == 5, "should have 5 ops")?;
    adv_ensure(
        matches!(ops.get(2), Some(ExprOp::Div)),
        "Div should be at index 2",
    )?;
    adv_ensure(
        matches!(ops.get(4), Some(ExprOp::Mul)),
        "Mul should be at index 4",
    )?;
    Ok(())
}

// 8. Comparison operators (Gt, Gte, Lt, Lte, NotEq, And, Or)

#[test]
fn greater_than_lowers_to_gt_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("5 > 3")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Gt), "should end with Gt")?;
    Ok(())
}

#[test]
fn greater_than_or_equal_lowers_to_gte_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("5 >= 3")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Gte), "should end with Gte")?;
    Ok(())
}

#[test]
fn less_than_lowers_to_lt_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("3 < 5")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Lt), "should end with Lt")?;
    Ok(())
}

#[test]
fn less_than_or_equal_lowers_to_lte_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("3 <= 5")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Lte), "should end with Lte")?;
    Ok(())
}

#[test]
fn not_equal_lowers_to_noteq_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("1 != 2")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::NotEq), "should end with NotEq")?;
    Ok(())
}

#[test]
fn and_operator_lowers_to_and_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("true and false")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::And), "should end with And")?;
    Ok(())
}

#[test]
fn or_operator_lowers_to_or_op() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("true or false")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Or), "should end with Or")?;
    Ok(())
}

#[test]
fn chained_comparison_operators_lowers_with_precedence() -> Result<(), String> {
    // 1 < 2 and 3 > 0 or 4 >= 4
    // Precedence: comparison > and > or
    // => ((1 < 2) and (3 > 0)) or (4 >= 4)
    let (ops, _constants, _max_stack) = lower("1 < 2 and 3 > 0 or 4 >= 4")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Or), "root should be Or")?;
    // The ops before the Or should include Lt, And, Gt, Gte
    let has_lt = ops.iter().any(|op| matches!(op, ExprOp::Lt));
    let has_and = ops.iter().any(|op| matches!(op, ExprOp::And));
    let has_gt = ops.iter().any(|op| matches!(op, ExprOp::Gt));
    let has_gte = ops.iter().any(|op| matches!(op, ExprOp::Gte));
    adv_ensure(has_lt, "should contain Lt op")?;
    adv_ensure(has_and, "should contain And op")?;
    adv_ensure(has_gt, "should contain Gt op")?;
    adv_ensure(has_gte, "should contain Gte op")?;
    Ok(())
}

#[test]
fn equality_and_inequality_left_associative() -> Result<(), String> {
    // == and != have same precedence (left-assoc)
    let (ops, _constants, _max_stack) = lower("1 == 2 != 3")?;
    // (1 == 2) != 3 => LoadConst, LoadConst, Eq, LoadConst, NotEq = 5 ops
    adv_ensure(ops.len() == 5, "should have 5 ops for chained equality")?;
    adv_ensure(
        matches!(ops.get(2), Some(ExprOp::Eq)),
        "Eq should be at index 2",
    )?;
    adv_ensure(
        matches!(ops.get(4), Some(ExprOp::NotEq)),
        "NotEq should be at index 4",
    )?;
    Ok(())
}

// 9. Helper arity validation for all helpers

#[test]
fn exists_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("exists(1)")?;
    adv_ensure(ops.len() == 2, "exists(1) should be load + Exists = 2 ops")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Exists)),
        "should end with Exists op",
    )
}

#[test]
fn exists_with_zero_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("exists()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "exists",
                expected: 1,
                actual: 0
            }
        ),
        "exists() should fail with arity mismatch",
    )
}

#[test]
fn exists_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("exists(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "exists",
                expected: 1,
                actual: 2
            }
        ),
        "exists(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn sum_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("sum(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Sum)),
        "should end with Sum op",
    )
}

#[test]
fn sum_with_zero_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("sum()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "sum",
                expected: 1,
                actual: 0
            }
        ),
        "sum() should fail with arity mismatch",
    )
}

#[test]
fn merge_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("merge(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Merge)),
        "should end with Merge op",
    )
}

#[test]
fn merge_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("merge(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "merge",
                expected: 2,
                actual: 1
            }
        ),
        "merge(1) should fail with arity mismatch",
    )
}

#[test]
fn merge_with_three_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("merge(1, 2, 3)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "merge",
                expected: 2,
                actual: 3
            }
        ),
        "merge(1, 2, 3) should fail with arity mismatch",
    )
}

#[test]
fn append_if_with_three_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("append_if(1, 2, 3)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::AppendIf)),
        "should end with AppendIf op",
    )
}

#[test]
fn append_if_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("append_if(1, 2)")?;
    // This is already tested above, but verifying with the adv_ensure pattern
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append_if",
                expected: 3,
                actual: 2
            }
        ),
        "append_if(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn append_if_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("append_if(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append_if",
                expected: 3,
                actual: 1
            }
        ),
        "append_if(1) should fail with arity mismatch",
    )
}

#[test]
fn contains_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("contains(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Contains)),
        "should end with Contains op",
    )
}

#[test]
fn contains_with_three_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("contains(1, 2, 3)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "contains",
                expected: 2,
                actual: 3
            }
        ),
        "contains(1, 2, 3) should fail with arity mismatch",
    )
}

#[test]
fn length_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("length(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Length)),
        "should end with Length op",
    )
}

#[test]
fn length_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("length(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "length",
                expected: 1,
                actual: 2
            }
        ),
        "length(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn unique_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("unique(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Unique)),
        "should end with Unique op",
    )
}

#[test]
fn unique_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("unique(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "unique",
                expected: 1,
                actual: 2
            }
        ),
        "unique(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn coalesce_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("coalesce(null, 7)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Coalesce)),
        "should end with Coalesce op",
    )
}

#[test]
fn coalesce_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("coalesce(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "coalesce",
                expected: 2,
                actual: 1
            }
        ),
        "coalesce(1) should fail with arity mismatch",
    )
}

#[test]
fn count_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("count(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Count)),
        "should end with Count op",
    )
}

#[test]
fn count_with_zero_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("count()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "count",
                expected: 1,
                actual: 0
            }
        ),
        "count() should fail with arity mismatch",
    )
}

#[test]
fn empty_helper_with_one_arg_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("empty(1)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Empty)),
        "should end with Empty op",
    )
}

#[test]
fn empty_helper_with_zero_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("empty()")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "empty",
                expected: 1,
                actual: 0
            }
        ),
        "empty() should fail with arity mismatch",
    )
}

#[test]
fn empty_helper_with_two_args_rejected() -> Result<(), String> {
    let error = adv_lower_error("empty(1, 2)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "empty",
                expected: 1,
                actual: 2
            }
        ),
        "empty(1, 2) should fail with arity mismatch",
    )
}

#[test]
fn append_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("append(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Append)),
        "should end with Append op",
    )
}

#[test]
fn append_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("append(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "append",
                expected: 2,
                actual: 1
            }
        ),
        "append(1) should fail with arity mismatch",
    )
}

#[test]
fn starts_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("starts_with(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::StartsWith)),
        "should end with StartsWith op",
    )
}

#[test]
fn starts_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("starts_with(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "starts_with",
                expected: 2,
                actual: 1
            }
        ),
        "starts_with(1) should fail with arity mismatch",
    )
}

#[test]
fn ends_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("ends_with(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::EndsWith)),
        "should end with EndsWith op",
    )
}

#[test]
fn has_with_two_args_succeeds() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("has(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Has)),
        "should end with Has op",
    )
}

#[test]
fn has_with_one_arg_rejected() -> Result<(), String> {
    let error = adv_lower_error("has(1)")?;
    adv_ensure(
        matches!(
            error,
            CompileError::ExpressionHelperArity {
                helper: "has",
                expected: 2,
                actual: 1
            }
        ),
        "has(1) should fail with arity mismatch",
    )
}

// Additional edge-case: helper with nested expression argument

#[test]
fn helper_with_nested_expression_argument() -> Result<(), String> {
    let (ops, constants, _max_stack) = lower("contains(1 + 2, 3)")?;
    adv_ensure(
        constants.len() == 3,
        "nested expression in helper should produce 3 constants",
    )?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Contains)),
        "should end with Contains",
    )?;
    Ok(())
}

// Additional edge-case: helper within a binary expression

#[test]
fn helper_result_used_in_binary_expression() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("length(1) + count(2)")?;
    // LoadConst, Length, LoadConst, Count, Add = 5 ops
    adv_ensure(ops.len() == 5, "helper in binary should have 5 ops")?;
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Add), "should end with Add")?;
    let has_length = ops.iter().any(|op| matches!(op, ExprOp::Length));
    let has_count = ops.iter().any(|op| matches!(op, ExprOp::Count));
    adv_ensure(has_length, "should contain Length op")?;
    adv_ensure(has_count, "should contain Count op")?;
    Ok(())
}

// Additional edge-case: not operator applied to helper result

#[test]
fn not_applied_to_helper_result() -> Result<(), String> {
    let (ops, _constants, _max_stack) = lower("not contains(1, 2)")?;
    adv_ensure(
        matches!(ops.last(), Some(ExprOp::Not)),
        "should end with Not",
    )?;
    let has_contains = ops.iter().any(|op| matches!(op, ExprOp::Contains));
    adv_ensure(has_contains, "should contain Contains op")?;
    Ok(())
}

// ── Edge-case: deeply nested expressions ────────────────────────────────

#[test]
fn deeply_nested_binary_tree_produces_valid_bytecode() -> Result<(), String> {
    // Build a balanced binary tree: ((1+2)+(3+4))+((5+6)+(7+8))
    let (ops, constants, max_stack) = lower("((1 + 2) + (3 + 4)) + ((5 + 6) + (7 + 8))")?;
    adv_ensure(constants.len() == 8, "should have 8 constants")?;
    // 8 LoadConst + 7 Add = 15 ops
    adv_ensure(ops.len() == 15, "should have 15 ops")?;
    let add_count = ops.iter().filter(|op| matches!(op, ExprOp::Add)).count();
    adv_ensure(add_count == 7, "should have 7 Add ops")?;
    adv_ensure(
        max_stack >= 4,
        "max_stack should be at least 4 for balanced tree",
    )?;
    Ok(())
}

#[test]
fn deeply_nested_left_chain_arithmetic() -> Result<(), String> {
    // Left-deep chain of 20 additions: 1+2+3+...+20
    let parts: Vec<String> = (1..=20i64).map(|i| i.to_string()).collect();
    let expr = parts.join(" + ");
    let (ops, constants, max_stack) = lower(&expr)?;
    adv_ensure(constants.len() == 20, "should have 20 constants")?;
    // 20 loads + 19 adds = 39 ops
    adv_ensure(ops.len() == 39, "should have 39 ops")?;
    adv_ensure(
        max_stack >= 2,
        "left-deep chain should need at least 2 stack slots",
    )?;
    Ok(())
}

#[test]
fn deeply_nested_mixed_and_or_precedence() -> Result<(), String> {
    // a or b and c or d and e => (a or (b and c)) or (d and e)
    let (ops, constants, _max_stack) = lower("true or false and true or false and true")?;
    // Constants: true, false, true, false, true = 5
    adv_ensure(constants.len() == 5, "should have 5 constants")?;
    let and_count = ops.iter().filter(|op| matches!(op, ExprOp::And)).count();
    let or_count = ops.iter().filter(|op| matches!(op, ExprOp::Or)).count();
    adv_ensure(and_count == 2, "should have 2 And ops")?;
    adv_ensure(or_count == 2, "should have 2 Or ops")?;
    // Root should be Or (left-assoc)
    let last = ops.last().ok_or("missing last op")?;
    adv_ensure(matches!(last, ExprOp::Or), "root should be Or")?;
    Ok(())
}

#[test]
fn deeply_nested_helper_inside_binary() -> Result<(), String> {
    // contains(length(1) == 0, true)
    // Wait, contains takes 2 args not a binary expr.
    // Let's use: length(1) == 0 and empty(1)
    let (ops, _constants, _max_stack) = lower("length(1) == 0 and empty(1)")?;
    let has_length = ops.iter().any(|op| matches!(op, ExprOp::Length));
    let has_empty = ops.iter().any(|op| matches!(op, ExprOp::Empty));
    let has_eq = ops.iter().any(|op| matches!(op, ExprOp::Eq));
    let has_and = ops.iter().any(|op| matches!(op, ExprOp::And));
    adv_ensure(has_length, "should contain Length")?;
    adv_ensure(has_empty, "should contain Empty")?;
    adv_ensure(has_eq, "should contain Eq")?;
    adv_ensure(has_and, "should contain And")?;
    Ok(())
}

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
        matches!(result, Ok(_)),
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

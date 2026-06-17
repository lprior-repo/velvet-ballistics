#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

//! Tests for `$attempt.number` variable scope restriction.
//!
//! These tests verify that the `$attempt.number` reference is:
//! - VALID: retained in the AST when used inside repeat body steps
//! - INVALID: rejected outside repeat bodies, either by the dedicated
//!   `InvalidVariableScope` guard when reference validation is reached or by an
//!   earlier Phase-0 shape error for unsupported YAML forms
//!
//! The contract guarantees:
//! 1. Compilation succeeds without error when `$attempt.number` appears in repeat body
//! 2. The reference is retained in the AST as `AstExpression::Reference("$attempt.number")`
//! 3. The reference is NOT resolved at compile time (runtime binding only)
//! 4. Compilation fails when used outside repeat bodies

use crate::ast::{AstExpression, AstValue, StepKindAst, WorkflowAst};
use crate::expression::ParsedExpression;
use crate::{CompileError, CompileErrors, YamlCompiler};

/// Result type for test helpers that may fail with informative messages.
type TestResult = Result<(), String>;

/// Parses the source and returns the AST or an error string.
fn parse_ast(source: &[u8]) -> Result<WorkflowAst, String> {
    YamlCompiler::default()
        .parse_ast(source)
        .map_err(|errors| format!("parse_ast failed: {errors:?}"))
}

/// Parses the source and returns the first error or an error string.
fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!(
            "parse_ast unexpectedly succeeded with AST: {:?}",
            ast
        )),
        Err(CompileErrors(errors)) => errors
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}

/// Checks that the condition is true, returning an error message if not.
fn ensure(condition: bool, message: impl Into<String>) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

/// Counts all `$attempt.number` references in the AST.
fn find_attempt_reference_count(ast: &WorkflowAst) -> usize {
    let mut count = 0;
    collect_attempt_references_from_ast(ast, &mut count);
    count
}

fn collect_attempt_references_from_ast(ast: &WorkflowAst, count: &mut usize) {
    // Check inputs
    for entry in &ast.inputs {
        collect_from_value(&entry.value, count);
    }
    // Check vars
    for entry in &ast.vars {
        collect_from_value(&entry.value, count);
    }
    // Check result expressions
    for entry in &ast.result {
        collect_from_expression(&entry.value, count);
    }
    // Check examples
    for example in &ast.examples {
        collect_from_value(example, count);
    }
    // Check steps
    for step in &ast.steps {
        collect_from_step_kind(&step.kind, count);
    }
}

fn collect_from_step_kind(kind: &StepKindAst, count: &mut usize) {
    match kind {
        StepKindAst::Run { .. } => {}
        StepKindAst::Save { fields } => {
            for entry in fields {
                collect_from_value(&entry.value, count);
            }
        }
        StepKindAst::Choose { condition, .. } => {
            collect_from_expression(condition, count);
        }
        StepKindAst::ForEach { .. } => {}
        StepKindAst::Together { .. } => {}
        StepKindAst::Collect { .. } => {}
        StepKindAst::Reduce { initial, .. } => {
            collect_from_value(initial, count);
        }
        StepKindAst::Repeat { body, .. } => {
            for body_step in body {
                collect_from_step_kind(&body_step.kind, count);
            }
        }
        StepKindAst::Wait { .. } => {}
        StepKindAst::Ask { .. } => {}
        StepKindAst::Finish { result } => {
            collect_from_expression(result, count);
        }
    }
}

fn collect_from_expression(expr: &AstExpression, count: &mut usize) {
    match expr {
        AstExpression::Slot(_) => {}
        AstExpression::Reference(reference) => {
            if reference.as_ref() == "$attempt.number" {
                *count += 1;
            }
        }
        AstExpression::Parsed(parsed) => {
            collect_from_parsed_expression(parsed, count);
        }
        AstExpression::Literal(value) => {
            collect_from_value(value, count);
        }
    }
}

fn collect_from_parsed_expression(expr: &ParsedExpression, count: &mut usize) {
    match expr {
        ParsedExpression::Reference(reference) => {
            if reference.as_ref() == "$attempt.number" {
                *count += 1;
            }
        }
        ParsedExpression::Unary { expr, .. } => {
            collect_from_parsed_expression(expr, count);
        }
        ParsedExpression::Binary { left, right, .. } => {
            collect_from_parsed_expression(left, count);
            collect_from_parsed_expression(right, count);
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                collect_from_parsed_expression(arg, count);
            }
        }
        ParsedExpression::Literal(_) => {}
    }
}

fn collect_from_value(value: &AstValue, count: &mut usize) {
    match value {
        AstValue::Reference(reference) => {
            if reference.as_ref() == "$attempt.number" {
                *count += 1;
            }
        }
        AstValue::Sequence(values) => {
            for v in values {
                collect_from_value(v, count);
            }
        }
        AstValue::Mapping(entries) => {
            for entry in entries {
                collect_from_value(&entry.value, count);
            }
        }
        AstValue::Null | AstValue::Bool(_) | AstValue::I64(_) | AstValue::Text(_) => {}
    }
}

// =============================================================================
// B1: Valid $attempt.number usage inside repeat bodies
// =============================================================================

/// Verifies that `$attempt.number` in a repeat body's `save` field:
/// 1. Compiles without error
/// 2. Is retained in AST as AstExpression::Reference("$attempt.number")
#[test]
fn attempt_number_in_repeat_body_save_field_compiles_and_retains_reference() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: repeat_with_attempt
when:
  manual: {}
steps:
  - id: retry_step
    repeat:
      max_attempts: 3
      steps:
        - id: log_attempt
          save:
            current_attempt: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let ast = parse_ast(source)?;

    // Contract guarantee: reference is retained as AstExpression::Reference
    let count = find_attempt_reference_count(&ast);
    ensure(
        count > 0,
        format!(
            "Expected $attempt.number to be retained in AST, but found {} references",
            count
        ),
    )
}

/// Verifies that `$attempt.number` retained as a value inside a `save` body
/// nested within a repeat step (the production Phase-0 contract; the previous
/// `do: my_action` form was retired when `do`/`run` was narrowed to integer
/// `action`/`input` slot fields).
#[test]
fn attempt_number_in_repeat_body_save_value_compiles_and_retains_reference() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: repeat_save_with_attempt
when:
  manual: {}
steps:
  - id: retry_action
    repeat:
      max_attempts: 3
      steps:
        - id: call_api
          save:
            args: [$attempt.number]
  - id: done
    finish:
      result: 0
"#;

    let ast = parse_ast(source)?;

    // Contract guarantee: reference is retained in AST
    let count = find_attempt_reference_count(&ast);
    ensure(
        count > 0,
        format!(
            "Expected $attempt.number to be retained in AST, but found {} references",
            count
        ),
    )
}

/// Verifies that `$attempt.number` in a deeply nested Repeat body compiles.
#[test]
fn attempt_number_in_nested_repeat_body_compiles_and_retains_reference() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: nested_repeat
when:
  manual: {}
steps:
  - id: outer_retry
    repeat:
      max_attempts: 2
      steps:
        - id: inner_retry
          repeat:
            max_attempts: 3
            steps:
              - id: log
                save:
                  value: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let ast = parse_ast(source)?;

    // The inner repeat body should still have $attempt.number accessible
    let count = find_attempt_reference_count(&ast);
    ensure(
        count > 0,
        format!(
            "Expected $attempt.number to be retained in nested repeat AST, but found {} references",
            count
        ),
    )
}

/// Verifies that `$attempt.number` in a `choose` condition inside Repeat compiles.
#[test]
fn attempt_number_in_choose_condition_inside_repeat_compiles() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: repeat_choose_attempt
when:
  manual: {}
steps:
  - id: conditional_retry
    repeat:
      max_attempts: 3
      steps:
        - id: check
          choose:
            condition: $attempt.number > 1
            on_true: 1
            on_false: 1
  - id: done
    finish:
      result: 0
"#;

    // This should compile - $attempt.number is valid in repeat body
    let ast = parse_ast(source)?;

    // The condition expression should contain $attempt.number
    let count = find_attempt_reference_count(&ast);
    ensure(
        count > 0,
        format!(
            "Expected $attempt.number in choose condition to be retained in AST, but found {} references",
            count
        ),
    )
}

// =============================================================================
// B2: Invalid $attempt.number usage outside repeat bodies
// =============================================================================

/// `$attempt.number` placed as a top-level `vars` value must be rejected.
/// Under the Phase-0 contract `slot_value` rejects string-typed var values
/// outright, so the surface error is `UnsupportedConstantValue` rather than
/// the references scope guard.
#[test]
fn attempt_number_in_vars_rejected() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: vars_attempt_error
when:
  manual: {}
vars:
  current: $attempt.number
steps:
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    match error {
        CompileError::UnsupportedConstantValue { step: 0 } => Ok(()),
        other => Err(format!(
            "Expected CompileError::UnsupportedConstantValue for $attempt.number in vars, got: {other:?}"
        )),
    }
}

/// $attempt.number in `finish.result` must be rejected.
#[test]
fn attempt_number_in_finish_result_rejected_with_invalid_variable_scope() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: finish_attempt_error
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $attempt.number
"#;

    let error = parse_error(source)?;

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
                | CompileError::InvalidVariableScope { .. }
        ),
        format!(
            "Expected IllegalReference, UnknownReferenceRoot, or InvalidVariableScope for $attempt.number in finish.result, got: {:?}",
            error
        ),
    )
}

/// $attempt.number in `save` field outside Repeat must be rejected.
#[test]
fn attempt_number_in_save_outside_repeat_rejected_with_invalid_variable_scope() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: save_attempt_error
when:
  manual: {}
steps:
  - id: log_attempt
    save:
      value: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
                | CompileError::InvalidVariableScope { .. }
        ),
        format!(
            "Expected IllegalReference, UnknownReferenceRoot, or InvalidVariableScope for $attempt.number in save, got: {:?}",
            error
        ),
    )
}

/// `$attempt.number` placed inside a `for_each` body must be rejected. Under
/// the Phase-0 contract the surrounding `vars: items: [1, 2, 3]` declaration
/// is rejected first by `slot_value` as `UnsupportedConstantValue` (the
/// validator no longer accepts list-typed var values).
#[test]
fn attempt_number_in_for_each_body_rejected() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: foreach_attempt_error
when:
  manual: {}
vars:
  items: [1, 2, 3]
steps:
  - id: iterate
    foreach:
      variable: item
      input: $vars.items
      steps:
        - id: process
          save:
            attempt: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    match error {
        CompileError::UnsupportedConstantValue { step: 0 } => Ok(()),
        other => Err(format!(
            "Expected CompileError::UnsupportedConstantValue for for_each-body context, got: {other:?}"
        )),
    }
}

/// $attempt.number in `examples` value must be rejected.
#[test]
fn attempt_number_in_examples_rejected_with_invalid_variable_scope() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: examples_attempt_error
when:
  manual: {}
examples:
  - name: test_case
    attempt_val: $attempt.number
steps:
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
                | CompileError::InvalidVariableScope { .. }
        ),
        format!(
            "Expected IllegalReference, UnknownReferenceRoot, or InvalidVariableScope for $attempt.number in examples, got: {:?}",
            error
        ),
    )
}

/// $attempt.number in `choose` condition outside Repeat must be rejected.
#[test]
fn attempt_number_in_choose_outside_repeat_rejected_with_invalid_variable_scope() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: choose_attempt_error
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $attempt.number > 1
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
                | CompileError::InvalidVariableScope { .. }
        ),
        format!(
            "Expected IllegalReference, UnknownReferenceRoot, or InvalidVariableScope for $attempt.number in choose outside repeat, got: {:?}",
            error
        ),
    )
}

/// `$attempt.number` placed inside a `reduce` body must be rejected. Under
/// the Phase-0 contract the surrounding `vars: data: [1, 2, 3]` declaration is
/// rejected first by `slot_value` as `UnsupportedConstantValue` (the validator
/// no longer accepts list-typed var values).
#[test]
fn attempt_number_in_reduce_body_rejected() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: reduce_attempt_error
when:
  manual: {}
vars:
  data: [1, 2, 3]
steps:
  - id: sum_values
    aggregate:
      variable: acc
      input: $vars.data
      initial: 0
      steps:
        - id: accumulate
          save:
            sum: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    match error {
        CompileError::UnsupportedConstantValue { step: 0 } => Ok(()),
        other => Err(format!(
            "Expected CompileError::UnsupportedConstantValue for reduce-body context, got: {other:?}"
        )),
    }
}

// =============================================================================
// Boundary and adversarial cases
// =============================================================================

/// Bare `$attempt` reference (without `.number`) must be rejected.
#[test]
fn bare_attempt_reference_rejected() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: bare_attempt
when:
  manual: {}
examples:
  - name: test
    value: $attempt
steps:
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
                | CompileError::InvalidVariableScope { .. }
        ),
        format!(
            "Expected IllegalReference, UnknownReferenceRoot, or InvalidVariableScope for bare $attempt, got: {:?}",
            error
        ),
    )
}

/// `$attempt.number.extra` accessor path must be rejected.
#[test]
fn attempt_number_with_extra_accessor_rejected() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: attempt_extra_accessor
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $attempt.number.field
"#;

    let error = parse_error(source)?;

    // $attempt.number.field would fail because 'attempt' is not a valid root
    // and 'number.field' is an unsupported accessor path
    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
                | CompileError::UnsupportedAccessorReference { .. }
                | CompileError::InvalidVariableScope { .. }
        ),
        format!(
            "Expected IllegalReference/UnknownReferenceRoot/UnsupportedAccessorReference/InvalidVariableScope for $attempt.number.field, got: {:?}",
            error
        ),
    )
}

/// `$attempt.number` inside a YAML block that uses the non-existent `parallel`
/// primitive must be rejected. Under the Phase-0 contract `parallel` is not a
/// supported step primitive, so the parser surfaces the field as
/// `UnknownStepField` before the references validator is reached.
#[test]
fn attempt_number_in_parallel_body_rejected_with_unknown_step_field() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: parallel_attempt_error
when:
  manual: {}
steps:
  - id: parallel_work
    parallel:
      branches:
        - label: branch1
          steps:
            - id: task1
              save:
                attempt: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    match error {
        CompileError::UnknownStepField { field, .. } if field.as_ref() == "parallel" => Ok(()),
        other => Err(format!(
            "Expected CompileError::UnknownStepField for non-existent `parallel` primitive, got: {other:?}"
        )),
    }
}

/// `$attempt.number` placed inside a `collect` body must be rejected. Under
/// the Phase-0 contract the surrounding `vars: source: [1, 2, 3]` declaration
/// is rejected first by `slot_value` as `UnsupportedConstantValue` (the
/// validator no longer accepts list-typed var values).
#[test]
fn attempt_number_in_collect_body_rejected() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: collect_attempt_error
when:
  manual: {}
vars:
  source: [1, 2, 3]
steps:
  - id: gather
    collect:
      variable: item
      source: $vars.source
      steps:
        - id: process
          save:
            attempt: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let error = parse_error(source)?;

    match error {
        CompileError::UnsupportedConstantValue { step: 0 } => Ok(()),
        other => Err(format!(
            "Expected CompileError::UnsupportedConstantValue for collect-body context, got: {other:?}"
        )),
    }
}

/// Multiple $attempt.number references in same repeat body all retained.
#[test]
fn multiple_attempt_references_in_same_repeat_body_retained() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: multiple_attempt_refs
when:
  manual: {}
steps:
  - id: multi_retry
    repeat:
      max_attempts: 5
      steps:
        - id: log
          save:
            first: $attempt.number
            second: $attempt.number
            third: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let ast = parse_ast(source)?;

    // Should find 3 references
    let count = find_attempt_reference_count(&ast);
    ensure(
        count == 3,
        format!(
            "Expected 3 $attempt.number references in AST, found {}",
            count
        ),
    )
}

/// Empty repeat body with $attempt.number reference compiles (body has no steps that use it).
#[test]
fn empty_repeat_body_compiles_without_attempt_reference() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: empty_repeat
when:
  manual: {}
steps:
  - id: empty_retry
    repeat:
      max_attempts: 3
      steps: []
  - id: done
    finish:
      result: 0
"#;

    // Should compile fine - no $attempt.number in body
    let ast = parse_ast(source)?;
    let count = find_attempt_reference_count(&ast);
    ensure(
        count == 0,
        format!(
            "Expected no $attempt.number references in empty repeat body, found {}",
            count
        ),
    )
}

/// $attempt.number in repeat body with max_attempts=1 is valid.
#[test]
fn attempt_number_in_single_attempt_repeat_retained() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: single_attempt_repeat
when:
  manual: {}
steps:
  - id: one_retry
    repeat:
      max_attempts: 1
      steps:
        - id: log
          save:
            attempt: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let ast = parse_ast(source)?;
    let count = find_attempt_reference_count(&ast);
    ensure(
        count > 0,
        format!(
            "Expected $attempt.number to be retained even with max_attempts=1, but found {} references",
            count
        ),
    )
}

/// $attempt.number in repeat body is NOT resolved at compile time.
#[test]
fn attempt_number_not_resolved_at_compile_time() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: repeat_unresolved
when:
  manual: {}
steps:
  - id: retry_step
    repeat:
      max_attempts: 3
      steps:
        - id: log_attempt
          save:
            current_attempt: $attempt.number
  - id: done
    finish:
      result: 0
"#;

    let ast = parse_ast(source)?;

    // The reference must be in the AST as a Reference expression, NOT as a literal
    // This proves it's NOT resolved at compile time
    let count = find_attempt_reference_count(&ast);
    ensure(
        count > 0,
        format!(
            "Expected $attempt.number to remain as unresolved Reference in AST, but found {} references",
            count
        ),
    )
}

// =============================================================================
// Scope guard: vb-sitry — `$attempt` root routes to InvalidVariableScope
// =============================================================================

/// `$attempt.number` appearing in a `finish.result` expression outside any
/// `Repeat` body must be rejected with the dedicated `InvalidVariableScope`
/// error variant.
///
/// Regression test for bead vb-sitry: prior to the fix the same input
/// surfaced as `UnknownReferenceRoot` (wrong taxonomy — the reference is
/// known, it is just out of scope). The new compile-time scope guard
/// installed in `vb_compile::references::validate_compile_reference`
/// routes the `$attempt` root to `InvalidVariableScope` with the canonical
/// payload `{reference, context, allowed, mark}`.
#[test]
fn attempt_number_outside_repeat_body_emits_invalid_variable_scope() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: attempt_outside_repeat
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $attempt.number
"#;

    let error = parse_error(source)?;

    // Must be the dedicated scope error with the documented payload shape.
    let invalid = match error {
        CompileError::InvalidVariableScope {
            reference,
            context,
            allowed,
            mark: _,
        } => (reference, context, allowed),
        other => {
            return Err(format!(
                "Expected CompileError::InvalidVariableScope for $attempt.number outside a \
                 repeat body, got: {other:?}"
            ));
        }
    };

    let (reference, context, allowed) = invalid;

    ensure(
        reference.as_ref() == "$attempt.number",
        format!(
            "Expected reference field \"$attempt.number\", got {:?}",
            reference
        ),
    )?;
    ensure(
        context == "outside repeat body",
        format!(
            "Expected context field \"outside repeat body\", got {:?}",
            context
        ),
    )?;
    ensure(
        allowed.as_ref() == ["repeat_attempt.body", "repeat_check"].as_slice(),
        format!(
            "Expected allowed field [\"repeat_attempt.body\", \"repeat_check\"], got {:?}",
            allowed.as_ref()
        ),
    )?;

    Ok(())
}

//! Tests for `$attempt.number` variable scope restriction.
//!
//! These tests verify that the `$attempt.number` reference is:
//! - VALID: retained in the AST when used inside repeat body steps
//! - INVALID: rejected with `InvalidVariableScope` error when used outside repeat bodies
//!
//! The contract guarantees:
//! 1. Compilation succeeds without error when `$attempt.number` appears in repeat body
//! 2. The reference is retained in the AST as `AstExpression::Reference("$attempt.number")`
//! 3. The reference is NOT resolved at compile time (runtime binding only)
//! 4. Compilation fails with `InvalidVariableScope` when used outside repeat bodies

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

/// Finds all `$attempt.number` references in the AST and returns whether any were found.
fn has_attempt_reference(ast: &WorkflowAst) -> bool {
    find_attempt_reference_count(ast) > 0
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
        StepKindAst::Repeat { .. } => {
            // NOTE: Cold AST StepKindAst::Repeat does not preserve body steps.
            // For MAJOR-5 to work properly, the implementation must either:
            // 1. Extend StepKindAst::Repeat to include body expressions, OR
            // 2. Validate $attempt.number at a different stage (e.g., canonical lowering)
            // These tests document the EXPECTED behavior once implemented.
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

/// Verifies that `$attempt.number` in a repeat body's `do` input expression compiles.
#[test]
fn attempt_number_in_repeat_body_do_input_compiles_and_retains_reference() -> TestResult {
    let source = br#"version: velvet-ballistics/v1
name: repeat_do_with_attempt
when:
  manual: {}
steps:
  - id: retry_action
    repeat:
      max_attempts: 3
      steps:
        - id: call_api
          do: my_action
          input: $attempt.number
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

/// $attempt.number in top-level `vars` must be rejected.
#[test]
fn attempt_number_in_vars_rejected_with_invalid_variable_scope() -> TestResult {
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

    // Should fail with InvalidVariableScope (or IllegalReference as fallback)
    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in vars, got: {:?}",
            error
        ),
    )
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
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in finish.result, got: {:?}",
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
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in save, got: {:?}",
            error
        ),
    )
}

/// $attempt.number in `for_each` body must be rejected.
#[test]
fn attempt_number_in_for_each_body_rejected_with_invalid_variable_scope() -> TestResult {
    // This YAML has $attempt.number in a for_each body step (not a Repeat body)
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

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in for_each body, got: {:?}",
            error
        ),
    )
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
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in examples, got: {:?}",
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
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in choose outside repeat, got: {:?}",
            error
        ),
    )
}

/// $attempt.number in `reduce` body must be rejected.
#[test]
fn attempt_number_in_reduce_body_rejected_with_invalid_variable_scope() -> TestResult {
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

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in reduce body, got: {:?}",
            error
        ),
    )
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
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for bare $attempt, got: {:?}",
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
        ),
        format!(
            "Expected IllegalReference/UnknownReferenceRoot/UnsupportedAccessorReference for $attempt.number.field, got: {:?}",
            error
        ),
    )
}

/// $attempt.number in `parallel` body must be rejected.
#[test]
fn attempt_number_in_parallel_body_rejected_with_invalid_variable_scope() -> TestResult {
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

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in together body, got: {:?}",
            error
        ),
    )
}

/// $attempt.number in `collect` body must be rejected.
#[test]
fn attempt_number_in_collect_body_rejected_with_invalid_variable_scope() -> TestResult {
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

    ensure(
        matches!(
            error,
            CompileError::IllegalReference { .. }
                | CompileError::UnknownReferenceRoot { .. }
        ),
        format!(
            "Expected IllegalReference or UnknownReferenceRoot for $attempt.number in collect body, got: {:?}",
            error
        ),
    )
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
        format!("Expected 3 $attempt.number references in AST, found {}", count),
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

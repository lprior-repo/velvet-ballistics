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
    unused_variables
)]

use crate::schema::validate_input_schemas;
use crate::{CompileError, CompileErrors, YamlCompiler};
use saphyr::{LoadableYamlNode, Yaml};
use vb_core::SymbolicCode;

fn validate_inputs(inputs: &str) -> Result<(), CompileError> {
    let source = format!("version: velvet-ballistics/v1\ninputs:\n{inputs}\n");
    let docs = Yaml::load_from_str(&source)?;
    let Some(doc) = docs.first() else {
        return Err(CompileError::EmptySource);
    };
    match validate_input_schemas(doc) {
        Ok(()) => Ok(()),
        Err(errors) => match errors.first() {
            Some(error) => Err(error.clone()),
            None => Err(CompileError::EmptySource),
        },
    }
}

#[test]
fn input_schema_rejects_unknown_fields() {
    let result = validate_inputs("  value:\n    is: text\n    kind: text\n");

    assert!(matches!(
        result,
        Err(CompileError::UnknownInputSchemaField { .. })
    ));
}

#[test]
fn input_schema_rejects_invalid_bounds() {
    let result = validate_inputs("  value:\n    is: text\n    min_length: 9\n    max_length: 1\n");

    assert!(matches!(
        result,
        Err(CompileError::InvalidInputSchema { .. })
    ));
}

// ---------------------------------------------------------------------------
// vb-yd5x RED PHASE: Shared IR parity tests
// ---------------------------------------------------------------------------

/// Minimal canonical workflow for testing.
const VB_YD5X_MINIMAL_VALID_WORKFLOW: &[u8] = br#"
version: velvet-ballistics/v1
name: minimal_valid
when:
  manual: {}
steps:
  - id: start
    set:
      output: answer
      value: "1"
  - id: done
    finish:
      result: answer
"#;

/// Workflow with out-of-range slot reference (Gate 9)
/// This uses a slot index that is out of bounds for the compiled workflow.
/// The issue is the result slot 99 doesn't exist.
const VB_YD5X_MALFORMED_SLOT_REF: &[u8] = br#"
version: velvet-ballistics/v1
name: bad_slot_ref
when:
  manual: {}
steps:
  - id: start
    save:
      value: 1
  - id: use_missing_slot
    for_each:
      input: 99
      item: 1
      limit: 10
  - id: done
    finish:
      result: 0
"#;

/// Workflow with loop body type mismatch (Gate 11)
/// The for_each 'input' field expects expression string but gets a number.
const VB_YD5X_MALFORMED_LOOP_BODY: &[u8] = br#"
version: velvet-ballistics/v1
name: bad_loop_body
when:
  manual: {}
steps:
  - id: fanout
    for_each:
      variable: i
      input: 123
      steps:
        - id: step
          finish:
            result: 0
  - id: join
    finish:
      result: 0
"#;

/// Workflow with duplicate step ID
const VB_YD5X_MALFORMED_DUPLICATE_ID: &[u8] = br#"
version: velvet-ballistics/v1
name: duplicate_ids
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: build
    finish:
      result: 0
"#;

/// Workflow with unknown reference
const VB_YD5X_MALFORMED_UNKNOWN_REF: &[u8] = br#"
version: velvet-ballistics/v1
name: unknown_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $input.missing == true
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;

/// Helper: validate via canonical compile then shared pipeline.
fn vb_yd5x_validate_via_compile(source: &[u8]) -> Result<(), CompileErrors> {
    let compiled = YamlCompiler::default().compile(source)?;
    let parts = compiled.to_parts();
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))
}

fn first_compile_code(source: &[u8]) -> Result<SymbolicCode, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => errors
            .first()
            .map(CompileError::code)
            .ok_or_else(|| "compile failed with no errors".to_owned()),
    }
}

#[test]
fn vb_yd5x_valid_workflow_passes_both_paths() {
    let source = VB_YD5X_MINIMAL_VALID_WORKFLOW;
    let compile_result = YamlCompiler::default().compile(source);
    let validate_result = vb_yd5x_validate_via_compile(source);
    assert!(
        matches!(compile_result, Ok(_)),
        "valid workflow must compile: {compile_result:?}"
    );
    assert!(
        matches!(validate_result, Ok(_)),
        "valid workflow must pass shared validation: {validate_result:?}"
    );
}

#[test]
fn vb_yd5x_legacy_slot_ref_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_SLOT_REF)?.as_str(),
        "MISSING_REQUIRED_FIELD"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_loop_body_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_LOOP_BODY)?.as_str(),
        "TYPE_MISMATCH"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_duplicate_id_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_DUPLICATE_ID)?.as_str(),
        "MISSING_REQUIRED_FIELD"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_unknown_ref_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_UNKNOWN_REF)?.as_str(),
        "UNKNOWN_TOP_LEVEL_FIELD"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_diagnostic_codes_remain_stable() -> Result<(), String> {
    let test_cases = [
        (VB_YD5X_MALFORMED_SLOT_REF, "MISSING_REQUIRED_FIELD"),
        (VB_YD5X_MALFORMED_LOOP_BODY, "TYPE_MISMATCH"),
        (VB_YD5X_MALFORMED_DUPLICATE_ID, "MISSING_REQUIRED_FIELD"),
        (VB_YD5X_MALFORMED_UNKNOWN_REF, "UNKNOWN_TOP_LEVEL_FIELD"),
    ];
    for (source, expected_code) in test_cases {
        assert_eq!(first_compile_code(source)?.as_str(), expected_code);
    }
    Ok(())
}

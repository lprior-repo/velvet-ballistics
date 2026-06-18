//! Tests for the verification pipeline (parse → compile → validate → gates).

#![forbid(unsafe_code)]
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
    clippy::io_other_error,
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
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
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
    clippy::new_without_default,
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
    unused_variables
)]

use crate::args::{DurabilityMode, VerifyProfile};
use crate::commands_verify::types::{VerifyError, VerifyOk};
use crate::commands_verify::run_verification;
use crate::commands_verify::exit_code_for_error;

const MINIMAL_WORKFLOW_YAML: &str =
    include_str!("../../../workspace_tests/tests/fixtures/valid/minimal.yaml");
const UNSUPPORTED_TOP_LEVEL_INPUTS_YAML: &str = r#"version: velvet-ballistics/v1
name: compile_scope_failure
when:
  manual: {}
inputs:
  count:
    type: u32
steps:
  - id: done
    finish:
      result: 0
"#;

const QUICK_PROFILE_EXPECTED_CHECKS: [&str; 15] = [
    "profile", "shape", "names", "references", "expressions", "CFG",
    "bounded:deferred", "budgets:deferred", "contracts:deferred",
    "taint:deferred", "idempotency:deferred", "durability:deferred",
    "capabilities:deferred", "results", "evidence:deferred",
];

const FULL_PROFILE_EXPECTED_CHECKS: [&str; 15] = [
    "profile", "shape", "names", "references", "expressions", "CFG",
    "bounded", "budgets", "contracts:deferred", "taint:deferred",
    "idempotency:deferred", "durability:deferred", "capabilities:deferred",
    "results", "evidence:deferred",
];

fn expect_success(result: Result<VerifyOk, VerifyError>) -> VerifyOk {
    match result {
        Ok(ok) => ok,
        Err(err) => panic!("expected verification success, got {err:?}"),
    }
}

fn expect_deferred_failure(result: Result<VerifyOk, VerifyError>) -> VerifyOk {
    match result {
        Err(VerifyError::DeferredGates(ok)) => ok,
        Err(err) => panic!("expected deferred-gates failure, got {err:?}"),
        Ok(ok) => panic!("expected deferred-gates failure, got success {ok:?}"),
    }
}

#[test]
fn malformed_yaml_returns_yaml_parse_error() {
    let result = run_verification("version: [", b"version: [", VerifyProfile::Quick, DurabilityMode::None);
    match result {
        Err(VerifyError::YamlParse(message)) => assert!(message.contains("YAML parse error")),
        Err(err) => panic!("expected YAML parse error, got {err:?}"),
        Ok(ok) => panic!("expected YAML parse error, got success {ok:?}"),
    }
}

#[test]
fn invalid_workflow_returns_compile_error() {
    let result = run_verification(
        UNSUPPORTED_TOP_LEVEL_INPUTS_YAML,
        UNSUPPORTED_TOP_LEVEL_INPUTS_YAML.as_bytes(),
        VerifyProfile::Quick,
        DurabilityMode::None,
    );
    match result {
        Err(VerifyError::Compile(errors)) => {
            assert!(!errors.is_empty());
            assert!(errors.iter().any(|e| e.contains("inputs")));
        }
        Err(err) => panic!("expected compile error, got {err:?}"),
        Ok(ok) => panic!("expected compile error, got success {ok:?}"),
    }
}

#[test]
fn quick_profile_reports_master_gate_names_in_order() {
    let ok = expect_success(run_verification(
        MINIMAL_WORKFLOW_YAML,
        MINIMAL_WORKFLOW_YAML.as_bytes(),
        VerifyProfile::Quick,
        DurabilityMode::None,
    ));
    assert_eq!(ok.checks, QUICK_PROFILE_EXPECTED_CHECKS);
}

#[test]
fn standard_profile_succeeds_with_deferred_gates_and_warnings() {
    let ok = expect_success(run_verification(
        MINIMAL_WORKFLOW_YAML,
        MINIMAL_WORKFLOW_YAML.as_bytes(),
        VerifyProfile::Standard,
        DurabilityMode::None,
    ));
    assert_eq!(ok.checks, FULL_PROFILE_EXPECTED_CHECKS);
    assert!(!ok.all_gates_closed());
    assert_eq!(
        ok.deferred_gates(),
        vec!["contracts", "taint", "idempotency", "durability", "capabilities", "evidence"]
    );
    assert!(ok.warnings.iter().any(|w| w.contains(
        "compiled-form WorkflowParts taint validation is not implemented"
    )));
}

#[test]
fn full_profile_fails_closed_when_deferred_gates_remain() {
    let ok = expect_deferred_failure(run_verification(
        MINIMAL_WORKFLOW_YAML,
        MINIMAL_WORKFLOW_YAML.as_bytes(),
        VerifyProfile::Full,
        DurabilityMode::None,
    ));
    assert_eq!(ok.checks, FULL_PROFILE_EXPECTED_CHECKS);
    assert!(!ok.all_gates_closed());
    assert!(ok.passed_gates().contains(&"bounded"));
    assert!(ok.deferred_gates().contains(&"evidence"));
}

#[test]
fn success_path_records_digest_node_count_and_durability() {
    let compiled = vb_compile::compile_workflow(MINIMAL_WORKFLOW_YAML.as_bytes())
        .expect("fixture must compile");
    let expected_digest: String = compiled.digest().as_bytes().iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let ok = expect_success(run_verification(
        MINIMAL_WORKFLOW_YAML,
        MINIMAL_WORKFLOW_YAML.as_bytes(),
        VerifyProfile::Quick,
        DurabilityMode::Strict,
    ));
    assert_eq!(ok.digest_hex, expected_digest);
    assert_eq!(ok.node_count, compiled.node_count());
    assert_eq!(ok.durability_mode, DurabilityMode::Strict);
}

#[test]
fn deferred_profile_omits_fabricated_gate_names() {
    let forbidden = [
        "digest_stability", "resource_contract_validation",
        "error_handler_completeness", "taint_boundary", "input_purity",
        "expression_complexity", "cycle_detection", "determinism_seed",
        "replay_round_trip",
    ];
    let ok = expect_deferred_failure(run_verification(
        MINIMAL_WORKFLOW_YAML,
        MINIMAL_WORKFLOW_YAML.as_bytes(),
        VerifyProfile::Full,
        DurabilityMode::None,
    ));
    for gate in forbidden {
        assert!(!ok.checks.contains(&gate));
    }
}

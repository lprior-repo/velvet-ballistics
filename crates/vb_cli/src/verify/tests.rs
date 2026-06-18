//! Tests for the verify command module.
//!
//! These tests exercise the public surface of `verify/`: command entry
//! points, report builders, error formatters, and the durability pipeline
//! integration.

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

use std::path::PathBuf;
use std::process::ExitCode;

use crate::args::{DurabilityMode, LegacyJsonOutput, OutputFormat, VerifyProfile};
use crate::commands_verify::{VerifyError, VerifyOk};
use crate::exit_code::CliExitCode;

use super::command::cmd_verify_with_durability;
use super::error::{cli_exit_code_number, human_verify_error_lines, verify_error_message};
use super::report::{
    verification_completion_message, verify_deferred_report, verify_success_report,
};
use crate::commands_verify::exit_code_for_error;

const VERIFY_HELPER_ENV: &str = "VB_VERIFY_DURABILITY_HELPER";
const VERIFY_WORKFLOW_ENV: &str = "VB_VERIFY_DURABILITY_WORKFLOW";
const VERIFY_HELPER_TEST: &str =
    "verify::tests::cmd_verify_with_durability_helper_emits_machine_report";

fn sample_result_with_durability(
    checks: Vec<&'static str>,
    durability_mode: DurabilityMode,
) -> VerifyOk {
    VerifyOk {
        digest_hex: "0123456789abcdef".repeat(4),
        ir_digest_hex: "fedcba9876543210".repeat(4),
        node_count: 2,
        checks,
        warnings: vec!["taint warning: not implemented".to_string()],
        durability_mode,
    }
}

fn sample_result(checks: Vec<&'static str>) -> VerifyOk {
    sample_result_with_durability(checks, DurabilityMode::Journaled)
}

fn fixture_path(relative: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root_fixture = root.join(relative);
    if root_fixture.exists() {
        return Ok(root_fixture);
    }

    let workspace_fixture = root.join("crates/workspace_tests").join(relative);
    if workspace_fixture.exists() {
        Ok(workspace_fixture)
    } else {
        Err(format!(
            "missing fixture {relative} under {} or {}",
            root_fixture.display(),
            workspace_fixture.display()
        ))
    }
}

fn parse_machine_report_line(stdout: &str) -> Result<serde_json::Value, String> {
    for line in stdout.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            return Ok(value);
        }
    }
    Err(format!(
        "expected one JSON machine-report line in helper stdout, got:\n{stdout}"
    ))
}

fn json_string_vec(value: &serde_json::Value, pointer: &str) -> Vec<String> {
    match value.pointer(pointer).and_then(serde_json::Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(std::string::ToString::to_string)
            .collect(),
        None => panic!("missing string array at {pointer}"),
    }
}

// --- Tests ---

#[test]
fn success_report_keeps_statuses_and_splits_deferred_gates() {
    let result = sample_result(vec![
        "profile",
        "shape",
        "bounded",
        "contracts:deferred",
        "results",
        "evidence:deferred",
    ]);
    let report = verify_success_report(&result, VerifyProfile::Standard);

    assert_eq!(
        json_string_vec(&report, "/checks"),
        vec![
            "profile",
            "shape",
            "bounded",
            "contracts:deferred",
            "results",
            "evidence:deferred",
        ]
    );
    assert_eq!(
        json_string_vec(&report, "/passed_checks"),
        vec!["profile", "shape", "bounded", "results"]
    );
    assert_eq!(
        json_string_vec(&report, "/deferred_checks"),
        vec!["contracts", "evidence"]
    );
    assert_eq!(
        report
            .pointer("/all_gates_closed")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json_string_vec(&report, "/replay/gates_passed"),
        vec!["profile", "shape", "bounded", "results"]
    );
    assert_eq!(
        report
            .pointer("/artifact/ir_digest_hex")
            .and_then(serde_json::Value::as_str),
        Some("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")
    );
    assert_eq!(
        report
            .pointer("/replay/replay_safe")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
}

#[test]
fn deferred_report_returns_failure_without_losing_gate_statuses() {
    let result = sample_result(vec![
        "profile",
        "shape",
        "bounded",
        "contracts:deferred",
        "results",
        "evidence:deferred",
    ]);
    let report = verify_deferred_report(
        &result,
        VerifyProfile::Full,
        CliExitCode::VerificationFailed,
    );

    assert_eq!(
        report
            .pointer("/success")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json_string_vec(&report, "/checks"),
        vec![
            "profile",
            "shape",
            "bounded",
            "contracts:deferred",
            "results",
            "evidence:deferred",
        ]
    );
    assert_eq!(
        report.pointer("/error").and_then(serde_json::Value::as_str),
        Some("full verification blocked: deferred gates remain: contracts, evidence")
    );
}

#[test]
fn human_verify_error_lines_formats_compile_errors() {
    let lines = human_verify_error_lines(&VerifyError::Compile(vec![
        String::from("first compile failure"),
        String::from("second compile failure"),
    ]));

    assert_eq!(
        lines,
        vec![
            String::from("compile error: first compile failure"),
            String::from("compile error: second compile failure"),
        ]
    );
    assert_eq!(
        verify_error_message(&VerifyError::Compile(vec![
            String::from("first compile failure"),
            String::from("second compile failure"),
        ])),
        String::from("compilation failed:\n  first compile failure\n  second compile failure\n")
    );
}

#[test]
fn remaining_verify_error_variants_keep_public_text_and_exit_codes() {
    let cases = vec![
        (
            VerifyError::IrValidation(String::from("bad ir")),
            vec![String::from("bad ir")],
            String::from("IR validation error: bad ir"),
            CliExitCode::VerificationFailed,
        ),
        (
            VerifyError::BudgetPolicy(String::from("over budget")),
            vec![String::from("over budget")],
            String::from("budget policy violation: over budget"),
            CliExitCode::VerificationFailed,
        ),
        (
            VerifyError::StorageError(String::from("disk full")),
            vec![String::from("disk full")],
            String::from("storage error: disk full"),
            CliExitCode::StorageError,
        ),
        (
            VerifyError::ReplayDivergence(String::from("state diverged")),
            vec![String::from("state diverged")],
            String::from("replay divergence: state diverged"),
            CliExitCode::ReplayDivergence,
        ),
    ];

    for (error, expected_lines, expected_message, expected_code) in cases {
        assert_eq!(human_verify_error_lines(&error), expected_lines);
        assert_eq!(verify_error_message(&error), expected_message);
        assert_eq!(
            cli_exit_code_number(exit_code_for_error(&error)),
            cli_exit_code_number(expected_code)
        );
    }
}

#[test]
fn success_report_preserves_journaled_durability() {
    let report = verify_success_report(
        &sample_result_with_durability(vec!["profile"], DurabilityMode::Journaled),
        VerifyProfile::Standard,
    );

    assert_eq!(
        report
            .pointer("/durability/profile")
            .and_then(serde_json::Value::as_str),
        Some("journaled")
    );
    assert_eq!(
        report
            .pointer("/durability/journal_written")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
}

#[test]
fn success_report_preserves_strict_durability() {
    let report = verify_success_report(
        &sample_result_with_durability(vec!["profile"], DurabilityMode::Strict),
        VerifyProfile::Standard,
    );

    assert_eq!(
        report
            .pointer("/durability/profile")
            .and_then(serde_json::Value::as_str),
        Some("strict")
    );
    assert_eq!(
        report
            .pointer("/durability/journal_written")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
}

#[test]
fn cmd_verify_with_durability_helper_emits_machine_report() {
    if std::env::var_os(VERIFY_HELPER_ENV).is_none() {
        return;
    }

    let workflow = match std::env::var_os(VERIFY_WORKFLOW_ENV) {
        Some(path) => PathBuf::from(path),
        None => panic!("missing {VERIFY_WORKFLOW_ENV}"),
    };

    let exit_code = cmd_verify_with_durability(
        &workflow,
        VerifyProfile::Standard,
        DurabilityMode::Strict,
        OutputFormat::Text,
        LegacyJsonOutput::Jsonl,
    );
    assert_eq!(exit_code, ExitCode::SUCCESS);
}

#[test]
fn cmd_verify_with_durability_emits_non_default_report_for_real_workflow() {
    let workflow = match fixture_path("tests/fixtures/valid/minimal.yaml") {
        Ok(path) => path,
        Err(error) => panic!("{error}"),
    };
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => panic!("failed to locate current test binary: {error}"),
    };

    let output = match std::process::Command::new(current_exe)
        .arg("--exact")
        .arg(VERIFY_HELPER_TEST)
        .arg("--quiet")
        .arg("--nocapture")
        .env(VERIFY_HELPER_ENV, "1")
        .env(VERIFY_WORKFLOW_ENV, workflow.as_os_str())
        .output()
    {
        Ok(output) => output,
        Err(error) => panic!("failed to spawn durability helper test: {error}"),
    };

    assert!(
        output.status.success(),
        "durability helper failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = match parse_machine_report_line(&stdout) {
        Ok(report) => report,
        Err(error) => panic!("{error}"),
    };

    assert_eq!(
        report
            .pointer("/durability/profile")
            .and_then(serde_json::Value::as_str),
        Some("strict")
    );
    assert_eq!(
        report
            .pointer("/durability/journal_written")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
}

#[test]
fn verification_completion_message_mentions_deferred_gates() {
    let result = sample_result(vec!["profile", "results", "evidence:deferred"]);

    assert_eq!(
        verification_completion_message(&result),
        "Deferred gates remain: evidence. This report does not close all master §63 gates."
    );
}

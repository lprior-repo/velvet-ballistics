//! Workflow verification command and helpers.
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::args::{DurabilityMode, LegacyJsonOutput, OutputFormat, VerifyProfile};
use crate::commands_verify::{VerifyError, VerifyOk, exit_code_for_error, run_verification};
use crate::exit_code::CliExitCode;
use crate::file_io::read_file;
use crate::output::{
    OutputError, json_error, json_out, output_error_exit, write_failure_message,
    write_legacy_json_stderr, write_legacy_json_stdout,
};

/// Default durability profile used by the `verify` command.
///
/// `verify` is a static-analysis pipeline and does not itself write a journal;
/// the durability block in the emitted report describes the *runtime* profile
/// the artifact is intended to run under, not the verify-time profile. The
/// `None` default is the most conservative: "this workflow has not been
/// durably accepted for any specific runtime profile". Callers that want a
/// stricter profile can pass a different [`DurabilityMode`] explicitly.
const VERIFY_DEFAULT_DURABILITY: DurabilityMode = DurabilityMode::None;

/// Run the `verify` command: full static analysis pipeline.
///
/// Returns `ExitCode` based on verification result.
pub(crate) fn cmd_verify(
    workflow: &Path,
    profile: VerifyProfile,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    cmd_verify_with_durability(
        workflow,
        profile,
        VERIFY_DEFAULT_DURABILITY,
        output,
        legacy_json,
    )
}

/// Run the `verify` command with an explicit durability profile.
///
/// Internal entry point that lets callers (notably the explain command, which
/// already knows the durability mode the workflow will run under) propagate
/// the actual runtime durability into the verify report.
pub(crate) fn cmd_verify_with_durability(
    workflow: &Path,
    profile: VerifyProfile,
    durability: DurabilityMode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    let bytes = match read_verify_file(workflow, output, legacy_json) {
        Ok(bytes) => bytes,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            return emit_verify_diagnostic(
                &format!("file is not valid UTF-8: {e}"),
                CliExitCode::ValidationFailed,
                output,
                legacy_json,
            );
        }
    };

    match run_verification(text, &bytes, profile, durability) {
        Ok(result) => {
            let passed_checks = result.passed_gates();
            let deferred_checks = result.deferred_gates();
            if uses_verify_human_text(output, legacy_json) {
                crate::outln!(
                    "verified ({} nodes, profile={})",
                    result.node_count,
                    profile.as_str()
                );
                crate::outln!("gate statuses: {}", result.checks.join(", "));
                crate::outln!("passed gates: {}", passed_checks.join(", "));
                if !deferred_checks.is_empty() {
                    crate::outln!("deferred gates: {}", deferred_checks.join(", "));
                }
                if !result.warnings.is_empty() {
                    crate::outln!("warnings: {}", result.warnings.join(" | "));
                }
                crate::outln!("{}", verification_completion_message(&result));
            } else {
                if let Err(error) = emit_verify_machine_stdout(
                    &verify_success_report(&result, profile),
                    output,
                    legacy_json,
                ) {
                    return output_error_exit(&error);
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = exit_code_for_error(&err);
            if !uses_verify_human_text(output, legacy_json) {
                if let VerifyError::DeferredGates(result) = &err {
                    if let Err(error) = emit_verify_machine_stdout(
                        &verify_deferred_report(result, profile, code),
                        output,
                        legacy_json,
                    ) {
                        return output_error_exit(&error);
                    }
                    return code.into();
                }
                return emit_verify_error(&err, profile, code, output, legacy_json);
            }
            for line in human_verify_error_lines(&err) {
                crate::errln!("{line}");
            }
            code.into()
        }
    }
}

fn uses_verify_human_text(output: OutputFormat, legacy_json: LegacyJsonOutput) -> bool {
    output == OutputFormat::Text && !legacy_json.is_enabled()
}

fn read_verify_file(
    workflow: &Path,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> Result<Vec<u8>, ExitCode> {
    if !legacy_json.is_enabled() {
        return read_file(workflow, output, CliExitCode::ValidationFailed);
    }
    match std::fs::read(workflow) {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(emit_verify_diagnostic(
            &format!("error reading {}: {error}", workflow.display()),
            CliExitCode::ValidationFailed,
            output,
            legacy_json,
        )),
    }
}

fn emit_verify_machine_stdout(
    value: &serde_json::Value,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> Result<(), OutputError> {
    if legacy_json.is_enabled() {
        write_legacy_json_stdout(value, legacy_json)
    } else {
        json_out(value, output)
    }
}

fn emit_verify_machine_stderr(
    value: &serde_json::Value,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    if legacy_json.is_enabled() {
        match write_legacy_json_stderr(value, legacy_json) {
            Ok(()) => code.into(),
            Err(error) => output_error_exit(&error),
        }
    } else {
        json_error(value, code, output);
        code.into()
    }
}

fn emit_verify_diagnostic(
    message: &str,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    if legacy_json.is_enabled() {
        let diagnostic = crate::output_utils::diagnostic_value(message, code);
        match write_legacy_json_stderr(&diagnostic, legacy_json) {
            Ok(()) => code.into(),
            Err(error) => output_error_exit(&error),
        }
    } else {
        write_failure_message(message, output, code);
        code.into()
    }
}

fn emit_verify_error(
    err: &VerifyError,
    profile: VerifyProfile,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    match err {
        VerifyError::YamlParse(msg) => {
            if legacy_json.is_enabled() {
                emit_verify_diagnostic(msg, code, output, legacy_json)
            } else {
                emit_verify_machine_stderr(
                    &serde_json::json!({
                        "success": false,
                        "profile": profile.as_str(),
                        "error": msg
                    }),
                    code,
                    output,
                    legacy_json,
                )
            }
        }
        VerifyError::IrValidation(msg)
        | VerifyError::BudgetPolicy(msg)
        | VerifyError::StorageError(msg)
        | VerifyError::ReplayDivergence(msg) => emit_verify_machine_stderr(
            &serde_json::json!({
                "success": false,
                "profile": profile.as_str(),
                "error": msg
            }),
            code,
            output,
            legacy_json,
        ),
        VerifyError::Compile(errors) => emit_verify_machine_stderr(
            &serde_json::json!({
                "success": false,
                "profile": profile.as_str(),
                "error": "compilation failed",
                "errors": errors
            }),
            code,
            output,
            legacy_json,
        ),
        VerifyError::DeferredGates(result) => emit_verify_machine_stderr(
            &verify_deferred_report(result, profile, code),
            code,
            output,
            legacy_json,
        ),
    }
}

fn human_verify_error_lines(err: &VerifyError) -> Vec<String> {
    match err {
        VerifyError::DeferredGates(result) => {
            let mut lines = vec![
                deferred_gate_message(result),
                format!("gate statuses: {}", result.checks.join(", ")),
                format!("passed gates: {}", result.passed_gates().join(", ")),
            ];
            let deferred_checks = result.deferred_gates();
            if !deferred_checks.is_empty() {
                lines.push(format!("deferred gates: {}", deferred_checks.join(", ")));
            }
            if !result.warnings.is_empty() {
                lines.push(format!("warnings: {}", result.warnings.join(" | ")));
            }
            lines
        }
        VerifyError::Compile(errors) => errors
            .iter()
            .map(|error| format!("compile error: {error}"))
            .collect(),
        VerifyError::YamlParse(message)
        | VerifyError::IrValidation(message)
        | VerifyError::BudgetPolicy(message)
        | VerifyError::StorageError(message)
        | VerifyError::ReplayDivergence(message) => vec![message.clone()],
    }
}

fn deferred_gate_message(result: &VerifyOk) -> String {
    let deferred_checks = result.deferred_gates();
    if deferred_checks.is_empty() {
        "full verification blocked: deferred gates remain".to_string()
    } else {
        format!(
            "full verification blocked: deferred gates remain: {}",
            deferred_checks.join(", ")
        )
    }
}

fn verification_completion_message(result: &VerifyOk) -> String {
    let deferred_checks = result.deferred_gates();
    if deferred_checks.is_empty() {
        "All verification gates closed.".to_string()
    } else {
        format!(
            "Deferred gates remain: {}. This report does not close all master §63 gates.",
            deferred_checks.join(", ")
        )
    }
}

pub(crate) fn verify_success_report(
    result: &VerifyOk,
    profile: VerifyProfile,
) -> serde_json::Value {
    let passed_checks = result.passed_gates();
    let deferred_checks = result.deferred_gates();
    let all_gates_closed = result.all_gates_closed();
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "verify_report",
        "success": true,
        "profile": profile.as_str(),
        "digest": result.digest_hex.as_str(),
        "node_count": result.node_count,
        "checks": &result.checks,
        "passed_checks": &passed_checks,
        "deferred_checks": &deferred_checks,
        "all_gates_closed": all_gates_closed,
        "warnings": &result.warnings,
        "artifact": {
            "source_digest_hex": result.digest_hex.as_str(),
            "ir_digest_hex": result.ir_digest_hex.as_str(),
            "node_count": result.node_count
        },
        "replay": {
            "gates_passed": &passed_checks,
            "gate_sequence": &result.checks,
            "replay_safe": all_gates_closed
        },
        "durability": durability_block(result.durability_mode),
        "repair_hints": [],
        "exit_code": cli_exit_code_number(CliExitCode::Success)
    })
}

fn verify_deferred_report(
    result: &VerifyOk,
    profile: VerifyProfile,
    code: CliExitCode,
) -> serde_json::Value {
    let mut report = verify_success_report(result, profile);
    if let Some(object) = report.as_object_mut() {
        object.insert("success".to_string(), serde_json::Value::Bool(false));
        object.insert(
            "error".to_string(),
            serde_json::Value::String(deferred_gate_message(result)),
        );
        object.insert(
            "repair_hints".to_string(),
            serde_json::json!([
                "Close every deferred master §63 gate before treating --profile full as acceptance evidence"
            ]),
        );
        object.insert(
            "exit_code".to_string(),
            serde_json::json!(cli_exit_code_number(code)),
        );
    }
    report
}

/// Build the `durability` block of the verify report from the durability
/// profile the workflow is intended to run under.
///
/// `journal_written` is `true` only for the `Strict` and `Journaled`
/// profiles (both imply persistence to a journal); for `None` there is no
/// journal, so the block reports `journal_written: false` honestly.
fn durability_block(mode: DurabilityMode) -> serde_json::Value {
    let profile = mode.as_str();
    let journal_written = matches!(mode, DurabilityMode::Strict | DurabilityMode::Journaled);
    serde_json::json!({
        "profile": profile,
        "journal_written": journal_written,
    })
}

pub(crate) fn verify_error_message(err: &VerifyError) -> String {
    match err {
        VerifyError::YamlParse(msg) => format!("YAML parse error: {msg}"),
        VerifyError::Compile(errors) => {
            let mut s = String::from("compilation failed:\n");
            for e in errors {
                s.push_str(&format!("  {e}\n"));
            }
            s
        }
        VerifyError::IrValidation(msg) => format!("IR validation error: {msg}"),
        VerifyError::BudgetPolicy(msg) => format!("budget policy violation: {msg}"),
        VerifyError::StorageError(msg) => format!("storage error: {msg}"),
        VerifyError::ReplayDivergence(msg) => format!("replay divergence: {msg}"),
        VerifyError::DeferredGates(result) => deferred_gate_message(result),
    }
}

pub(crate) fn cli_exit_code_number(code: CliExitCode) -> u8 {
    code.into()
}

#[cfg(test)]
mod tests {
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
    unused_variables,
)]

    use super::*;

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

    fn fixture_path(relative: &str) -> Result<std::path::PathBuf, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
            String::from(
                "compilation failed:\n  first compile failure\n  second compile failure\n"
            )
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
            Some(path) => std::path::PathBuf::from(path),
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
}

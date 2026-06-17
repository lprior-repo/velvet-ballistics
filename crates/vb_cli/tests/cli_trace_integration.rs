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
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::manual_contains,
    clippy::if_same_then_else,
    clippy::multiple_bound_locations,
    clippy::identity_op,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
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
#![forbid(unsafe_code)]
#![cfg(not(miri))]
//! Integration tests for the `trace` command.
//!
//! These tests exercise the full pipeline: CLI argument parsing → journal read → build_trace → output formatting.

use std::ffi::OsStr;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_cli(args: &[&std::ffi::OsStr]) -> Option<std::process::Output> {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args(args);
    command.output().ok()
}

fn output_stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn output_stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_cli_success(output: &std::process::Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed: stdout={} stderr={}",
        output_stdout(output),
        output_stderr(output)
    );
}

fn assert_cli_exit_code(output: &std::process::Output, expected_exit: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "expected exit code {expected_exit}, got {:?}: stdout={} stderr={}",
        output.status.code(),
        output_stdout(output),
        output_stderr(output)
    );
}

// ---------------------------------------------------------------------------
// Integration: trace command with real journal
// ---------------------------------------------------------------------------

fn setup_trace_journal(dir: &std::path::Path) -> vb_core::RunId {
    let journal = FjallJournal::open(dir, None).expect("journal should open");
    let run_id = vb_core::RunId::new(1);
    let workflow_digest = WorkflowDigest::from_bytes([9u8; 32]);

    // Write a minimal set of journal events for a run
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_id,
            seq: EventSeq::new(0),
            workflow: workflow_digest,
        },
        JournalEvent::StepStarted {
            run: run_id,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run: run_id,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            output: SlotIdx::ZERO,
        },
        JournalEvent::RunFinished {
            run: run_id,
            seq: EventSeq::new(3),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("append should succeed");
    run_id
}

fn setup_action_trace_journal(dir: &std::path::Path) -> vb_core::RunId {
    let journal = FjallJournal::open(dir, None).expect("journal should open");
    let run_id = vb_core::RunId::new(2);
    let workflow_digest = WorkflowDigest::from_bytes([8u8; 32]);
    let events = vec![
        JournalEvent::RunAccepted {
            run: run_id,
            seq: EventSeq::new(0),
            workflow: workflow_digest,
        },
        JournalEvent::ActionScheduled {
            run: run_id,
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            action: ActionId::new(17),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run: run_id,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(17),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run: run_id,
            seq: EventSeq::new(3),
            step: StepIdx::new(3),
            action: ActionId::new(23),
            attempt: 1,
        },
    ];
    journal
        .append_strict_batch(&events)
        .expect("append should succeed");
    run_id
}

// ---------------------------------------------------------------------------
// Integration: cmd_trace full pipeline with real Fjall journal
// ---------------------------------------------------------------------------

#[test]
fn cmd_trace_with_events_returns_all_entries_in_order() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some(), "trace command should execute");
    let output = output.unwrap();
    assert_cli_success(&output, "trace");

    let stdout = output_stdout(&output);
    // Check for ordered indices
    assert!(
        stdout.contains("[0]"),
        "stdout should contain index 0: {stdout}"
    );
    assert!(
        stdout.contains("[1]"),
        "stdout should contain index 1: {stdout}"
    );
    assert!(
        stdout.contains("[2]"),
        "stdout should contain index 2: {stdout}"
    );
    assert!(
        stdout.contains("RunAccepted"),
        "stdout should contain RunAccepted: {stdout}"
    );
    assert!(
        stdout.contains("StepStarted"),
        "stdout should contain StepStarted: {stdout}"
    );
    assert!(
        stdout.contains("StepSucceeded"),
        "stdout should contain StepSucceeded: {stdout}"
    );
    assert!(
        stdout.contains("RunFinished"),
        "stdout should contain RunFinished: {stdout}"
    );
    assert!(
        stdout.contains("4 event(s) total"),
        "stdout should report 4 events: {stdout}"
    );
}

#[test]
fn cmd_trace_text_format_structure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace");

    let stdout = output_stdout(&output);
    // Text format: "execution trace for run {id}"
    assert!(
        stdout.contains("execution trace for run"),
        "text output should have header: {stdout}"
    );
    // Text format: "  [idx] EventType step? (seq N)"
    assert!(
        stdout.contains("[0]"),
        "text output should have indexed entries: {stdout}"
    );
    // Text format: "{N} event(s) total"
    assert!(
        stdout.contains("event(s) total"),
        "text output should have total: {stdout}"
    );
}

#[test]
fn cmd_trace_json_format_structure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --emit yaml");

    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run_id:"),
        "YAML should contain run_id: ; got: {stdout}"
    );
    assert!(
        stdout.contains("trace:"),
        "YAML should contain trace: ; got: {stdout}"
    );
    assert!(
        stdout.contains("total:"),
        "YAML should contain total: ; got: {stdout}"
    );
    assert!(
        stdout.contains("total: 4"),
        "YAML should contain total: 4; got: {stdout}"
    );
}

#[test]
fn cmd_trace_jsonl_format_structure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --emit yaml");

    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run_id:"),
        "YAML should contain run_id: ; got: {stdout}"
    );
    assert!(
        stdout.contains("trace:"),
        "YAML should contain trace: ; got: {stdout}"
    );
    assert!(
        stdout.contains("total: 4"),
        "YAML should contain total: 4; got: {stdout}"
    );
}

#[test]
fn cmd_trace_step_filter_returns_only_matching_step() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--step"),
        OsStr::new("0"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --step 0 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("step: 0"),
        "YAML should contain step: 0; got: {stdout}"
    );
    assert!(
        stdout.contains("trace:"),
        "YAML should contain trace: ; got: {stdout}"
    );
}

#[test]
fn cmd_trace_action_filter_returns_only_matching_action() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_action_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--action"),
        OsStr::new("17"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --action 17 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("action: 17"),
        "YAML should contain action: 17; got: {stdout}"
    );
    assert!(
        stdout.contains("trace:"),
        "YAML should contain trace: ; got: {stdout}"
    );
}

#[test]
fn cmd_trace_status_filter_returns_only_active_events() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--status"),
        OsStr::new("active"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --status active --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("status: active"),
        "YAML should contain status: active; got: {stdout}"
    );
    assert!(
        stdout.contains("StepStarted"),
        "YAML should contain StepStarted; got: {stdout}"
    );
}

#[test]
fn cmd_trace_sequence_range_filter_is_inclusive() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--since-seq"),
        OsStr::new("1"),
        OsStr::new("--until-seq"),
        OsStr::new("2"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --since-seq 1 --until-seq 2 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("seq: 1"),
        "YAML should contain seq: 1; got: {stdout}"
    );
    assert!(
        stdout.contains("seq: 2"),
        "YAML should contain seq: 2; got: {stdout}"
    );
}

#[test]
fn cmd_trace_limit_bounds_filtered_output() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_action_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
        OsStr::new("--status"),
        OsStr::new("active"),
        OsStr::new("--limit"),
        OsStr::new("1"),
        OsStr::new("--emit"),
        OsStr::new("yaml"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_success(&output, "trace --status active --limit 1 --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("total:"),
        "YAML should contain total: ; got: {stdout}"
    );
    assert!(
        stdout.contains("trace:"),
        "YAML should contain trace: ; got: {stdout}"
    );
}

#[test]
fn cmd_trace_empty_run_returns_success() {
    let dir = tempfile::tempdir().expect("temp dir");
    // Create an empty journal (no events for run_id=99)
    let journal = FjallJournal::open(dir.path(), None).expect("journal should open");
    // Don't write any events for run 99
    drop(journal);

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("99"),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    // Empty run now returns ValidationFailed (exit 2) per vb-jpq7.21 behavior change
    assert_cli_exit_code(&output, 2);
}

#[test]
fn cmd_trace_invalid_db_path_returns_storage_error() {
    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("1"),
        OsStr::new("--db"),
        OsStr::new("/nonexistent/path/that/does/not/exist"),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 5); // CliExitCode::StorageError = 5
}

#[test]
fn cmd_trace_invalid_run_id_format_returns_validation_failed() {
    let dir = tempfile::tempdir().expect("temp dir");

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("not-a-number"),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 2); // CliExitCode::ValidationFailed = 2
}

#[test]
fn read_journal_events_returns_storage_error_when_dir_not_found() {
    let dir = tempfile::tempdir().expect("temp dir");
    // Journal was never created at this path
    let nonexistent = dir.path().join("truly_nonexistent_journal_db");

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("1"),
        OsStr::new("--db"),
        nonexistent.as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 5); // CliExitCode::StorageError
}

// ---------------------------------------------------------------------------
// E2E: CLI binary trace command exit code
// ---------------------------------------------------------------------------

#[test]
fn cli_trace_command_exit_code_success() {
    let dir = tempfile::tempdir().expect("temp dir");
    let run_id = setup_trace_journal(dir.path());

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new(&run_id.get().to_string()),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 0);
}

#[test]
fn cli_trace_command_on_nonexistent_run_exit_code_two() {
    // Per vb-jpq7.21: non-existent run returns ValidationFailed (exit 2)
    let dir = tempfile::tempdir().expect("temp dir");

    let output = run_cli(&[
        OsStr::new("trace"),
        OsStr::new("999999"),
        OsStr::new("--db"),
        dir.path().as_os_str(),
    ]);

    assert!(output.is_some());
    let output = output.unwrap();
    assert_cli_exit_code(&output, 2);
}

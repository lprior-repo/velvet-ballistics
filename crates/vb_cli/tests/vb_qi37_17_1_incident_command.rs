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
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Integration tests for the `incident` command — vb-qi37.17.1.
//!
//! These tests create a temporary journal, populate it with events, and
//! invoke the velvet-ballistics CLI binary to verify end-to-end behavior.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use vb_core::RunId;
use vb_storage::EventSeq;
use vb_storage::FjallJournal;
use vb_storage::events::JournalEvent;

/// A guard that holds a temp directory and the path to a journal inside it.
struct JournalGuard {
    _temp_dir: tempfile::TempDir,
    db_path: PathBuf,
}

impl JournalGuard {
    fn path(&self) -> &PathBuf {
        &self.db_path
    }
}

/// Helper: run the velvet-ballistics binary with the given arguments.
fn run_cli(args: Vec<OsString>) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_velvet-ballistics");
    let output = Command::new(exe).args(args).output().expect("cli must run");
    output
}

/// Helper to build OsString args from str parts.
fn make_args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|s| OsString::from(s)).collect()
}

/// Create a temporary FjallJournal and append events to it.
fn setup_test_journal(events: &[JournalEvent]) -> JournalGuard {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/incident-command-tmp");
    std::fs::create_dir_all(&root).expect("create temp root");
    let temp_dir = tempfile::Builder::new()
        .prefix("vb-incident-")
        .tempdir_in(root)
        .expect("create temp dir");
    let db_path = temp_dir.path().join("test_db");

    let journal = FjallJournal::open(&db_path, None).expect("open journal");
    journal
        .append_strict_batch(events)
        .expect("append strict batch");

    JournalGuard {
        _temp_dir: temp_dir,
        db_path,
    }
}

/// Build a minimal set of events for a failed run.
fn failed_run_events() -> Vec<JournalEvent> {
    vec![
        JournalEvent::StepStarted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            step: vb_core::ids::StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: RunId::new(42),
            seq: EventSeq::new(1),
            step: vb_core::ids::StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run: RunId::new(42),
            seq: EventSeq::new(2),
            attempt: 1,
        },
    ]
}

/// Build events for a successful run.
fn successful_run_events() -> Vec<JournalEvent> {
    vec![
        JournalEvent::StepStarted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            step: vb_core::ids::StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: RunId::new(42),
            seq: EventSeq::new(1),
            result: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        },
    ]
}

// ---------------------------------------------------------------------------
// T-014: Failed run → JSON output
// ---------------------------------------------------------------------------

#[test]
fn t_014_failed_run_yaml_output() {
    let guard = setup_test_journal(&failed_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--emit",
        "yaml",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "incident should succeed: status={:?} stderr={:?}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    // Verify YAML output contains expected fields
    assert!(
        stdout.contains("run_id"),
        "YAML output should contain run_id field"
    );
    assert!(
        stdout.contains("failure_code"),
        "YAML output should contain failure_code field"
    );
}

// ---------------------------------------------------------------------------
// T-015: Non-existent run → structured error on stderr
// ---------------------------------------------------------------------------

#[test]
fn t_015_nonexistent_run_structured_error() {
    let guard = setup_test_journal(&successful_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "99999",
        "--db",
        db_path.to_str().unwrap(),
        "--emit",
        "yaml",
    ]);
    let output = run_cli(args);

    // Error output is written to stderr as text.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code().map(|c| c != 0).unwrap_or(true),
        "non-existent run should return non-zero exit"
    );
    // Check that stderr contains error indication (YAML or text format).
    // Bead vb-194cz removed the legacy substring matcher that previously
    // re-derived a JSON `code` from the free-form error message; the new
    // typed path emits the caller's explicit `CliExitCode`. For the
    // "no events for run" case that code is `StorageError`.
    assert!(
        stderr.contains("StorageError")
            || stderr.contains("ValidationFailed")
            || stderr.contains("validation")
            || stderr.contains("error"),
        "stderr should contain error indication"
    );
    // POST-003 / INV-002: no stack traces in error output
    assert!(
        !stderr.to_lowercase().contains("backtrace"),
        "error output must not contain stack traces"
    );
    assert!(
        !stderr.contains("at crates/"),
        "error output must not contain source location traces"
    );
}

// ---------------------------------------------------------------------------
// T-016: Successful run → no failure fields populated
// ---------------------------------------------------------------------------

#[test]
fn t_016_successful_run_not_incident() {
    let guard = setup_test_journal(&successful_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--emit",
        "yaml",
    ]);
    let output = run_cli(args);

    // The YAML report should not contain failure indicators for a successful run.
    // POST-004: non-failed run should return StorageError (exit code 5)
    assert_eq!(
        output.status.code(),
        Some(5),
        "non-failed run should return StorageError"
    );
}

// ---------------------------------------------------------------------------
// T-017: Text output format
// ---------------------------------------------------------------------------

#[test]
fn t_017_text_output_format() {
    let guard = setup_test_journal(&failed_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--emit",
        "text",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("incident report for run"));
    assert!(stdout.contains("RunFailed"));
}

// ---------------------------------------------------------------------------
// T-018: YAML output format
// ---------------------------------------------------------------------------

#[test]
fn t_018_yaml_output_format() {
    let guard = setup_test_journal(&failed_run_events());
    let db_path = guard.path();

    let args = make_args(&[
        "incident",
        "42",
        "--db",
        db_path.to_str().unwrap(),
        "--emit",
        "yaml",
    ]);
    let output = run_cli(args);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("failure_code"),
        "YAML output should contain failure_code"
    );
}

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

//! CLI Run and Lifecycle Behavior Tests
//!
//! Tests command parsing, run execution (happy and failure paths), exit code
//! behavior, and input/output contracts for the velvet-ballistics CLI.
//!
//! These are BEHAVIOR tests that verify observable CLI behavior by invoking
//! the binary through process execution and checking stdout/stderr/exit codes.

use std::path::PathBuf;
use std::process::{Command, Output};

/// Helper to get the path to the velvet-ballistics binary.
fn vb_binary() -> PathBuf {
    // Use the cargo test environment or target directory
    std::env::var("CARGO_BIN_EXE_velvet-ballistics")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::fs::canonicalize(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
                .ok()
                .map(|p| p.join("../../target/debug/velvet-ballistics"))
        })
        .unwrap_or_else(|| PathBuf::from("velvet-ballistics"))
}

/// Run the velvet-ballistics binary with the given args and return the output.
fn run_vb(args: &[&str]) -> Output {
    let binary = vb_binary();
    let mut cmd = Command::new(&binary);
    cmd.args(args);
    cmd.output().expect("failed to execute velvet-ballistics")
}

/// Check that velvet-ballistics prints help and exits successfully.
#[test]
fn cli_help_command_succeeds() {
    let output = run_vb(&["help"]);
    assert!(
        output.status.success(),
        "help command should succeed, but got exit code {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("velvet-ballistics") && stdout.contains("commands:"),
        "help output should contain command list: {}",
        stdout
    );
}

/// Check that velvet-ballistics prints version and exits successfully.
#[test]
fn cli_version_command_succeeds() {
    let output = run_vb(&["version"]);
    assert!(output.status.success(), "version command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("velvet-ballistics"),
        "version output should contain package name: {}",
        stdout
    );
}

/// Check that running with no command shows help.
#[test]
fn cli_no_command_shows_help() {
    let output = run_vb(&[]);
    // No command should show help or error gracefully
    let has_output = !output.stdout.is_empty() || !output.stderr.is_empty();
    assert!(has_output, "should produce some output");
}

/// Check that unknown command produces an error.
#[test]
fn cli_unknown_command_returns_nonzero() {
    let output = run_vb(&["foobar"]);
    assert!(!output.status.success(), "unknown command should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown command") || stderr.contains("help"),
        "error output should mention unknown command or help: {}",
        stderr
    );
}

/// Check that --version short flag works.
#[test]
fn cli_version_short_flag_succeeds() {
    let output = run_vb(&["--version"]);
    assert!(output.status.success(), "--version flag should succeed");
}

/// Check that -V short flag works.
#[test]
fn cli_version_v_flag_succeeds() {
    let output = run_vb(&["-V"]);
    assert!(output.status.success(), "-V flag should succeed");
}

/// Check that --help short flag works.
#[test]
fn cli_help_short_flag_succeeds() {
    let output = run_vb(&["--help"]);
    assert!(output.status.success(), "--help flag should succeed");
}

/// Check that -h short flag works.
#[test]
fn cli_help_h_flag_succeeds() {
    let output = run_vb(&["-h"]);
    assert!(output.status.success(), "-h flag should succeed");
}

// ============================================================================
// Run Command Behavior Tests
// ============================================================================

mod run_command {
    use super::*;

    /// run without any arguments should fail with usage info.
    #[test]
    fn cli_run_without_args_fails() {
        let output = run_vb(&["run"]);
        assert!(!output.status.success(), "run without args should fail");
    }

    /// run with workflow but missing --input-bin should fail.
    #[test]
    fn cli_run_missing_input_bin_fails() {
        let output = run_vb(&["run", "workflow.yaml", "--durability", "none"]);
        assert!(
            !output.status.success(),
            "run missing --input-bin should fail"
        );
    }

    /// run with workflow and --input-bin but missing --durability should fail.
    #[test]
    fn cli_run_missing_durability_fails() {
        let output = run_vb(&["run", "workflow.yaml", "--input-bin", "input.bin"]);
        assert!(
            !output.status.success(),
            "run missing --durability should fail"
        );
    }

    /// run with --durability none but missing --db should succeed (none mode doesn't need db).
    #[test]
    fn cli_run_durability_none_without_db_succeeds() {
        // This will fail at workflow validation, but not at arg parsing
        // We just verify the arg parsing succeeds
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
        ]);
        // Should fail on file read/validation, not on missing --db
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument: --db"),
            "durability=none should not require --db: {}",
            stderr
        );
    }

    /// run with --durability journaled without --db should fail.
    #[test]
    fn cli_run_durability_journaled_requires_db() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "journaled",
        ]);
        assert!(
            !output.status.success(),
            "run with journaled durability without --db should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument: --db") || stderr.contains("--db"),
            "should report missing --db: {}",
            stderr
        );
    }

    /// run with --durability strict without --db should fail.
    #[test]
    fn cli_run_durability_strict_requires_db() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "strict",
        ]);
        assert!(
            !output.status.success(),
            "run with strict durability without --db should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument: --db") || stderr.contains("--db"),
            "should report missing --db: {}",
            stderr
        );
    }

    /// run with journaled durability and --db should work (up to file validation).
    #[test]
    fn cli_run_durability_journaled_with_db_passes_arg_parsing() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "journaled",
            "--db",
            "journal-db",
        ]);
        // Should fail on file not found, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument"),
            "should not fail on missing args: {}",
            stderr
        );
    }

    /// run with invalid durability mode should fail with appropriate error.
    #[test]
    fn cli_run_invalid_durability_mode_fails() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "invalid-mode",
        ]);
        assert!(
            !output.status.success(),
            "run with invalid durability should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown durability mode") || stderr.contains("invalid-mode"),
            "should report invalid durability mode: {}",
            stderr
        );
    }

    /// run with --step but missing --step-input should fail.
    #[test]
    fn cli_run_step_requires_step_input() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
            "--step",
            "5",
        ]);
        assert!(
            !output.status.success(),
            "run with --step but no --step-input should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument") || stderr.contains("--step-input"),
            "should report missing --step-input: {}",
            stderr
        );
    }

    /// run with both --step and --step-input should pass arg parsing.
    #[test]
    fn cli_run_with_step_flags_passes_arg_parsing() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
            "--step",
            "3",
            "--step-input",
            "step-data.bin",
        ]);
        // Should fail on file validation, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument: --step"),
            "should not fail on step args: {}",
            stderr
        );
    }
}

// ============================================================================
// Validate Command Behavior Tests
// ============================================================================

mod validate_command {
    use super::*;

    /// validate without workflow should fail.
    #[test]
    fn cli_validate_without_workflow_fails() {
        let output = run_vb(&["validate"]);
        assert!(
            !output.status.success(),
            "validate without workflow should fail"
        );
    }

    /// validate with nonexistent file should fail with file error.
    #[test]
    fn cli_validate_nonexistent_file_fails() {
        let output = run_vb(&["validate", "nonexistent-file.yaml"]);
        assert!(
            !output.status.success(),
            "validate nonexistent file should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("error reading") || stderr.contains("nonexistent"),
            "should report file error: {}",
            stderr
        );
    }
}

// ============================================================================
// Compile Command Behavior Tests
// ============================================================================

mod compile_command {
    use super::*;

    /// compile without args should fail.
    #[test]
    fn cli_compile_without_args_fails() {
        let output = run_vb(&["compile"]);
        assert!(!output.status.success(), "compile without args should fail");
    }

    /// compile missing --emit should fail.
    #[test]
    fn cli_compile_missing_emit_fails() {
        let output = run_vb(&["compile", "workflow.yaml", "--out", "output.vbir"]);
        assert!(
            !output.status.success(),
            "compile missing --emit should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument") || stderr.contains("--emit"),
            "should report missing --emit: {}",
            stderr
        );
    }

    /// compile missing --out should fail.
    #[test]
    fn cli_compile_missing_out_fails() {
        let output = run_vb(&["compile", "workflow.yaml", "--emit", "ir"]);
        assert!(
            !output.status.success(),
            "compile missing --out should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument") || stderr.contains("--out"),
            "should report missing --out: {}",
            stderr
        );
    }

    /// compile with unknown emit target should fail.
    #[test]
    fn cli_compile_unknown_emit_fails() {
        let output = run_vb(&[
            "compile",
            "workflow.yaml",
            "--emit",
            "wasm",
            "--out",
            "output.vbir",
        ]);
        assert!(
            !output.status.success(),
            "compile with unknown emit target should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown emit target") || stderr.contains("wasm"),
            "should report unknown emit target: {}",
            stderr
        );
    }
}

// ============================================================================
// Verify Command Behavior Tests
// ============================================================================

mod verify_command {
    use super::*;

    /// verify without workflow should fail.
    #[test]
    fn cli_verify_without_workflow_fails() {
        let output = run_vb(&["verify"]);
        assert!(
            !output.status.success(),
            "verify without workflow should fail"
        );
    }

    /// verify with unknown profile should fail.
    #[test]
    fn cli_verify_unknown_profile_fails() {
        let output = run_vb(&["verify", "workflow.yaml", "--profile", "thorough"]);
        assert!(
            !output.status.success(),
            "verify with unknown profile should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown verify profile") || stderr.contains("thorough"),
            "should report unknown profile: {}",
            stderr
        );
    }
}

// ============================================================================
// Cancel/Retry/Resume Command Behavior Tests
// ============================================================================

mod lifecycle_commands {
    use super::*;

    /// cancel without run_id should fail.
    #[test]
    fn cli_cancel_without_run_id_fails() {
        let output = run_vb(&["cancel"]);
        assert!(
            !output.status.success(),
            "cancel without run_id should fail"
        );
    }

    /// cancel without --db should fail.
    #[test]
    fn cli_cancel_without_db_fails() {
        let output = run_vb(&["cancel", "42"]);
        assert!(!output.status.success(), "cancel without --db should fail");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument") || stderr.contains("--db"),
            "should report missing --db: {}",
            stderr
        );
    }

    /// cancel with reason longer than 256 chars should fail.
    #[test]
    fn cli_cancel_reason_too_long_fails() {
        let long_reason = "a".repeat(257);
        let output = run_vb(&[
            "cancel",
            "42",
            "--db",
            "journal-db",
            "--reason",
            &long_reason,
        ]);
        assert!(
            !output.status.success(),
            "cancel with reason > 256 chars should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("256") || stderr.contains("reason"),
            "should report reason length error: {}",
            stderr
        );
    }

    /// retry without run_id should fail.
    #[test]
    fn cli_retry_without_run_id_fails() {
        let output = run_vb(&["retry"]);
        assert!(!output.status.success(), "retry without run_id should fail");
    }

    /// retry without --db should fail.
    #[test]
    fn cli_retry_without_db_fails() {
        let output = run_vb(&["retry", "42"]);
        assert!(!output.status.success(), "retry without --db should fail");
    }

    /// resume without run_id should fail.
    #[test]
    fn cli_resume_without_run_id_fails() {
        let output = run_vb(&["resume"]);
        assert!(
            !output.status.success(),
            "resume without run_id should fail"
        );
    }

    /// resume without --db should fail.
    #[test]
    fn cli_resume_without_db_fails() {
        let output = run_vb(&["resume", "42"]);
        assert!(!output.status.success(), "resume without --db should fail");
    }
}

// ============================================================================
// Answer Command Behavior Tests
// ============================================================================

mod answer_command {
    use super::*;

    /// answer without args should fail.
    #[test]
    fn cli_answer_without_args_fails() {
        let output = run_vb(&["answer"]);
        assert!(!output.status.success(), "answer without args should fail");
    }

    /// answer with invalid slot should fail.
    #[test]
    fn cli_answer_invalid_slot_fails() {
        let output = run_vb(&[
            "answer",
            "42",
            "--slot",
            "not-a-number",
            "--value",
            "value.bin",
            "--db",
            "journal-db",
        ]);
        assert!(
            !output.status.success(),
            "answer with invalid slot should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid") || stderr.contains("not-a-number"),
            "should report invalid slot: {}",
            stderr
        );
    }
}

// ============================================================================
// Trace Command Behavior Tests
// ============================================================================

mod trace_command {
    use super::*;

    /// trace without run_id should fail.
    #[test]
    fn cli_trace_without_run_id_fails() {
        let output = run_vb(&["trace"]);
        assert!(!output.status.success(), "trace without run_id should fail");
    }

    /// trace without --db should fail.
    #[test]
    fn cli_trace_without_db_fails() {
        let output = run_vb(&["trace", "7"]);
        assert!(!output.status.success(), "trace without --db should fail");
    }

    /// trace with unknown filter flag should fail.
    #[test]
    fn cli_trace_unknown_filter_fails() {
        let output = run_vb(&[
            "trace",
            "7",
            "--db",
            "journal-db",
            "--unknown-flag",
            "value",
        ]);
        assert!(
            !output.status.success(),
            "trace with unknown filter should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown") || stderr.contains("trace"),
            "should report unknown flag: {}",
            stderr
        );
    }
}

// ============================================================================
// Diff Command Behavior Tests
// ============================================================================

mod diff_command {
    use super::*;

    /// diff without both run_ids should fail.
    #[test]
    fn cli_diff_without_both_run_ids_fails() {
        let output = run_vb(&["diff", "1"]);
        assert!(
            !output.status.success(),
            "diff with only one run_id should fail"
        );
    }

    /// diff without --db should fail.
    #[test]
    fn cli_diff_without_db_fails() {
        let output = run_vb(&["diff", "1", "2"]);
        assert!(!output.status.success(), "diff without --db should fail");
    }
}

// ============================================================================
// Simulate/Graph/Explain/Bench-run Command Behavior Tests
// ============================================================================

mod workflow_commands {
    use super::*;

    /// simulate without workflow should fail.
    #[test]
    fn cli_simulate_without_workflow_fails() {
        let output = run_vb(&["simulate"]);
        assert!(
            !output.status.success(),
            "simulate without workflow should fail"
        );
    }

    /// graph without workflow should fail.
    #[test]
    fn cli_graph_without_workflow_fails() {
        let output = run_vb(&["graph"]);
        assert!(
            !output.status.success(),
            "graph without workflow should fail"
        );
    }

    /// explain without workflow should fail.
    #[test]
    fn cli_explain_without_workflow_fails() {
        let output = run_vb(&["explain"]);
        assert!(
            !output.status.success(),
            "explain without workflow should fail"
        );
    }

    /// bench-run without workflow should fail.
    #[test]
    fn cli_bench_run_without_workflow_fails() {
        let output = run_vb(&["bench-run"]);
        assert!(
            !output.status.success(),
            "bench-run without workflow should fail"
        );
    }
}

// ============================================================================
// Doctor Command Behavior Tests
// ============================================================================

mod doctor_command {
    use super::*;

    /// doctor without --db should succeed (stateless mode).
    #[test]
    fn cli_doctor_without_db_runs() {
        let output = run_vb(&["doctor"]);
        // Doctor can run without a db in stateless mode
        let has_output = !output.stdout.is_empty() || !output.stderr.is_empty();
        assert!(has_output, "doctor should produce output");
        // Should not fail on missing --db
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument: --db"),
            "doctor should not require --db: {}",
            stderr
        );
    }

    /// doctor with --db should also work.
    #[test]
    fn cli_doctor_with_db_runs() {
        let output = run_vb(&["doctor", "--db", "nonexistent-db"]);
        // Should run (may fail on actual db operations, but not arg parsing)
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument"),
            "doctor with --db should not fail on arg parsing: {}",
            stderr
        );
    }
}

// ============================================================================
// IPC Serve Command Behavior Tests
// ============================================================================

mod ipc_serve_command {
    use super::*;

    /// ipc-serve without --socket should fail.
    #[test]
    fn cli_ipc_serve_without_socket_fails() {
        let output = run_vb(&["ipc-serve", "--db", "journal-db"]);
        assert!(
            !output.status.success(),
            "ipc-serve without --socket should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument") || stderr.contains("--socket"),
            "should report missing --socket: {}",
            stderr
        );
    }

    /// ipc-serve without --db should fail.
    #[test]
    fn cli_ipc_serve_without_db_fails() {
        let output = run_vb(&["ipc-serve", "--socket", "/tmp/socket"]);
        assert!(
            !output.status.success(),
            "ipc-serve without --db should fail"
        );
    }
}

// ============================================================================
// Inspect/Events/Replay Command Behavior Tests
// ============================================================================

mod inspect_events_replay {
    use super::*;

    /// inspect without run_id should fail.
    #[test]
    fn cli_inspect_without_run_id_fails() {
        let output = run_vb(&["inspect"]);
        assert!(
            !output.status.success(),
            "inspect without run_id should fail"
        );
    }

    /// inspect without --db should fail.
    #[test]
    fn cli_inspect_without_db_fails() {
        let output = run_vb(&["inspect", "42"]);
        assert!(!output.status.success(), "inspect without --db should fail");
    }

    /// events without run_id should fail.
    #[test]
    fn cli_events_without_run_id_fails() {
        let output = run_vb(&["events"]);
        assert!(
            !output.status.success(),
            "events without run_id should fail"
        );
    }

    /// events without --db should fail.
    #[test]
    fn cli_events_without_db_fails() {
        let output = run_vb(&["events", "42"]);
        assert!(!output.status.success(), "events without --db should fail");
    }

    /// replay without run_id should fail.
    #[test]
    fn cli_replay_without_run_id_fails() {
        let output = run_vb(&["replay"]);
        assert!(
            !output.status.success(),
            "replay without run_id should fail"
        );
    }

    /// replay without --db should fail.
    #[test]
    fn cli_replay_without_db_fails() {
        let output = run_vb(&["replay", "42"]);
        assert!(!output.status.success(), "replay without --db should fail");
    }
}

// ============================================================================
// Submit Command Behavior Tests
// ============================================================================

mod submit_command {
    use super::*;

    /// submit without all required args should fail.
    #[test]
    fn cli_submit_without_db_fails() {
        let output = run_vb(&[
            "submit",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "journaled",
        ]);
        assert!(!output.status.success(), "submit without --db should fail");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("missing argument") || stderr.contains("--db"),
            "should report missing --db: {}",
            stderr
        );
    }

    /// submit with all required args should pass arg parsing.
    #[test]
    fn cli_submit_with_all_args_passes_parsing() {
        let output = run_vb(&[
            "submit",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "journaled",
            "--db",
            "journal-db",
        ]);
        // Should fail on file validation, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument"),
            "should not fail on missing args: {}",
            stderr
        );
    }
}

// ============================================================================
// Incident Command Behavior Tests
// ============================================================================

mod incident_command {
    use super::*;

    /// incident without run_id should fail.
    #[test]
    fn cli_incident_without_run_id_fails() {
        let output = run_vb(&["incident"]);
        assert!(
            !output.status.success(),
            "incident without run_id should fail"
        );
    }

    /// incident without --db should fail.
    #[test]
    fn cli_incident_without_db_fails() {
        let output = run_vb(&["incident", "42"]);
        assert!(
            !output.status.success(),
            "incident without --db should fail"
        );
    }
}

// ============================================================================
// Agent Context Command Behavior Tests
// ============================================================================

mod agent_context_command {
    use super::*;

    /// agent-context without args should succeed.
    #[test]
    fn cli_agent_context_without_args_succeeds() {
        let output = run_vb(&["agent-context"]);
        // agent-context can run without additional args
        let has_output = !output.stdout.is_empty() || !output.stderr.is_empty();
        assert!(has_output, "agent-context should produce output");
    }

    /// agent-context with --deliver flag and target should work.
    #[test]
    fn cli_agent_context_with_deliver_succeeds() {
        let output = run_vb(&["agent-context", "--deliver", "file:/tmp/test.jsonl"]);
        // Should either succeed or fail gracefully on file write, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unknown flag"),
            "should not fail on --deliver flag: {}",
            stderr
        );
    }
}

// ============================================================================
// Action Command Behavior Tests
// ============================================================================

mod action_command {
    use super::*;

    /// action list without args should work.
    #[test]
    fn cli_action_list_succeeds() {
        let output = run_vb(&["action", "list"]);
        // action list should work and produce output
        let has_output = !output.stdout.is_empty() || !output.stderr.is_empty();
        assert!(has_output, "action list should produce output");
    }

    /// action inspect without action_id should fail.
    #[test]
    fn cli_action_inspect_without_id_fails() {
        let output = run_vb(&["action", "inspect"]);
        assert!(
            !output.status.success(),
            "action inspect without id should fail"
        );
    }

    /// action inspect with invalid id should fail.
    #[test]
    fn cli_action_inspect_invalid_id_fails() {
        let output = run_vb(&["action", "inspect", "not-a-number"]);
        assert!(
            !output.status.success(),
            "action inspect with invalid id should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid action id") || stderr.contains("not-a-number"),
            "should report invalid action id: {}",
            stderr
        );
    }

    /// action with unknown subcommand should fail.
    #[test]
    fn cli_action_unknown_subcommand_fails() {
        let output = run_vb(&["action", "unknown"]);
        assert!(
            !output.status.success(),
            "action with unknown subcommand should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown action command") || stderr.contains("unknown"),
            "should report unknown action command: {}",
            stderr
        );
    }
}

// ============================================================================
// System Status Command Behavior Tests
// ============================================================================

mod system_status_command {
    use super::*;

    /// system without subcommand should fail.
    #[test]
    fn cli_system_without_subcommand_fails() {
        let output = run_vb(&["system"]);
        assert!(
            !output.status.success(),
            "system without subcommand should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("system subcommand") || stderr.contains("missing"),
            "should report missing system subcommand: {}",
            stderr
        );
    }

    /// system status should work.
    #[test]
    fn cli_system_status_succeeds() {
        let output = run_vb(&["system", "status"]);
        // system status should work
        let has_output = !output.stdout.is_empty() || !output.stderr.is_empty();
        assert!(has_output, "system status should produce output");
    }

    /// system status with unknown profile should fail.
    #[test]
    fn cli_system_status_unknown_profile_fails() {
        let output = run_vb(&["system", "status", "--profile", "deep"]);
        assert!(
            !output.status.success(),
            "system status with unknown profile should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown profile") || stderr.contains("deep"),
            "should report unknown profile: {}",
            stderr
        );
    }

    /// system status with unprobed server mode should fail.
    #[test]
    fn cli_system_status_unprobed_server_mode_fails() {
        let output = run_vb(&["system", "status", "--server", "strict"]);
        assert!(
            !output.status.success(),
            "system status with strict server mode should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("server mode") || stderr.contains("strict"),
            "should report unprobed server mode: {}",
            stderr
        );
    }
}

// ============================================================================
// Status Command Behavior Tests
// ============================================================================

mod status_command {
    use super::*;

    /// status without args should work.
    #[test]
    fn cli_status_without_args_succeeds() {
        let output = run_vb(&["status"]);
        // status should work and produce output
        let has_output = !output.stdout.is_empty() || !output.stderr.is_empty();
        assert!(has_output, "status should produce output");
    }

    /// status with --queue-depth 1025 should fail (exceeds max of 1024).
    #[test]
    fn cli_status_queue_depth_out_of_range_fails() {
        let output = run_vb(&["status", "--queue-depth", "1025"]);
        assert!(
            !output.status.success(),
            "status with queue-depth > 1024 should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("1024") || stderr.contains("queue-depth"),
            "should report out of range: {}",
            stderr
        );
    }

    /// status with --active-runs 1025 should fail.
    #[test]
    fn cli_status_active_runs_out_of_range_fails() {
        let output = run_vb(&["status", "--active-runs", "1025"]);
        assert!(
            !output.status.success(),
            "status with active-runs > 1024 should fail"
        );
    }

    /// status with non-numeric value should fail.
    #[test]
    fn cli_status_non_numeric_arg_fails() {
        let output = run_vb(&["status", "--queue-depth", "many"]);
        assert!(
            !output.status.success(),
            "status with non-numeric value should fail"
        );
    }
}

// ============================================================================
// AI Context Command Behavior Tests
// ============================================================================

mod ai_context_command {
    use super::*;

    /// ai-context without run_id should fail.
    #[test]
    fn cli_ai_context_without_run_id_fails() {
        let output = run_vb(&["ai-context"]);
        assert!(
            !output.status.success(),
            "ai-context without run_id should fail"
        );
    }

    /// ai-context without --db should fail.
    #[test]
    fn cli_ai_context_without_db_fails() {
        let output = run_vb(&["ai-context", "42"]);
        assert!(
            !output.status.success(),
            "ai-context without --db should fail"
        );
    }

    /// ai-context with all required args should pass arg parsing.
    #[test]
    fn cli_ai_context_with_all_args_passes_parsing() {
        let output = run_vb(&["ai-context", "42", "--db", "journal-db"]);
        // Should fail on db operations, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("missing argument"),
            "should not fail on missing args: {}",
            stderr
        );
    }
}

// ============================================================================
// Output Format Behavior Tests
// ============================================================================

mod output_format {
    use super::*;

    /// Commands should accept --emit yaml flag.
    #[test]
    fn cli_validate_accepts_emit_yaml() {
        let output = run_vb(&["validate", "--emit", "yaml", "workflow.yaml"]);
        // Should fail on file, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unknown emit mode"),
            "should accept --emit yaml: {}",
            stderr
        );
    }

    /// Commands should accept --json flag.
    #[test]
    fn cli_run_accepts_json_flag() {
        let output = run_vb(&[
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
            "--json",
        ]);
        // Should fail on file, not arg parsing
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unknown flag"),
            "should accept --json: {}",
            stderr
        );
    }
}

// ============================================================================
// Subcommand Help Flag Behavior Tests
// ============================================================================

mod subcommand_help {
    use super::*;

    /// Commands should return Help when given --help.
    #[test]
    fn cli_run_with_help_flag_shows_help() {
        let output = run_vb(&["run", "--help"]);
        assert!(output.status.success(), "run --help should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("run") || stdout.contains("workflow"),
            "help output should mention run: {}",
            stdout
        );
    }

    /// Commands should return Help when given -h.
    #[test]
    fn cli_validate_with_h_flag_shows_help() {
        let output = run_vb(&["validate", "-h"]);
        assert!(output.status.success(), "validate -h should succeed");
    }
}

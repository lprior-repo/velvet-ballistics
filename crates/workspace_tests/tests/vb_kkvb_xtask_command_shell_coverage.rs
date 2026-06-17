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

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const WORKSPACE_MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
const TOKENS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../design/tokens/velvet_ui_tokens.toml"
);

#[test]
fn xtask_help_lists_required_and_legacy_commands_when_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["--help"])?;

    // Then
    let stdout = stdout_text(&output)?;
    let stderr = stderr_text(&output)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "xtask help failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_eq!(stdout.contains("Required command families:"), true);
    assert_eq!(stdout.contains("  ai-context"), true);
    assert_eq!(stdout.contains("Legacy commands:"), true);
    assert_eq!(stdout.contains("  ui-snapshot"), true);
    Ok(())
}

#[test]
fn xtask_version_prints_package_version_when_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["--version"])?;

    // Then
    let stdout = stdout_text(&output)?;
    let stderr = stderr_text(&output)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "xtask version failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_eq!(stdout, "xtask 0.1.0\n");
    Ok(())
}

#[test]
#[ignore] // UI commands moved to velvet-optional repo
fn xtask_legacy_separator_routes_ui_overlap_check_and_reports_missing_screen()
-> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(
        workspace.path(),
        &[
            "--",
            "ui-overlap-check",
            "--screen",
            "missing_screen",
            "--input-dir",
            "missing_snapshots",
        ],
    )?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout_text(&output)?.contains("FAIL: missing_snapshots/missing_screen.png does not exist"),
        true
    );
    assert_eq!(
        stderr_text(&output)?.contains("UI overlap check failed"),
        true
    );
    Ok(())
}

#[test]
#[ignore] // UI commands moved to velvet-optional repo
fn xtask_ui_tokens_writes_rust_constants_when_tokens_are_valid() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("generated").join("tokens.rs");
    let output_arg = output_path.to_string_lossy().to_string();

    // When
    let output = run_xtask(
        workspace.path(),
        &[
            "ui-tokens",
            "--input",
            TOKENS_FILE,
            "--output",
            &output_arg,
            "--emit",
            "json",
        ],
    )?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output_path.exists(), true);
    assert_eq!(stdout_text(&output)?.contains("background_board"), true);
    assert_eq!(
        std::fs::read_to_string(output_path)?.contains("pub const TOKENS"),
        true
    );
    Ok(())
}

#[test]
#[ignore] // UI commands moved to velvet-optional repo
fn xtask_ui_tokens_check_confirms_generated_tokens_when_file_matches() -> Result<(), Box<dyn Error>>
{
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("generated_tokens.rs");
    let output_arg = output_path.to_string_lossy().to_string();
    let write_output = run_xtask(
        workspace.path(),
        &["ui-tokens", "--input", TOKENS_FILE, "--output", &output_arg],
    )?;
    assert_eq!(write_output.status.code(), Some(0));

    // When
    let check_output = run_xtask(
        workspace.path(),
        &[
            "ui-tokens",
            "--input",
            TOKENS_FILE,
            "--output",
            &output_arg,
            "--check",
        ],
    )?;

    // Then
    assert_eq!(check_output.status.code(), Some(0));
    assert_eq!(
        stdout_text(&check_output)?.contains("Generated UI tokens are current"),
        true
    );
    Ok(())
}

#[test]
#[ignore] // UI commands moved to velvet-optional repo
fn xtask_ui_tokens_check_rejects_stale_generated_tokens() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("stale_tokens.rs");
    std::fs::write(&output_path, "stale")?;
    let output_arg = output_path.to_string_lossy().to_string();

    // When
    let output = run_xtask(
        workspace.path(),
        &[
            "ui-tokens",
            "--input",
            TOKENS_FILE,
            "--output",
            &output_arg,
            "--check",
        ],
    )?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_text(&output)?.contains("Generated UI tokens are stale"),
        true
    );
    Ok(())
}

#[test]
#[ignore] // UI commands moved to velvet-optional repo
fn xtask_ui_snapshot_captures_named_fixture_and_writes_report() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_dir = workspace.path().join("snapshots");
    let output_arg = output_dir.to_string_lossy().to_string();

    // When
    let output = run_xtask(
        Path::new(WORKSPACE_ROOT),
        &[
            "ui-snapshot",
            "--fixture",
            "execution_overview",
            "--output-dir",
            &output_arg,
            "--emit",
            "yaml",
        ],
    )?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output_dir.join("execution_overview.png").exists(), true);
    assert_eq!(output_dir.join("ui_snapshot_report.yaml").exists(), true);
    assert_eq!(
        stdout_text(&output)?.contains("Snapshot report written to:"),
        true
    );
    Ok(())
}

#[test]
#[ignore] // UI commands moved to velvet-optional repo
fn xtask_ui_snapshot_rejects_invocation_without_all_or_fixture() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["ui-snapshot"])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_text(&output)?.contains("Must specify --all or --fixture <name>"),
        true
    );
    Ok(())
}

fn run_xtask(current_dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let cargo_target_dir = xtask_target_dir();
    Command::new("cargo")
        .current_dir(current_dir)
        .args([
            "run",
            "--manifest-path",
            WORKSPACE_MANIFEST,
            "-p",
            "xtask",
            "--",
        ])
        .args(args)
        .env("CARGO_TARGET_DIR", cargo_target_dir)
        .env("TMPDIR", current_dir)
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTDOCFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .output()
        .map_err(Into::into)
}

fn xtask_target_dir() -> PathBuf {
    Path::new(WORKSPACE_ROOT).join("target/xtask-command-shell-coverage")
}

fn stdout_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stdout.clone()).map_err(Into::into)
}

fn stderr_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stderr.clone()).map_err(Into::into)
}

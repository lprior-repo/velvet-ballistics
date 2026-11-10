use std::error::Error;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const TOKENS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../design/tokens/velvet_ui_tokens.toml"
);

#[test]
fn xtask_help_lists_required_and_legacy_commands_when_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = Path::new(WORKSPACE_ROOT);

    // When
    let output = run_xtask(workspace, &["--help"])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout_text(&output)?;
    assert_eq!(stdout.contains("Required command families:"), true);
    assert_eq!(stdout.contains("  ai-context"), true);
    assert_eq!(stdout.contains("Legacy commands:"), true);
    assert_eq!(stdout.contains("  ui-snapshot"), true);
    Ok(())
}

#[test]
fn xtask_version_prints_package_version_when_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = Path::new(WORKSPACE_ROOT);

    // When
    let output = run_xtask(workspace, &["--version"])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_text(&output)?, "xtask 0.1.0\n");
    Ok(())
}

#[test]
fn xtask_legacy_separator_routes_ui_overlap_check_and_reports_moved() -> Result<(), Box<dyn Error>>
{
    let workspace = TempDir::new()?;
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
    assert_deferred_ui_command(&output)
}

#[test]
fn xtask_ui_tokens_reports_moved_when_tokens_are_valid() -> Result<(), Box<dyn Error>> {
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("generated").join("tokens.rs");
    let output_arg = output_path.to_string_lossy().to_string();
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
    assert_deferred_ui_command(&output)?;
    assert_eq!(output_path.exists(), false);
    Ok(())
}

#[test]
fn xtask_ui_tokens_check_reports_moved_when_file_matches() -> Result<(), Box<dyn Error>> {
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("generated_tokens.rs");
    let output_arg = output_path.to_string_lossy().to_string();
    let write_output = run_xtask(
        workspace.path(),
        &["ui-tokens", "--input", TOKENS_FILE, "--output", &output_arg],
    )?;
    assert_deferred_ui_command(&write_output)?;
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
    assert_deferred_ui_command(&check_output)
}

#[test]
fn xtask_ui_tokens_check_reports_moved_when_tokens_are_stale() -> Result<(), Box<dyn Error>> {
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("stale_tokens.rs");
    std::fs::write(&output_path, "stale")?;
    let output_arg = output_path.to_string_lossy().to_string();
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
    assert_deferred_ui_command(&output)?;
    assert_eq!(std::fs::read_to_string(&output_path)?, "stale");
    Ok(())
}

#[test]
fn xtask_ui_snapshot_reports_moved_when_fixture_requested() -> Result<(), Box<dyn Error>> {
    let workspace = TempDir::new()?;
    let output_dir = workspace.path().join("snapshots");
    let output_arg = output_dir.to_string_lossy().to_string();
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
    assert_deferred_ui_command(&output)?;
    assert_eq!(output_dir.exists(), false);
    Ok(())
}

#[test]
fn xtask_ui_snapshot_reports_moved_without_all_or_fixture() -> Result<(), Box<dyn Error>> {
    let workspace = TempDir::new()?;
    let output = run_xtask(workspace.path(), &["ui-snapshot"])?;
    assert_deferred_ui_command(&output)
}

fn assert_deferred_ui_command(output: &Output) -> Result<(), Box<dyn Error>> {
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(output)?, "");
    assert_eq!(
        stderr_text(output)?
            .contains("UI commands moved to velvet-optional repo (deferred milestone)"),
        true
    );
    Ok(())
}

fn run_xtask(current_dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Command::new("cargo")
        .current_dir(current_dir)
        .args([
            "run",
            "--manifest-path",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../xtask/Cargo.toml"),
            "--",
        ])
        .args(args)
        .output()
        .map_err(Into::into)
}

fn stdout_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stdout.clone()).map_err(Into::into)
}

fn stderr_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stderr.clone()).map_err(Into::into)
}

use std::error::Error;
use std::path::Path;
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
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["--version"])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_text(&output)?, "xtask 0.1.0\n");
    Ok(())
}

#[test]
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
        .output()
        .map_err(Into::into)
}

fn stdout_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stdout.clone()).map_err(Into::into)
}

fn stderr_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stderr.clone()).map_err(Into::into)
}

use std::error::Error;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const TOKENS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../design/tokens/velvet_ui_tokens.toml"
);

#[test]
fn ui_tokens_stdout_is_json_when_json_emit_is_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("tokens_generated.rs");
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
    assert_eq!(stdout_text(&output)?.contains("\"window_width\""), true);
    assert_eq!(stdout_text(&output)?.contains("pub const TOKENS"), false);
    assert_eq!(
        std::fs::read_to_string(output_path)?.contains("pub const TOKENS"),
        true
    );
    Ok(())
}

#[test]
fn ui_tokens_stdout_is_rust_when_rust_emit_is_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("tokens_generated.rs");
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
            "rust",
        ],
    )?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_text(&output)?.contains("pub const TOKENS"), true);
    assert_eq!(stdout_text(&output)?.contains("\"window_width\""), false);
    assert_eq!(
        std::fs::read_to_string(output_path)?.contains("pub const TOKENS"),
        true
    );
    Ok(())
}

fn run_xtask(current_dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .map_err(Into::into)
}

fn stdout_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stdout.clone()).map_err(Into::into)
}

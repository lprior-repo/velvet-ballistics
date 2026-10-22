#![forbid(unsafe_code)]

//! Current-scope CLI and backend tests for vb-nf2u.
//!
//! Historical UI release acceptance tests have been archived to
//! `docs/history/vb_nf2u_ui_release_acceptance.rs.pre-vb-5aw5i` because
//! the MASTER explicitly states the current scope is Backend / IR
//! Interpreter Complete, with UI/codegen/maxperf removed.
//!
//! These tests verify:
//! - Removed UI command refusal
//! - Core CLI commands function
//! - Current workspace structure (no removed crates as members)

use std::error::Error;
use std::path::Path;
use std::process::Command;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const CLI_BINARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/release/velvet-ballistics");

fn run_cli(args: &[&str]) -> Result<CommandOutput, Box<dyn Error>> {
    let output = Command::new(CLI_BINARY)
        .current_dir(WORKSPACE_ROOT)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run CLI: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok(CommandOutput {
        status: output.status.code(),
        stdout,
        stderr,
    })
}

struct CommandOutput {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    fn success(&self) -> bool {
        self.status == Some(0)
    }

    fn all_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

#[test]
fn removed_ui_command_is_refused_with_unknown_command_error() -> Result<(), Box<dyn Error>> {
    // The `ui` command was removed from the current command surface.
    // Invoking it should produce an "unknown command" error.
    let output = run_cli(&["ui"])?;
    assert!(!output.success(), "ui command should fail");
    let all = output.all_output();
    assert!(
        all.contains("unknown command: ui"),
        "expected 'unknown command: ui' in output, got: {all}"
    );
    Ok(())
}

#[test]
fn removed_ui_release_command_is_not_registered() -> Result<(), Box<dyn Error>> {
    // There is no `ui-release` CLI command in the current scope.
    let output = run_cli(&["ui-release"])?;
    assert!(!output.success(), "ui-release command should fail");
    let all = output.all_output();
    assert!(
        all.contains("unknown command: ui-release"),
        "expected 'unknown command: ui-release' in output, got: {all}"
    );
    Ok(())
}

#[test]
fn cli_help_lists_current_command_surface() -> Result<(), Box<dyn Error>> {
    let output = run_cli(&["help"])?;
    assert!(output.success(), "help command should succeed");
    let stdout = &output.stdout;

    // Core commands that must be present
    assert!(stdout.contains("validate"), "missing validate command");
    assert!(stdout.contains("verify"), "missing verify command");
    assert!(stdout.contains("compile"), "missing compile command");
    assert!(stdout.contains("run"), "missing run command");
    assert!(stdout.contains("replay"), "missing replay command");
    assert!(stdout.contains("inspect"), "missing inspect command");
    assert!(stdout.contains("doctor"), "missing doctor command");

    // Removed UI commands should not be in the list
    assert!(!stdout.contains("  ui "), "ui command should not be listed");
    assert!(!stdout.contains("  ui-release "), "ui-release command should not be listed");
    Ok(())
}

#[test]
fn cli_version_prints_version_string() -> Result<(), Box<dyn Error>> {
    let output = run_cli(&["version"])?;
    assert!(output.success(), "version command should succeed");
    assert!(
        output.stdout.contains("velvet-ballistics"),
        "version output should contain product name"
    );
    Ok(())
}

#[test]
fn validate_command_runs_on_valid_workflow() -> Result<(), Box<dyn Error>> {
    // Create a minimal valid workflow
    let workflow_path = format!("{}/target/vb-nf2u-test-workflow.yaml", WORKSPACE_ROOT);
    let content = "version: velvet-ballistics/v1\nname: nf2u-test\ntriggers: [manual]\nsteps:\n  - id: hello\n    type: print\n    data:\n      message: test\n";
    std::fs::write(&workflow_path, content)?;

    let output = run_cli(&["validate", &workflow_path])?;
    assert!(output.success(), "validate should succeed: {}", output.all_output());
    Ok(())
}

#[test]
fn removed_codegen_ui_crates_are_not_workspace_members() -> Result<(), Box<dyn Error>> {
    let cargo_toml = std::fs::read_to_string(format!("{}/Cargo.toml", WORKSPACE_ROOT))?;

    // vb_codegen, vb_ui_model, vb_ui_makepad are removed from active workspace
    assert!(
        !cargo_toml.contains("\"crates/vb_codegen\""),
        "vb_codegen should not be a workspace member"
    );
    assert!(
        !cargo_toml.contains("\"crates/vb_ui_model\""),
        "vb_ui_model should not be a workspace member"
    );
    assert!(
        !cargo_toml.contains("\"crates/vb_ui_makepad\""),
        "vb_ui_makepad should not be a workspace member"
    );

    // Current scope crates must be members
    assert!(cargo_toml.contains("\"crates/vb_core\""), "vb_core must be a member");
    assert!(
        cargo_toml.contains("\"crates/vb_runtime\""),
        "vb_runtime must be a member"
    );
    assert!(
        cargo_toml.contains("\"crates/velvet_ballistics\""),
        "velvet_ballistics must be a member"
    );
    Ok(())
}

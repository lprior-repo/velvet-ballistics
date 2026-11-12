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
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[path = "support/prebuilt_cli_path.rs"]
mod prebuilt_cli_path;

fn standalone_cli_command() -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(WORKSPACE_ROOT);
    command.args([
        "run",
        "--quiet",
        "--manifest-path",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
        "-p",
        "velvet-ballistics",
        "--",
    ]);
    command
}

fn cli_command(args: &[&str], prebuilt: Option<OsString>) -> Command {
    let root = Path::new(WORKSPACE_ROOT);
    let mut cmd = match prebuilt {
        Some(path) => {
            let resolved = prebuilt_cli_path::resolve_prebuilt_cli(path, root);
            let mut command = Command::new(resolved);
            command.current_dir(root);
            command
        }
        None => standalone_cli_command(),
    };
    cmd.args(args);
    cmd
}

fn run_cli(args: &[&str]) -> Result<CommandOutput, Box<dyn Error>> {
    let prebuilt = prebuilt_cli_path::configured_prebuilt_cli();
    let output = cli_command(args, prebuilt)
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
    assert!(
        !stdout.contains("  ui-release "),
        "ui-release command should not be listed"
    );
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
    let workflow_path = format!("{}/target/vb-nf2u-test-workflow.yaml", WORKSPACE_ROOT);
    let content = "version: velvet-ballistics/v1\nname: nf2u_test\nwhen:\n  manual: {}\nsteps:\n  - id: hello\n    set:\n      output: msg\n      value: \"1\"\n  - id: done\n    finish:\n      result: msg\n";
    std::fs::write(&workflow_path, content)?;

    let output = run_cli(&["validate", &workflow_path])?;
    assert!(
        output.success(),
        "validate should succeed: {}",
        output.all_output()
    );
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
    assert!(
        cargo_toml.contains("\"crates/vb_core\""),
        "vb_core must be a member"
    );
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

#[test]
fn prebuilt_cli_selector_honors_explicit_os_path() {
    let cmd = cli_command(
        &["version"],
        Some(OsString::from("prebuilt-acceptance-cli")),
    );
    let expected = std::path::Path::new(WORKSPACE_ROOT).join("prebuilt-acceptance-cli");
    assert_eq!(cmd.get_program(), expected.as_os_str());
    assert!(
        std::path::Path::new(cmd.get_program()).is_absolute(),
        "explicit path must resolve to absolute"
    );
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        vec![OsStr::new("version")]
    );
}

#[test]
fn absent_path_selects_cargo_with_exact_prefix() {
    let cmd = cli_command(&["help"], None);
    assert_eq!(cmd.get_program(), OsStr::new("cargo"));
    assert_eq!(
        cmd.get_args().collect::<Vec<_>>(),
        vec![
            OsStr::new("run"),
            OsStr::new("--quiet"),
            OsStr::new("--manifest-path"),
            OsStr::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml")),
            OsStr::new("-p"),
            OsStr::new("velvet-ballistics"),
            OsStr::new("--"),
            OsStr::new("help")
        ]
    );
}

#[test]
fn invalid_explicit_path_is_classified_before_spawn() -> Result<(), Box<dyn Error>> {
    let tmp = tempfile::tempdir()?;
    let missing = tmp.path().join("nonexistent-cli-binary");
    let mut cmd = cli_command(&["version"], Some(missing.as_os_str().to_owned()));
    assert_eq!(
        cmd.get_program(),
        missing.as_os_str(),
        "explicit path must be preserved as program"
    );
    let error = cmd
        .output()
        .err()
        .ok_or("nonexistent explicit CLI must fail to spawn")?;
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    Ok(())
}

#[cfg(unix)]
#[test]
fn prebuilt_path_preserves_non_utf8_os_bytes() {
    use std::os::unix::ffi::OsStringExt;
    let non_utf8 = OsString::from_vec(vec![b'x', 0xff]);
    let cmd = cli_command(&["version"], Some(non_utf8.clone()));
    let expected = std::path::Path::new(WORKSPACE_ROOT).join(&non_utf8);
    assert_eq!(cmd.get_program(), expected.as_os_str());
}

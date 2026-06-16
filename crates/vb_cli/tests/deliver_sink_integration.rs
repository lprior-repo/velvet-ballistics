#![forbid(unsafe_code)]
//! End-to-end integration tests for the deliver-sink binary.
//!
//! These tests are the source of truth for behavior that must be verified
//! through the real `velvet-ballistics` binary:
//! - CLI argument parsing and validation (`--deliver`, missing/unknown flags)
//! - Process exit codes and the `deliver failed: ...` stderr envelope
//! - Real-path edge cases that depend on the host filesystem: exact 4094 /
//!   4095 / 4096-byte delivery paths, blocked-root rejection (`/dev`, `/proc`,
//!   `/sys` and their descendants), `/proc/../<allowed-parent>` resolution
//! - The in-process debug hooks activated by the `VB_DELIVER_SINK_TEST_*_ENV`
//!   variables are reachable only from the binary; this file exercises them
//!   end-to-end. They are honored only when the binary is built with the
//!   `instrumented-cli` Cargo feature (e.g. via
//!   `cargo test -p velvet-ballistics --features instrumented-cli`, not a
//!   plain `cargo build` or `cargo build --release`); `assert_test_hooks_active`
//!   below probes and fails loudly if the binary under test does not honor
//!   them.
//!
//! In-process library behavior (every `write_json_line` error branch,
//! rollback and post-commit state contracts, internal helpers) lives in the
//! `tests` module of `crates/vb_cli/src/deliver_sink.rs`, which uses the
//! in-process `HookConfig` API. Scenarios that appear in both files are
//! intentionally redundant only when the integration path adds CLI-specific
//! evidence on top of the library path.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use vb_cli::deliver_sink::PUBLISH_STATE_UNKNOWN_MESSAGE;

const TEST_CLEANUP_FAILURES_ENV: &str = "VB_DELIVER_SINK_TEST_CLEANUP_FAILURES";
const TEST_POST_COMMIT_FINAL_ACTION_ENV: &str = "VB_DELIVER_SINK_TEST_POST_COMMIT_FINAL_ACTION";
const TEST_SYNC_RESULTS_ENV: &str = "VB_DELIVER_SINK_TEST_SYNC_RESULTS";
const RIVAL_REPLACEMENT_TEXT: &str = "rival replacement\n";

fn publish_state_unknown_line() -> String {
    format!("deliver failed: {PUBLISH_STATE_UNKNOWN_MESSAGE}")
}

#[test]
fn agent_context_deliver_stdout_writes_single_json_line() -> Result<(), String> {
    let output = run_agent_context_deliver("stdout")?;

    assert!(
        output.status.success(),
        "agent-context --deliver stdout must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, Vec::<u8>::new());

    assert_agent_context_jsonl_bytes(&output.stdout, "agent-context --deliver stdout stdout")
}

#[test]
fn agent_context_deliver_file_writes_json_without_stdout() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = agent_context_deliver_command(target.as_str())
        .output()
        .map_err(|error| error.to_string())?;

    assert_successful_file_delivery(
        &output,
        &deliver_path,
        &["agent-context.jsonl"],
        "agent-context --deliver",
    )
}

#[test]
fn agent_context_deliver_reports_unknown_publish_when_rival_unlinks_final_path_after_publish()
-> Result<(), String> {
    assert_test_hooks_active()?;
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = agent_context_deliver_command(target.as_str())
        .env(TEST_POST_COMMIT_FINAL_ACTION_ENV, "unlink-final")
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(&output, &publish_state_unknown_line())?;
    assert!(!deliver_path.exists());
    assert_directory_entries_exact(dir.path(), &[])?;
    Ok(())
}

#[test]
fn agent_context_deliver_reports_unknown_publish_when_rival_replaces_final_path_after_publish()
-> Result<(), String> {
    assert_test_hooks_active()?;
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = agent_context_deliver_command(target.as_str())
        .env(TEST_POST_COMMIT_FINAL_ACTION_ENV, "replace-final")
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(&output, &publish_state_unknown_line())?;
    assert_eq!(
        std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?,
        String::from(RIVAL_REPLACEMENT_TEXT)
    );
    assert_directory_entries_exact(dir.path(), &["agent-context.jsonl"])?;
    Ok(())
}

#[test]
fn agent_context_deliver_reports_unknown_publish_when_rollback_leaves_temp_link()
-> Result<(), String> {
    assert_test_hooks_active()?;
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let temp_stage_path = dir.path().join(".agent-context.jsonl.tmp");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = agent_context_deliver_command(target.as_str())
        .env(TEST_SYNC_RESULTS_ENV, "permission_denied,ok")
        .env(TEST_CLEANUP_FAILURES_ENV, ".agent-context.jsonl.tmp")
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(&output, &publish_state_unknown_line())?;
    assert!(!deliver_path.exists());
    assert_agent_context_jsonl_at_path(&temp_stage_path)?;
    assert_directory_entries_exact(dir.path(), &[".agent-context.jsonl.tmp"])?;
    Ok(())
}

#[test]
fn agent_context_deliver_retries_after_preexisting_preferred_stage_file() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let preferred_stage_path = dir.path().join(".agent-context.jsonl.tmp");
    std::fs::write(&preferred_stage_path, "preexisting stage\n")
        .map_err(|error| error.to_string())?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_successful_file_delivery(
        &output,
        &deliver_path,
        &[".agent-context.jsonl.tmp", "agent-context.jsonl"],
        "agent-context --deliver",
    )?;
    assert_eq!(
        std::fs::read_to_string(&preferred_stage_path).map_err(|error| error.to_string())?,
        String::from("preexisting stage\n")
    );
    Ok(())
}

#[test]
fn agent_context_deliver_file_succeeds_at_exact_max_path_bytes_4095_with_staging_room()
-> Result<(), String> {
    let (_root, deliver_path) = actual_exact_path_target(4095, "four")?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(
        output.status.success(),
        "agent-context --deliver at 4095 bytes must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(path_text(&deliver_path)?.len(), 4095);

    assert_successful_file_delivery(
        &output,
        &deliver_path,
        &["four"],
        "agent-context --deliver at 4095 bytes",
    )
}

#[test]
fn agent_context_deliver_rejects_exact_4095_byte_path_without_staging_retry_room()
-> Result<(), String> {
    let (_root, deliver_path) = actual_exact_path_target(4095, "f")?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(&output, "deliver failed: deliver file path is too long")?;
    assert!(!deliver_path.exists());
    Ok(())
}

#[test]
fn agent_context_deliver_accepts_exact_4094_byte_path_when_short_stage_name_still_fits()
-> Result<(), String> {
    let (_root, deliver_path) = actual_exact_path_target(4094, "f")?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(
        output.status.success(),
        "agent-context --deliver at 4094 bytes must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(path_text(&deliver_path)?.len(), 4094);

    assert_successful_file_delivery(
        &output,
        &deliver_path,
        &["f"],
        "agent-context --deliver at 4094 bytes",
    )
}

#[test]
fn agent_context_deliver_reports_staging_unavailable_for_4094_byte_path_when_short_stage_is_taken()
-> Result<(), String> {
    let (_root, deliver_path) = actual_exact_path_target(4094, "f")?;
    let parent = deliver_path
        .parent()
        .ok_or_else(|| String::from("deliver path is missing parent"))?;
    let short_stage_path = parent.join(".t");
    std::fs::write(&short_stage_path, "preexisting short stage\n")
        .map_err(|error| error.to_string())?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = run_agent_context_deliver(&target)?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(
        &output,
        "deliver failed: deliver temporary staging path is unavailable",
    )?;
    assert!(!deliver_path.exists());
    assert_eq!(
        std::fs::read_to_string(&short_stage_path).map_err(|error| error.to_string())?,
        String::from("preexisting short stage\n")
    );
    assert_directory_entries_exact(parent, &[".t"])?;
    Ok(())
}

#[test]
fn agent_context_deliver_accepts_251_to_255_byte_file_names_with_shorter_stage_fallback()
-> Result<(), String> {
    for file_name_len in 251..=255 {
        let dir = deliver_tempdir()?;
        let file_name = repeated_file_name(file_name_len);
        let deliver_path = dir.path().join(&file_name);
        let target = format!("file:{}", path_text(&deliver_path)?);
        let output = run_agent_context_deliver(&target)?;
        let expected_parent_entries = [file_name.as_str()];
        let label = format!("agent-context --deliver with {file_name_len}-byte file name");

        assert_successful_file_delivery(&output, &deliver_path, &expected_parent_entries, &label)?;
    }

    Ok(())
}

#[test]
fn agent_context_deliver_handles_256_byte_file_name_at_parent_component_boundary()
-> Result<(), String> {
    let dir = deliver_tempdir()?;
    let file_name = repeated_file_name(256);
    let deliver_path = dir.path().join(&file_name);
    let target = format!("file:{}", path_text(&deliver_path)?);
    let file_name_supported = probe_file_name_support(dir.path(), &file_name)?;
    let output = run_agent_context_deliver(&target)?;

    if file_name_supported {
        let expected_parent_entries = [file_name.as_str()];
        assert_successful_file_delivery(
            &output,
            &deliver_path,
            &expected_parent_entries,
            "agent-context --deliver with 256-byte file name",
        )
    } else {
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_stderr_line(&output, "deliver failed: deliver file path is too long")?;
        assert_directory_entries_exact(dir.path(), &[])
    }
}

#[test]
fn agent_context_deliver_rejects_path_just_above_4095_bytes() -> Result<(), String> {
    let (_root, deliver_path) = actual_exact_path_target(4096, "f")?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(&output, "deliver failed: deliver file path is too long")?;
    assert!(!deliver_path.exists());
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_unknown_flag_before_writing_file() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str(), "--bogus"])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_starts_with_line(
        &output,
        "invalid agent-context argument: unknown flag --bogus",
    )?;
    assert!(!deliver_path.exists());
    assert_directory_entries_exact(dir.path(), &[])?;
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_missing_target() -> Result<(), String> {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver"])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_starts_with_line(
        &output,
        "invalid agent-context argument: --deliver requires stdout, file:<absolute-path>, or webhook:<url>",
    )?;
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_bare_unschemed_target() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    assert_deliver_validation_failure(
        &path_text(&deliver_path)?,
        "deliver failed: deliver target must be stdout, file:<path>, or webhook:<url>",
    )
}

#[test]
fn agent_context_deliver_rejects_empty_file_target() -> Result<(), String> {
    assert_deliver_validation_failure(
        "file:",
        "deliver failed: deliver file target is missing a path",
    )
}

#[test]
fn agent_context_deliver_rejects_unsupported_webhook_target() -> Result<(), String> {
    assert_deliver_validation_failure(
        "webhook:https://example.invalid/agent-context",
        "deliver failed: deliver webhook target is not supported yet",
    )
}

#[test]
fn agent_context_deliver_rejects_stdout_with_trailing_colon() -> Result<(), String> {
    // Contract decision: the only canonical stdout form is the bare token
    // `stdout`. A colon-suffixed variant (`stdout:`) is parsed as the unknown
    // scheme `stdout` with an empty value, so it must surface
    // `UnknownScheme` rather than silently route to stdout or fall through
    // to `MissingFilePath`.
    assert_deliver_validation_failure(
        "stdout:",
        "deliver failed: deliver target scheme is unknown",
    )
}

#[test]
fn agent_context_deliver_rejects_unknown_target_scheme() -> Result<(), String> {
    assert_deliver_validation_failure(
        "ftp:/absolute/agent-context.jsonl",
        "deliver failed: deliver target scheme is unknown",
    )
}

#[test]
fn agent_context_deliver_rejects_relative_file_target() -> Result<(), String> {
    assert_deliver_validation_failure(
        "file:agent-context.jsonl",
        "deliver failed: deliver file path must be absolute",
    )
}

#[test]
fn agent_context_deliver_rejects_missing_parent_directory() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir
        .path()
        .join("missing-parent")
        .join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);
    assert_deliver_validation_failure(
        &target,
        "deliver failed: deliver file parent directory is missing",
    )
}

#[test]
fn agent_context_deliver_rejects_directory_target() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let target = format!("file:{}", path_text(dir.path())?);
    assert_deliver_validation_failure(
        &target,
        "deliver failed: deliver file target is a directory",
    )
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_rejects_symlink_alias_to_blocked_root() -> Result<(), String> {
    let Some(blocked_root) = blocked_root_symlink_target() else {
        return Ok(());
    };

    let dir = deliver_tempdir()?;
    let blocked_root_alias = dir.path().join("blocked-root-alias");
    std::os::unix::fs::symlink(blocked_root, &blocked_root_alias)
        .map_err(|error| error.to_string())?;
    let deliver_path = blocked_root_alias.join("vb-blocked-root-alias.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(
        &output,
        "deliver failed: deliver file path uses a blocked system root",
    )?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_rejects_dev_root_directly() -> Result<(), String> {
    assert_direct_blocked_root_rejection(Path::new("/dev"), "vb-direct-dev.jsonl")
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_rejects_sys_root_directly() -> Result<(), String> {
    assert_direct_blocked_root_rejection(Path::new("/sys"), "vb-direct-sys.jsonl")
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_rejects_proc_descendant_path() -> Result<(), String> {
    assert_direct_blocked_root_rejection(Path::new("/proc/self"), "vb-proc-self.jsonl")
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_rejects_sys_descendant_path() -> Result<(), String> {
    assert_direct_blocked_root_rejection(Path::new("/sys/kernel"), "vb-sys-kernel.jsonl")
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_rejects_dev_descendant_path() -> Result<(), String> {
    assert_direct_blocked_root_rejection(Path::new("/dev/shm"), "vb-dev-shm.jsonl")
}

#[cfg(unix)]
#[test]
fn agent_context_deliver_accepts_symlink_alias_to_allowed_parent() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let real_parent = dir.path().join("real-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    let alias_parent = dir.path().join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).map_err(|error| error.to_string())?;

    let deliver_path = alias_parent.join("agent-context.jsonl");
    let actual_path = real_parent.join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(deliver_path.exists());

    assert_successful_file_delivery(
        &output,
        &actual_path,
        &["agent-context.jsonl"],
        "agent-context --deliver via symlink alias",
    )
}

#[cfg(unix)]
#[test]
fn agent_context_deliver_resolves_parent_as_written_before_collapsing_dotdot() -> Result<(), String>
{
    let dir = deliver_tempdir()?;
    let real_root = dir.path().join("real-root");
    let real_inner = real_root.join("inner");
    let real_parent = real_root.join("deliver-parent");
    std::fs::create_dir_all(&real_inner).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&real_parent).map_err(|error| error.to_string())?;
    let alias_inner = dir.path().join("alias-inner");
    std::os::unix::fs::symlink(&real_inner, &alias_inner).map_err(|error| error.to_string())?;

    let deliver_path = alias_inner
        .join("..")
        .join("deliver-parent")
        .join("agent-context.jsonl");
    let actual_path = real_parent.join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_successful_file_delivery(
        &output,
        &actual_path,
        &["agent-context.jsonl"],
        "agent-context --deliver via symlink/.. parent",
    )
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_allows_proc_dotdot_tmp_path_when_resolved_parent_is_allowed()
-> Result<(), String> {
    let dir = repo_tempdir("vb-deliver-proc-dotdot-")?;
    let actual_path = dir.path().join("agent-context.jsonl");
    let deliver_path = proc_dotdot_alias(&actual_path)?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_successful_file_delivery(
        &output,
        &actual_path,
        &["agent-context.jsonl"],
        "agent-context --deliver must use resolved parent semantics for /proc/../<repo-path>",
    )
}

#[cfg(unix)]
#[test]
fn agent_context_deliver_rejects_symlink_alias_to_existing_real_file() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let real_parent = dir.path().join("real-parent");
    std::fs::create_dir(&real_parent).map_err(|error| error.to_string())?;
    let alias_parent = dir.path().join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).map_err(|error| error.to_string())?;

    let actual_path = real_parent.join("agent-context.jsonl");
    std::fs::write(&actual_path, "already here\n").map_err(|error| error.to_string())?;
    let deliver_path = alias_parent.join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(
        &output,
        "deliver failed: deliver file target already exists",
    )?;
    assert_eq!(
        std::fs::read_to_string(&actual_path).map_err(|error| error.to_string())?,
        String::from("already here\n")
    );
    assert_directory_entries_exact(&real_parent, &["agent-context.jsonl"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn blocked_root_symlink_target() -> Option<&'static std::path::Path> {
    let candidate = std::path::Path::new("/proc");
    if candidate.is_dir() {
        Some(candidate)
    } else {
        None
    }
}

fn path_text(path: &std::path::Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| String::from("test path must be UTF-8"))
}

fn run_agent_context_deliver(target: &str) -> Result<std::process::Output, String> {
    agent_context_deliver_command(target)
        .output()
        .map_err(|error| error.to_string())
}

fn agent_context_deliver_command(target: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args(["agent-context", "--deliver", target]);
    command
}

fn run_agent_context() -> Result<std::process::Output, String> {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("agent-context")
        .output()
        .map_err(|error| error.to_string())
}

#[derive(Clone, PartialEq, Eq)]
struct BinaryFingerprint {
    resolved_path: PathBuf,
    mtime: SystemTime,
    size: u64,
}

fn binary_fingerprint() -> Result<BinaryFingerprint, String> {
    let raw = Path::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    let resolved = std::fs::canonicalize(raw).map_err(|error| {
        format!("deliver-sink test-hook probe could not canonicalize {raw:?}: {error}")
    })?;
    let metadata = std::fs::metadata(&resolved).map_err(|error| {
        format!(
            "deliver-sink test-hook probe could not stat {}: {error}",
            resolved.display()
        )
    })?;
    let mtime = metadata.modified().map_err(|error| {
        format!(
            "deliver-sink test-hook probe could not read mtime of {}: {error}",
            resolved.display()
        )
    })?;
    Ok(BinaryFingerprint {
        resolved_path: resolved,
        mtime,
        size: metadata.len(),
    })
}

fn probe_test_hooks_active() -> Result<(), String> {
    let dir = match deliver_tempdir() {
        Ok(d) => d,
        Err(error) => {
            return Err(format!(
                "deliver-sink test-hook probe could not create tempdir: {error}"
            ));
        }
    };
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = match path_text(&deliver_path) {
        Ok(text) => format!("file:{text}"),
        Err(error) => {
            return Err(format!(
                "deliver-sink test-hook probe could not format target: {error}"
            ));
        }
    };
    let output = match agent_context_deliver_command(&target)
        .env(TEST_POST_COMMIT_FINAL_ACTION_ENV, "unlink-final")
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return Err(format!(
                "deliver-sink test-hook probe could not invoke binary: {error}"
            ));
        }
    };
    let expected_stderr = format!("deliver failed: {PUBLISH_STATE_UNKNOWN_MESSAGE}\n");
    if output.status.code() == Some(2) && output.stderr == expected_stderr.as_bytes() {
        Ok(())
    } else {
        Err(String::from(
            "VB_DELIVER_SINK_TEST_* env vars are not honored by the velvet-ballistics binary under test: \
             the `debug_test_support` module in crates/vb_cli/src/deliver_sink.rs is gated by \
             `#[cfg(all(not(test), feature = \"instrumented-cli\"))]`, so the hooks are only compiled when \
             the binary is built with the `instrumented-cli` Cargo feature. Rebuild with \
             `cargo test -p velvet-ballistics --features instrumented-cli` to enable the hooks.",
        ))
    }
}

fn assert_test_hooks_active() -> Result<(), String> {
    // The probe is cached so subsequent tests skip the binary invocation, but
    // a `cargo build` between tests would replace the binary on disk and the
    // old cached verdict would silently apply to the new binary. The cache
    // is therefore keyed by a binary fingerprint (resolved path + mtime +
    // size) so a rebuild invalidates the entry and forces a fresh probe.
    static PROBE: OnceLock<Mutex<Option<(BinaryFingerprint, Result<(), String>)>>> =
        OnceLock::new();
    let current = binary_fingerprint()?;
    let cell = PROBE.get_or_init(|| Mutex::new(None));
    let mut guard = cell
        .lock()
        .map_err(|error| format!("deliver-sink test-hook probe lock poisoned: {error}"))?;
    if let Some((fingerprint, result)) = guard.as_ref() {
        if *fingerprint == current {
            return result.clone();
        }
    }
    let result = probe_test_hooks_active();
    *guard = Some((current, result.clone()));
    result
}

fn assert_deliver_validation_failure(target: &str, expected_message: &str) -> Result<(), String> {
    let output = run_agent_context_deliver(target)?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(&output, expected_message)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn assert_direct_blocked_root_rejection(parent: &Path, file_name: &str) -> Result<(), String> {
    // The helper is `#[cfg(target_os = "linux")]`-gated because the deliver
    // sink's blocked-root rejection only makes sense on Linux. Fail loudly
    // when the host is Linux but the expected path is missing (e.g. running
    // inside a stripped container) so the skipped test is not mistaken for a
    // passing one.
    if !parent.is_dir() {
        return Err(format!(
            "blocked-root precondition unmet: {} must be an existing directory to verify \
             deliver-sink rejection of {}/{}; the helper is gated by `#[cfg(target_os = \"linux\")]` \
             and is only meaningful on Linux hosts where the path is present",
            parent.display(),
            parent.display(),
            file_name,
        ));
    }

    let deliver_path = parent.join(file_name);
    let output = run_agent_context_deliver(&format!("file:{}", path_text(&deliver_path)?))?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_stderr_line(
        &output,
        "deliver failed: deliver file path uses a blocked system root",
    )
}

fn deliver_tempdir() -> Result<tempfile::TempDir, String> {
    repo_tempdir("vb-deliver-")
}

fn repo_temp_root() -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deliver-sink-tmp");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    std::fs::canonicalize(&root).map_err(|error| error.to_string())
}

fn repo_tempdir(prefix: &str) -> Result<tempfile::TempDir, String> {
    let root = repo_temp_root()?;
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(root)
        .map_err(|error| error.to_string())
}

fn assert_successful_file_delivery(
    output: &std::process::Output,
    deliver_path: &Path,
    expected_parent_entries: &[&str],
    label: &str,
) -> Result<(), String> {
    if !output.status.success() {
        return Err(format!(
            "{label} must succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if output.stdout != Vec::<u8>::new() {
        return Err(format!(
            "{label} must not write stdout for file delivery, got {:?}",
            output.stdout
        ));
    }
    if output.stderr != Vec::<u8>::new() {
        return Err(format!(
            "{label} must not write stderr on success, got {:?}",
            output.stderr
        ));
    }

    assert_agent_context_jsonl_at_path(deliver_path)?;
    #[cfg(unix)]
    assert_owner_only_file_permissions(deliver_path)?;
    let parent = deliver_path
        .parent()
        .ok_or_else(|| String::from("deliver path is missing parent"))?;
    assert_directory_entries_exact(parent, expected_parent_entries)
}

#[cfg(unix)]
fn assert_owner_only_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .permissions()
        .mode()
        & 0o777;
    if mode & 0o600 == 0o600 && mode & 0o077 == 0 {
        Ok(())
    } else {
        Err(format!(
            "expected owner-only delivered file permissions at {}, got {mode:o}",
            path.display()
        ))
    }
}

fn expected_agent_context_value() -> Result<serde_json::Value, String> {
    let output = run_agent_context()?;
    if !output.status.success() {
        return Err(format!(
            "agent-context must succeed when building expected value, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if output.stderr != Vec::<u8>::new() {
        return Err(format!(
            "agent-context must not write stderr when building expected value, got {:?}",
            output.stderr
        ));
    }

    serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())
}

fn expected_agent_context_jsonl_bytes() -> Result<Vec<u8>, String> {
    let mut bytes =
        serde_json::to_vec(&expected_agent_context_value()?).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn assert_agent_context_jsonl_at_path(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    assert_agent_context_jsonl_bytes(&bytes, &path.display().to_string())
}

fn assert_agent_context_jsonl_bytes(bytes: &[u8], location: &str) -> Result<(), String> {
    let _ = parse_single_jsonl_record_bytes(bytes, location)?;
    let expected = expected_agent_context_jsonl_bytes()?;
    if bytes == expected.as_slice() {
        Ok(())
    } else {
        Err(format!(
            "expected exact JSONL bytes {:?} at {location}, got {:?}",
            expected, bytes
        ))
    }
}

fn assert_stderr_line(output: &std::process::Output, expected_line: &str) -> Result<(), String> {
    let expected = format!("{expected_line}\n");
    if output.stderr == expected.as_bytes() {
        Ok(())
    } else {
        Err(format!(
            "expected stderr {:?}, got {:?}",
            expected.as_bytes(),
            output.stderr
        ))
    }
}

fn assert_stderr_starts_with_line(
    output: &std::process::Output,
    expected_line: &str,
) -> Result<(), String> {
    let expected = format!("{expected_line}\n");
    if output.stderr.starts_with(expected.as_bytes()) {
        Ok(())
    } else {
        Err(format!(
            "expected stderr to start with {:?}, got {:?}",
            expected.as_bytes(),
            output.stderr
        ))
    }
}

fn parse_single_jsonl_record_bytes(
    bytes: &[u8],
    location: &str,
) -> Result<(String, serde_json::Value), String> {
    if bytes.last().copied() != Some(b'\n') {
        return Err(format!(
            "JSONL payload at {location} must end with a trailing newline"
        ));
    }
    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    if newline_count != 1 {
        return Err(format!(
            "JSONL payload at {location} must contain exactly one newline-delimited record"
        ));
    }
    let text = String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())?;
    let line = text
        .strip_suffix('\n')
        .ok_or_else(|| format!("JSONL payload at {location} lost its trailing newline"))?;
    if line.is_empty() {
        return Err(format!(
            "JSONL payload at {location} must contain exactly one JSON record"
        ));
    }
    if line.contains('\n') || line.contains('\r') {
        return Err(format!(
            "JSONL payload at {location} must use a single LF-delimited record"
        ));
    }
    let value = serde_json::from_str(line).map_err(|error| error.to_string())?;
    Ok((text, value))
}

fn assert_directory_entries_exact(directory: &Path, expected: &[&str]) -> Result<(), String> {
    let mut actual_entries = Vec::new();
    for entry_result in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry_result.map_err(|error| error.to_string())?;
        actual_entries.push(entry.file_name().to_string_lossy().into_owned());
    }
    actual_entries.sort();

    let mut expected_entries = expected
        .iter()
        .map(|entry| String::from(*entry))
        .collect::<Vec<_>>();
    expected_entries.sort();

    if actual_entries == expected_entries {
        Ok(())
    } else {
        Err(format!(
            "expected directory entries {:?} at {}, got {:?}",
            expected_entries,
            directory.display(),
            actual_entries
        ))
    }
}

fn repeated_file_name(byte_len: usize) -> String {
    "f".repeat(byte_len)
}

fn probe_file_name_support(parent: &Path, file_name: &str) -> Result<bool, String> {
    let probe_path = parent.join(file_name);
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
    {
        Ok(file) => {
            drop(file);
            std::fs::remove_file(&probe_path).map_err(|error| error.to_string())?;
            Ok(true)
        }
        Err(error) => {
            if error.raw_os_error() == Some(rustix::io::Errno::NAMETOOLONG.raw_os_error()) {
                Ok(false)
            } else {
                Err(format!(
                    "failed to probe file-name support for {}: {error}",
                    probe_path.display()
                ))
            }
        }
    }
}

fn actual_exact_path_target(
    total_bytes: usize,
    file_name: &str,
) -> Result<(tempfile::TempDir, PathBuf), String> {
    let root = repo_tempdir("vb-path-")?;
    let separator_bytes = 1_usize;
    let target_parent_len = total_bytes
        .checked_sub(file_name.len())
        .and_then(|value| value.checked_sub(separator_bytes))
        .ok_or_else(|| String::from("target path length underflow"))?;
    let parent = grow_parent_to_len(root.path(), target_parent_len)?;
    let deliver_path = parent.join(file_name);
    if path_text(&deliver_path)?.len() != total_bytes {
        return Err(format!(
            "expected exact {total_bytes}-byte path, got {} bytes",
            path_text(&deliver_path)?.len()
        ));
    }
    Ok((root, deliver_path))
}

fn grow_parent_to_len(root: &Path, target_len: usize) -> Result<PathBuf, String> {
    const MAX_SEGMENT_LEN: usize = 200;

    let mut parent = root.to_path_buf();
    let mut current_len = path_text(&parent)?.len();
    while current_len < target_len {
        let remaining = target_len
            .checked_sub(current_len)
            .ok_or_else(|| String::from("parent path length underflow"))?;
        let max_add = MAX_SEGMENT_LEN + 1;
        let add = if remaining <= max_add {
            remaining
        } else if remaining.checked_sub(max_add) == Some(1) {
            max_add - 1
        } else {
            max_add
        };
        if add < 2 {
            return Err(format!(
                "cannot extend path from {} bytes to {} bytes",
                current_len, target_len
            ));
        }
        let segment = "d".repeat(add - 1);
        parent = parent.join(segment);
        std::fs::create_dir(&parent).map_err(|error| error.to_string())?;
        current_len = path_text(&parent)?.len();
    }
    if current_len != target_len {
        return Err(format!(
            "expected parent path length {target_len}, got {current_len}"
        ));
    }
    Ok(parent)
}

#[cfg(target_os = "linux")]
fn proc_dotdot_alias(path: &Path) -> Result<PathBuf, String> {
    let relative = path
        .strip_prefix(Path::new("/"))
        .map_err(|error| error.to_string())?;
    Ok(Path::new("/proc").join("..").join(relative))
}

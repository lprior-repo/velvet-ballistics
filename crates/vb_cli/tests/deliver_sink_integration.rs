#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_TEMP_STAGE_ATTEMPTS: usize = 8;

#[test]
fn agent_context_deliver_stdout_writes_single_json_line() -> Result<(), String> {
    let output = run_agent_context_deliver("stdout")?;

    assert!(
        output.status.success(),
        "agent-context --deliver stdout must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, Vec::<u8>::new());

    let (_, value) =
        parse_single_jsonl_record_bytes(&output.stdout, "agent-context --deliver stdout stdout")?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    assert_eq!(
        value.get("cli"),
        Some(&serde_json::json!("velvet-ballistics"))
    );
    Ok(())
}

#[test]
fn agent_context_deliver_file_writes_json_without_stdout() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(
        output.status.success(),
        "agent-context --deliver must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, Vec::<u8>::new());

    let (_, value) = read_single_jsonl_record(&deliver_path)?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    assert_eq!(
        value.get("cli"),
        Some(&serde_json::json!("velvet-ballistics"))
    );
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

    assert!(
        output.status.success(),
        "agent-context --deliver must retry around a stage-file collision, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(
        std::fs::read_to_string(&preferred_stage_path).map_err(|error| error.to_string())?,
        String::from("preexisting stage\n")
    );

    let (_, value) = read_single_jsonl_record(&deliver_path)?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    Ok(())
}

#[test]
fn agent_context_deliver_file_succeeds_at_exact_max_path_bytes_4095_with_staging_room(
) -> Result<(), String> {
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

    let (_, value) = read_single_jsonl_record(&deliver_path)?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_exact_4095_byte_path_without_staging_retry_room(
) -> Result<(), String> {
    let (_root, deliver_path) = actual_exact_path_target(4095, "f")?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("deliver failed: deliver file path is too long"));
    assert!(!deliver_path.exists());
    Ok(())
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
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("deliver failed: deliver file path is too long"));
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

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("invalid agent-context argument: unknown flag --bogus"));
    assert!(!deliver_path.exists());
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_missing_target() -> Result<(), String> {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver"])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--deliver requires stdout, file:<absolute-path>, or webhook:<url>"));
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
        "webhook:",
        "deliver failed: deliver webhook target is not supported yet",
    )
}

#[test]
fn agent_context_deliver_rejects_unknown_target_scheme() -> Result<(), String> {
    assert_deliver_validation_failure(
        "ftp:/tmp/agent-context.jsonl",
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

#[cfg(unix)]
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
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("deliver failed: deliver file path uses a blocked system root"));
    Ok(())
}

#[test]
fn agent_context_deliver_reports_staging_unavailable_when_all_stage_names_are_taken(
) -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    occupy_all_stage_names(&deliver_path)?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("deliver failed: deliver temporary staging path is unavailable"));
    assert!(!deliver_path.exists());
    Ok(())
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

    assert!(
        output.status.success(),
        "agent-context --deliver via symlink alias must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(deliver_path.exists());

    let (_, value) = read_single_jsonl_record(&actual_path)?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    assert_eq!(
        value.get("cli"),
        Some(&serde_json::json!("velvet-ballistics"))
    );
    Ok(())
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

    assert!(
        output.status.success(),
        "agent-context --deliver via symlink/.. parent must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, Vec::<u8>::new());

    let (_, value) = read_single_jsonl_record(&actual_path)?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn agent_context_deliver_allows_proc_dotdot_tmp_path_when_resolved_parent_is_allowed(
) -> Result<(), String> {
    let dir = tempfile::Builder::new()
        .prefix("vb-deliver-proc-dotdot-")
        .tempdir_in("/tmp")
        .map_err(|error| error.to_string())?;
    let relative_parent = dir
        .path()
        .strip_prefix("/tmp")
        .map_err(|error| error.to_string())?;
    let deliver_path = std::path::Path::new("/proc")
        .join("..")
        .join("tmp")
        .join(relative_parent)
        .join("agent-context.jsonl");
    let actual_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(
        output.status.success(),
        "agent-context --deliver must use resolved parent semantics for /proc/../tmp, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, Vec::<u8>::new());

    let (_, value) = read_single_jsonl_record(&actual_path)?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    Ok(())
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
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("deliver failed: deliver file target already exists"));
    assert_eq!(
        std::fs::read_to_string(&actual_path).map_err(|error| error.to_string())?,
        String::from("already here\n")
    );
    Ok(())
}

#[cfg(unix)]
fn blocked_root_symlink_target() -> Option<&'static std::path::Path> {
    let candidate = std::path::Path::new("/proc");
    if cfg!(target_os = "linux") && candidate.is_dir() {
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
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target])
        .output()
        .map_err(|error| error.to_string())
}

fn assert_deliver_validation_failure(target: &str, expected_message: &str) -> Result<(), String> {
    let output = run_agent_context_deliver(target)?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected_message),
        "expected `{expected_message}` in stderr, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn deliver_tempdir() -> Result<tempfile::TempDir, String> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deliver-sink-tmp");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    tempfile::Builder::new()
        .prefix("vb-deliver-")
        .tempdir_in(root)
        .map_err(|error| error.to_string())
}

fn read_single_jsonl_record(path: &Path) -> Result<(String, serde_json::Value), String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    parse_single_jsonl_record_bytes(&bytes, &path.display().to_string())
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

fn occupy_all_stage_names(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| String::from("deliver path is missing parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| String::from("deliver path is missing file name"))?;
    let resolved_parent = std::fs::canonicalize(parent).map_err(|error| error.to_string())?;
    let resolved_path = resolved_parent.join(file_name);
    let base_names = [
        preferred_temp_name(file_name),
        hashed_temp_name(&resolved_path),
        OsString::from(".tmp"),
        OsString::from(".t"),
    ];

    for base_name in base_names {
        for attempt in 0..MAX_TEMP_STAGE_ATTEMPTS {
            let candidate = resolved_parent.join(temp_stage_name(&base_name, attempt));
            std::fs::write(&candidate, b"occupied stage\n").map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn preferred_temp_name(file_name: &OsStr) -> OsString {
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    temp_name
}

fn hashed_temp_name(path: &Path) -> OsString {
    let digest = blake3::hash(path.as_os_str().as_encoded_bytes());
    let mut temp_name = String::from(".vb");
    for byte in &digest.as_bytes()[..8] {
        temp_name.push_str(&format!("{byte:02x}"));
    }
    OsString::from(temp_name)
}

fn temp_stage_name(base_name: &OsStr, attempt: usize) -> OsString {
    if attempt == 0 {
        return base_name.to_os_string();
    }

    let mut candidate_name = base_name.to_os_string();
    candidate_name.push(".");
    candidate_name.push(attempt.to_string());
    candidate_name
}

fn actual_exact_path_target(
    total_bytes: usize,
    file_name: &str,
) -> Result<(tempfile::TempDir, PathBuf), String> {
    let root = tempfile::Builder::new()
        .prefix("vb-path-")
        .tempdir_in("/tmp")
        .map_err(|error| error.to_string())?;
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

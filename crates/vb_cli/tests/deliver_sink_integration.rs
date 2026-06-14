#![forbid(unsafe_code)]

use std::process::Command;

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

    let delivered = std::fs::read_to_string(&deliver_path).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(delivered.trim_end()).map_err(|error| error.to_string())?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    assert_eq!(
        value.get("cli"),
        Some(&serde_json::json!("velvet-ballistics"))
    );
    assert_eq!(delivered.lines().count(), 1);
    Ok(())
}

#[test]
fn agent_context_deliver_file_succeeds_at_exact_max_path_bytes_4096() -> Result<(), String> {
    let root = deliver_tempdir()?.keep();
    let (deliver_path, actual_path) = exact_max_path_target(&root, 4096)?;
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["agent-context", "--deliver", target.as_str()])
        .output()
        .map_err(|error| error.to_string())?;

    assert!(
        output.status.success(),
        "agent-context --deliver at 4096 bytes must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(path_text(&deliver_path)?.len(), 4096);

    let delivered = std::fs::read_to_string(&actual_path).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(delivered.trim_end()).map_err(|error| error.to_string())?;
    assert_eq!(value.get("kind"), Some(&serde_json::json!("AgentContext")));
    assert_eq!(delivered.lines().count(), 1);
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
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("invalid agent-context argument: unknown flag --bogus")
    );
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
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--deliver requires stdout, file:<absolute-path>, or webhook:<url>")
    );
    Ok(())
}

fn path_text(path: &std::path::Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| String::from("test path must be UTF-8"))
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

fn exact_max_path_target(
    root: &std::path::Path,
    total_bytes: usize,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let parent = root.join("exact-max-parent");
    std::fs::create_dir_all(&parent).map_err(|error| error.to_string())?;

    for file_name_bytes in [32_usize, 31_usize] {
        let file_name = "f".repeat(file_name_bytes);
        let base_target = parent.join(&file_name);
        let padding_bytes = total_bytes
            .checked_sub(path_text(&base_target)?.len())
            .ok_or_else(|| String::from("base target already exceeds target length"))?;
        if padding_bytes % 2 != 0 {
            continue;
        }

        let mut raw_parent = path_text(&parent)?;
        raw_parent.push_str(&"/.".repeat(padding_bytes / 2));
        let raw_target = format!("{raw_parent}/{file_name}");
        if raw_target.len() == total_bytes {
            return Ok((std::path::PathBuf::from(raw_target), base_target));
        }
    }

    Err(String::from(
        "could not construct exact 4096-byte path alias",
    ))
}

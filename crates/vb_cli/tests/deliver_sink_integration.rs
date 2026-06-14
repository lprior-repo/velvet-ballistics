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

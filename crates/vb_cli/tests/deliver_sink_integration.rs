#![forbid(unsafe_code)]

use std::process::Command;

#[test]
fn agent_context_deliver_file_writes_json_without_stdout() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"))
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
        Some(&serde_json::json!("velvet-ballastics"))
    );
    assert_eq!(delivered.lines().count(), 1);
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_unknown_flag_before_writing_file() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let deliver_path = dir.path().join("agent-context.jsonl");
    let target = format!("file:{}", path_text(&deliver_path)?);

    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"))
        .args(["agent-context", "--deliver", target.as_str(), "--bogus"])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(
        output.status.code(),
        Some(2),
        "unknown flag must exit with ValidationFailed (2)"
    );
    assert_eq!(
        output.stdout,
        Vec::<u8>::new(),
        "stdout must be empty on error"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected_prefix =
        "invalid agent-context argument: unknown flag --bogus\n\nvelvet-ballastics - compiled workflow runtime";
    assert!(
        stderr.starts_with(expected_prefix),
        "stderr must start with exact error prefix\ngot: {stderr}"
    );
    assert!(
        stderr.lines().count() > 5,
        "stderr must include help text after error"
    );
    assert!(!deliver_path.exists(), "artifact must not exist on error");
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_missing_target() -> Result<(), String> {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"))
        .args(["agent-context", "--deliver"])
        .output()
        .map_err(|error| error.to_string())?;

    assert_eq!(
        output.status.code(),
        Some(2),
        "missing target must exit with ValidationFailed (2)"
    );
    assert_eq!(
        output.stdout,
        Vec::<u8>::new(),
        "stdout must be empty on error"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected_prefix =
        "invalid agent-context argument: --deliver requires stdout or file:<absolute-path>\n\nvelvet-ballastics - compiled workflow runtime";
    assert!(
        stderr.starts_with(expected_prefix),
        "stderr must start with exact error prefix\ngot: {stderr}"
    );
    assert!(
        stderr.lines().count() > 5,
        "stderr must include help text after error"
    );
    Ok(())
}

#[test]
fn agent_context_deliver_rejects_sink_variants_with_exact_diagnostics() -> Result<(), String> {
    let dir = deliver_tempdir()?;
    let existing = dir.path().join("existing.jsonl");
    std::fs::write(&existing, b"already-present").map_err(|error| error.to_string())?;
    let missing_parent = dir.path().join("missing").join("out.jsonl");

    let cases = [
        (
            "missing scheme",
            path_text(&dir.path().join("plain.jsonl"))?,
            "deliver failed: deliver target must be stdout or file:<path>",
        ),
        (
            "unknown scheme",
            String::from("s3:/tmp/vb-agent-context.jsonl"),
            "deliver failed: deliver target scheme is unknown",
        ),
        (
            "unsupported webhook",
            String::from("webhook:https://example.invalid/hook"),
            "deliver failed: deliver webhook target is not supported yet",
        ),
        (
            "relative path",
            String::from("file:relative.jsonl"),
            "deliver failed: deliver file path must be absolute",
        ),
        (
            "missing file path",
            String::from("file:"),
            "deliver failed: deliver file target is missing a path",
        ),
        (
            "missing parent",
            format!("file:{}", path_text(&missing_parent)?),
            "deliver failed: deliver file parent directory is missing",
        ),
        (
            "directory target",
            format!("file:{}", path_text(dir.path())?),
            "deliver failed: deliver file target is a directory",
        ),
        (
            "blocked path",
            String::from("file:/dev/vb-agent-context.jsonl"),
            "deliver failed: deliver file path uses a blocked system root",
        ),
        (
            "existing file",
            format!("file:{}", path_text(&existing)?),
            "deliver failed: deliver file target already exists",
        ),
    ];

    for (label, target, expected_stderr) in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"))
            .args(["agent-context", "--deliver", target.as_str()])
            .output()
            .map_err(|error| error.to_string())?;

        assert_eq!(
            output.status.code(),
            Some(2),
            "{label} should be rejected as validation failure"
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("{expected_stderr}\n"),
            "{label} diagnostic changed"
        );
        assert_eq!(
            output.stdout,
            Vec::<u8>::new(),
            "{label} must not write stdout"
        );
    }

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

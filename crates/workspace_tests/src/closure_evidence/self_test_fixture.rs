//! Self-test fixture builder and helpers.

use std::fs;
use std::path::Path;

use crate::closure_evidence::issues::Issue;

pub(crate) fn make_children() -> Vec<Issue> {
    vec![
        Issue {
            bead_id: "vb-jpq7.1".to_string(),
            status: "closed".to_string(),
            close_reason: "done".to_string(),
            notes: "".to_string(),
        },
        Issue {
            bead_id: "vb-jpq7.2".to_string(),
            status: "open".to_string(),
            close_reason: "".to_string(),
            notes: "".to_string(),
        },
    ]
}

pub(crate) fn make_good_row(root: &str, raw_log_path: &str) -> serde_json::Value {
    serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": root,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": raw_log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    })
}

pub(crate) fn raw_log_text(row: &serde_json::Value) -> String {
    let command = row.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let cwd = row.get("cwd").and_then(|v| v.as_str()).unwrap_or("");
    let timestamp = row.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
    let exit_code = row.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
    format!(
        "command: {}\ncwd: {}\ntimestamp: {}\nexit_code: {}\nstdout: command output observed\nstderr: command error stream observed\n",
        command, cwd, timestamp, exit_code
    )
}

pub(crate) fn write_log_file(path: &str, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("cannot write log file {}: {}", path, e))?;
    Ok(())
}

pub(crate) fn mutate_row(
    row: &serde_json::Value,
    field: &str,
    value: serde_json::Value,
) -> serde_json::Value {
    let mut obj = row.clone();
    if let Some(o) = obj.as_object_mut() {
        o.insert(field.to_string(), value);
    }
    obj
}

pub(crate) fn row_without(row: &serde_json::Value, field: &str) -> serde_json::Value {
    let mut obj = row.clone();
    if let Some(o) = obj.as_object_mut() {
        o.remove(field);
    }
    obj
}

pub(crate) fn write_manifest(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text)
        .map_err(|e| format!("cannot write manifest {}: {}", path.display(), e))?;
    Ok(())
}

//! Per-row validation logic.

use std::path::Path;

use crate::closure_evidence::checks::{
    exit_code_as_i64, is_integer_value, is_valid_sha, is_valid_timestamp, is_value_missing,
};
use crate::closure_evidence::markers::find_rejected_marker;
use crate::closure_evidence::raw_log::validate_raw_log;

const REQUIRED_ROW_FIELDS: &[&str] = &[
    "bead_id",
    "command",
    "cwd",
    "commit_sha",
    "tool_version",
    "timestamp",
    "raw_log_path",
    "stdout_summary",
    "stderr_summary",
    "exit_code",
    "status",
    "evidence_kind",
];

pub(crate) fn validate_row_shape(
    root: &Path,
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    let mut failures = Vec::new();
    failures.extend(check_required_fields(row_number, data));
    failures.extend(check_commit_sha(row_number, data));
    failures.extend(check_timestamp(row_number, data));
    failures.extend(check_exit_code(row_number, data));
    failures.extend(check_cwd(row_number, data));
    failures.extend(check_raw_log(root, row_number, data));
    failures.extend(check_rejected_markers(row_number, data));
    failures
}

fn check_required_fields(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    let mut failures = Vec::new();
    for field in REQUIRED_ROW_FIELDS {
        let val = data.get(*field);
        if val.is_none() || val.map(is_value_missing).unwrap_or(true) {
            failures.push(format!(
                "manifest row {}: missing required field {}",
                row_number, field
            ));
        }
    }
    failures
}

fn check_commit_sha(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    data.get("commit_sha")
        .and_then(|val| val.as_str())
        .filter(|s| !is_valid_sha(s))
        .map(|s| format!("manifest row {}: invalid commit_sha {}", row_number, s))
}

fn check_timestamp(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    data.get("timestamp")
        .and_then(|val| val.as_str())
        .filter(|s| !is_valid_timestamp(s))
        .map(|_| format!("manifest row {}: timestamp is not RFC3339 UTC", row_number))
}

fn check_exit_code(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    if data.get("exit_code").is_none_or(|v| !is_integer_value(v)) {
        Some(format!(
            "manifest row {}: exit_code must be integer",
            row_number
        ))
    } else {
        None
    }
}

fn check_cwd(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    data.get("cwd")
        .and_then(|val| val.as_str())
        .filter(|s| exit_code_as_i64(data) == Some(0) && !Path::new(s).is_dir())
        .map(|s| format!("manifest row {}: cwd is not a directory {}", row_number, s))
}

fn check_raw_log(
    root: &Path,
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    match data.get("raw_log_path").and_then(|val| val.as_str()) {
        Some(s) => {
            let log_path = resolve_path(root, s);
            validate_raw_log(row_number, data, &log_path)
        }
        None => Vec::new(),
    }
}

fn check_rejected_markers(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let mut combined = String::new();
    for v in data.values() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&v.to_string());
    }
    find_rejected_marker(&combined).map(|marker| {
        format!(
            "manifest row {}: rejected evidence marker present: {}",
            row_number, marker
        )
    })
}

fn resolve_path(root: &Path, raw: &str) -> std::path::PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

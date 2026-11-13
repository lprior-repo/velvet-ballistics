//! Raw log file validation.

use std::fs;
use std::path::Path;

use crate::closure_evidence::checks::{is_integer_value, string_field};

fn exit_code_text(data: &serde_json::Map<String, serde_json::Value>) -> String {
    let Some(v) = data.get("exit_code") else {
        return String::new();
    };
    if !is_integer_value(v) {
        return String::new();
    }
    if v.is_i64() {
        return v.as_i64().map_or_else(String::new, |x| x.to_string());
    }
    v.as_u64().map_or_else(String::new, |x| x.to_string())
}

fn read_log_file(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("missing file {}", path.display()));
    }
    if !path.is_file() {
        return Err(format!("is not a file {}", path.display()));
    }
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return Err(format!("unreadable: {}", e)),
    };
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn has_exact_binding(text: &str, expected: &str) -> bool {
    text.match_indices(expected).any(|(offset, matched)| {
        let before = text.get(..offset);
        let after = offset
            .checked_add(matched.len())
            .and_then(|end| text.get(end..));
        before.is_some_and(|part| part.is_empty() || part.ends_with('\n') || part.ends_with('\r'))
            && after.is_some_and(|part| {
                part.is_empty() || part.starts_with('\n') || part.starts_with('\r')
            })
    })
}

fn check_string_binding(row: usize, field: &str, val: &str, text: &str) -> Option<String> {
    let expected = format!("{}: {}", field, val);
    if has_exact_binding(text, &expected) {
        return None;
    }
    Some(format!(
        "manifest row {}: raw_log_path lacks provenance binding {}",
        row, expected
    ))
}

fn check_string_bindings(
    row: usize,
    data: &serde_json::Map<String, serde_json::Value>,
    text: &str,
) -> Vec<String> {
    [
        ("command", "command"),
        ("cwd", "cwd"),
        ("timestamp", "timestamp"),
    ]
    .iter()
    .filter_map(|(field, values)| match string_field(data, values) {
        Some(val) => check_string_binding(row, field, &val, text),
        None => Some(format!(
            "manifest row {}: raw_log_path lacks provenance binding {}: <missing>",
            row, field
        )),
    })
    .collect()
}

fn check_exit_code_binding(
    row: usize,
    data: &serde_json::Map<String, serde_json::Value>,
    text: &str,
) -> Option<String> {
    let exit_text = exit_code_text(data);
    if exit_text.is_empty() {
        return Some(format!(
            "manifest row {}: raw_log_path lacks provenance binding exit_code: <missing>",
            row
        ));
    }
    let expected = format!("exit_code: {}", exit_text);
    if has_exact_binding(text, &expected) {
        return None;
    }
    Some(format!(
        "manifest row {}: raw_log_path lacks provenance binding {}",
        row, expected
    ))
}

pub(crate) fn validate_raw_log(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
    path: &Path,
) -> Vec<String> {
    let text = match read_log_file(path) {
        Ok(t) => t,
        Err(msg) => return vec![format!("manifest row {}: raw_log_path {}", row_number, msg)],
    };
    if text.trim().is_empty() {
        return vec![format!(
            "manifest row {}: raw_log_path is empty {}",
            row_number,
            path.display()
        )];
    }
    let mut failures = check_string_bindings(row_number, data, &text);
    if let Some(failure) = check_exit_code_binding(row_number, data, &text) {
        failures.push(failure);
    }
    failures
}

//! Resolution validation for non-zero exit rows.

use std::collections::HashMap;

use crate::closure_evidence::checks::{exit_code_is_zero, string_field};

fn is_blank(s: &str) -> bool {
    s.trim_matches(crate::source_text::legacy_whitespace)
        .is_empty()
}

fn check_resolution_kind(row: usize, kind: &str) -> Option<String> {
    if kind == "split_followup" || kind == "approved_waiver" {
        return None;
    }
    Some(format!(
        "manifest row {}: nonzero row lacks valid resolution_kind",
        row
    ))
}

fn check_rationale(row: usize, rationale: &str) -> Option<String> {
    if is_blank(rationale) {
        return Some(format!(
            "manifest row {}: nonzero row lacks resolution_rationale",
            row
        ));
    }
    None
}

fn check_resolution_fields(row: usize, kind: &str, rationale: &str) -> Vec<String> {
    let mut failures = Vec::new();
    if let Some(f) = check_resolution_kind(row, kind) {
        failures.push(f);
    }
    if let Some(f) = check_rationale(row, rationale) {
        failures.push(f);
    }
    failures
}

fn check_split_followup(
    row: usize,
    data: &serde_json::Map<String, serde_json::Value>,
    rationale: &str,
    seen: &mut HashMap<(String, String), usize>,
) -> Vec<String> {
    let split = string_field(data, "split_bead_id");
    let split_val = split.as_deref().map_or("", |v| v);
    if is_blank(split_val) {
        return vec![format!(
            "manifest row {}: split_followup lacks split_bead_id",
            row
        )];
    }
    let key = (split_val.to_string(), rationale.to_string());
    let mut failures = Vec::new();
    if let Some(prev_row) = seen.get(&key) {
        failures.push(format!(
            "manifest row {}: duplicate split_followup rationale also used on row {}",
            row, prev_row
        ));
    }
    seen.insert(key, row);
    failures
}

fn check_approved_waiver(
    row: usize,
    data: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let waiver_id = string_field(data, "waiver_id");
    let waiver_val = waiver_id.as_deref().map_or("", |v| v);
    if is_blank(waiver_val) {
        return Some(format!(
            "manifest row {}: approved_waiver lacks waiver_id",
            row
        ));
    }
    None
}

pub(crate) fn validate_resolution(
    row_number: usize,
    data: &serde_json::Map<String, serde_json::Value>,
    seen: &mut HashMap<(String, String), usize>,
) -> Vec<String> {
    if exit_code_is_zero(data) {
        return vec![];
    }
    let resolution_field = string_field(data, "resolution_kind");
    let resolution = resolution_field.as_deref().map_or("", |v| v);
    let rationale_field = string_field(data, "resolution_rationale");
    let rationale = rationale_field.as_deref().map_or("", |v| v);
    let mut failures = check_resolution_fields(row_number, resolution, rationale);
    if resolution == "split_followup" {
        failures.extend(check_split_followup(row_number, data, rationale, seen));
    }
    if resolution == "approved_waiver"
        && let Some(f) = check_approved_waiver(row_number, data)
    {
        failures.push(f);
    }
    failures
}

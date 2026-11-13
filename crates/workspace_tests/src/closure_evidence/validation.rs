//! Main validation logic for closure evidence.

use std::collections::HashMap;
use std::path::Path;

use crate::closure_evidence::checks::{exit_code_as_i64, string_field};
use crate::closure_evidence::issues::Issue;
use crate::closure_evidence::markers::find_rejected_marker;
use crate::closure_evidence::resolution::validate_resolution;
use crate::closure_evidence::row_validation::validate_row_shape;

const CLOSURE_STATUSES: &[&str] = &["PASS", "CLOSURE_PASS", "EXECUTED_PASS"];
const EVIDENCE_KINDS: &[&str] = &["raw-command", "command", "raw-log", "live-command"];

#[derive(Debug, Clone)]
pub(crate) struct ValidationOutcome {
    pub failures: Vec<String>,
    pub closed_count: usize,
}

pub(crate) fn validate_closed_children(
    root: &Path,
    issues: &[Issue],
    manifest_rows: &[(usize, serde_json::Value)],
) -> ValidationOutcome {
    let closed_issues = filter_closed_issues(issues);
    let by_bead = build_bead_index(manifest_rows);
    let mut failures = Vec::new();
    let mut seen_followups: HashMap<(String, String), usize> = HashMap::new();
    failures.extend(validate_all_rows(root, manifest_rows, &mut seen_followups));
    failures.extend(check_all_closed_issues(&closed_issues, &by_bead));
    ValidationOutcome {
        failures,
        closed_count: closed_issues.len(),
    }
}

fn check_all_closed_issues(
    closed_issues: &[&Issue],
    by_bead: &HashMap<String, Vec<&serde_json::Value>>,
) -> Vec<String> {
    closed_issues
        .iter()
        .flat_map(|issue| check_single_closed_issue(issue, by_bead))
        .collect()
}

fn check_single_closed_issue(
    issue: &Issue,
    by_bead: &HashMap<String, Vec<&serde_json::Value>>,
) -> Vec<String> {
    let issue_rows = by_bead
        .get(&issue.bead_id)
        .map_or_else(Vec::new, Clone::clone);
    if issue_rows.is_empty() {
        return vec![format!(
            "{}: closed child lacks manifest row",
            issue.bead_id
        )];
    }
    let mut failures = check_passing_row(issue, &issue_rows);
    if let Some(f) = check_closure_prose(issue) {
        failures.push(f);
    }
    failures
}

fn check_passing_row(issue: &Issue, issue_rows: &[&serde_json::Value]) -> Vec<String> {
    if has_passing_row(issue_rows) {
        return vec![];
    }
    vec![format!(
        "{}: no passing closure row or waiver/split-linked failure row",
        issue.bead_id
    )]
}

fn filter_closed_issues(issues: &[Issue]) -> Vec<&Issue> {
    issues.iter().filter(|i| i.status == "closed").collect()
}

fn build_bead_index(
    manifest_rows: &[(usize, serde_json::Value)],
) -> HashMap<String, Vec<&serde_json::Value>> {
    manifest_rows
        .iter()
        .fold(HashMap::new(), |mut by_bead, (_, data)| {
            if let Some(obj) = data.as_object()
                && let Some(bid) = string_field(obj, "bead_id")
            {
                by_bead.entry(bid).or_default().push(data);
            }
            by_bead
        })
}

fn validate_all_rows(
    root: &Path,
    manifest_rows: &[(usize, serde_json::Value)],
    seen_followups: &mut HashMap<(String, String), usize>,
) -> Vec<String> {
    manifest_rows
        .iter()
        .fold(Vec::new(), |mut failures, (row_number, data)| {
            if let Some(obj) = data.as_object() {
                failures.extend(validate_row_shape(root, *row_number, obj));
                failures.extend(validate_resolution(*row_number, obj, seen_followups));
            }
            failures
        })
}

fn has_passing_row(rows: &[&serde_json::Value]) -> bool {
    rows.iter()
        .any(|data| data.as_object().is_some_and(row_closes_or_links_followup))
}

fn check_closure_prose(issue: &Issue) -> Option<String> {
    let closure_text = format!("{}\n{}", issue.close_reason, issue.notes).to_lowercase();
    find_rejected_marker(&closure_text).map(|marker| {
        format!(
            "{}: closure prose contains rejected evidence marker {}",
            issue.bead_id, marker
        )
    })
}

fn is_blank(s: &str) -> bool {
    s.trim_matches(crate::source_text::legacy_whitespace)
        .is_empty()
}

fn row_closes_or_links_followup(data: &serde_json::Map<String, serde_json::Value>) -> bool {
    if row_is_closure_pass(data) {
        return true;
    }
    if exit_code_as_i64(data) == Some(0) {
        return false;
    }
    let rationale = string_field(data, "resolution_rationale");
    let rationale_val = rationale.as_deref().map_or("", |v| v);
    if is_blank(rationale_val) {
        return false;
    }
    let resolution = string_field(data, "resolution_kind");
    let resolution_val = resolution.as_deref().map_or("", |v| v);
    if resolution_val == "approved_waiver" {
        return check_waiver_resolution(data);
    }
    if resolution_val == "split_followup" {
        return check_split_resolution(data);
    }
    false
}

fn check_waiver_resolution(data: &serde_json::Map<String, serde_json::Value>) -> bool {
    let waiver_id = string_field(data, "waiver_id");
    let waiver_val = waiver_id.as_deref().map_or("", |v| v);
    !is_blank(waiver_val)
}

fn check_split_resolution(data: &serde_json::Map<String, serde_json::Value>) -> bool {
    let split_id = string_field(data, "split_bead_id");
    let split_val = split_id.as_deref().map_or("", |v| v);
    !is_blank(split_val)
}

fn row_is_closure_pass(data: &serde_json::Map<String, serde_json::Value>) -> bool {
    if exit_code_as_i64(data) != Some(0) {
        return false;
    }
    let status = string_field(data, "status").map_or(String::new(), |v| v);
    let evidence_kind = string_field(data, "evidence_kind").map_or(String::new(), |v| v);
    let upper = status.to_uppercase();
    CLOSURE_STATUSES.contains(&upper.as_str()) && EVIDENCE_KINDS.contains(&evidence_kind.as_str())
}

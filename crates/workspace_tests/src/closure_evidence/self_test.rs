//! Self-test logic for closure evidence checker.

use std::fs;
use std::path::Path;

use crate::closure_evidence::issues::Issue;
use crate::closure_evidence::validation::validate_closed_children;

pub(crate) fn run_self_test() -> Result<i32, String> {
    let root = std::env::current_dir()
        .map(|d| d.to_string_lossy().to_string())
        .map_err(|e| format!("cannot determine cwd: {}", e))?;
    let root = root.trim_end_matches('/');

    let self_test_dir = ".evidence/vb-jpq7.48/scratch/self-test";
    fs::create_dir_all(self_test_dir)
        .map_err(|e| format!("cannot create self-test directory: {}", e))?;

    let children = vec![
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
    ];

    let mut failures = Vec::new();

    // Test 1: valid row passes
    failures.extend(test_valid_row(root, &children));

    // Test 2: bad marker
    failures.extend(test_bad_marker(root, &children));

    // Test 3: missing required field
    failures.extend(test_missing_field(root, &children));

    // Test 4: invalid sha
    failures.extend(test_invalid_sha(root, &children));

    // Test 5: non-integer exit_code
    failures.extend(test_non_integer_exit(root, &children));

    // Test 6: invalid timestamp
    failures.extend(test_invalid_timestamp(root, &children));

    // Test 7: split-followup
    failures.extend(test_split_followup(root, &children));

    // Test 8: approved waiver
    failures.extend(test_approved_waiver(root, &children));

    // Test 9: missing child row
    failures.extend(test_missing_child_row(&children));

    if failures.is_empty() {
        Ok(0)
    } else {
        for f in &failures {
            eprintln!("- {}", f);
        }
        Ok(1)
    }
}

fn write_log(path: &str, content: &str) {
    let _ = fs::write(path, content);
}

fn good_row(root: &str, log_path: &str) -> serde_json::Value {
    serde_json::json!({
        "bead_id": "vb-jpq7.1",
        "command": "cargo test",
        "cwd": root,
        "commit_sha": "abcdef1",
        "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z",
        "raw_log_path": log_path,
        "stdout_summary": "1 passed",
        "stderr_summary": "empty",
        "exit_code": 0,
        "status": "PASS",
        "evidence_kind": "raw-command"
    })
}

fn manifest_from_rows(rows: &[serde_json::Value]) -> String {
    rows.iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_valid_row(root: &str, children: &[Issue]) -> Vec<String> {
    let raw_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    write_log(
        &raw_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n",
            root
        ),
    );

    let row = good_row(root, &raw_log_path);
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if !outcome.failures.is_empty() {
        return vec![format!(
            "valid row: expected pass, got {:?}",
            outcome.failures
        )];
    }
    vec![]
}

fn test_bad_marker(root: &str, children: &[Issue]) -> Vec<String> {
    let raw_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    write_log(
        &raw_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n",
            root
        ),
    );

    let mut row = good_row(root, &raw_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert(
            "stdout_summary".to_string(),
            serde_json::json!("subagent-only result"),
        );
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if outcome.failures.is_empty() {
        return vec!["subagent-only: expected failure".to_string()];
    }
    vec![]
}

fn test_missing_field(root: &str, children: &[Issue]) -> Vec<String> {
    let raw_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    write_log(
        &raw_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n",
            root
        ),
    );

    let mut row = good_row(root, &raw_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.remove("command");
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if outcome.failures.is_empty() {
        return vec!["missing command: expected failure".to_string()];
    }
    vec![]
}

fn test_invalid_sha(root: &str, children: &[Issue]) -> Vec<String> {
    let raw_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    write_log(
        &raw_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n",
            root
        ),
    );

    let mut row = good_row(root, &raw_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("commit_sha".to_string(), serde_json::json!("not-a-sha"));
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if outcome.failures.is_empty() {
        return vec!["invalid sha: expected failure".to_string()];
    }
    vec![]
}

fn test_non_integer_exit(root: &str, children: &[Issue]) -> Vec<String> {
    let raw_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    write_log(
        &raw_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n",
            root
        ),
    );

    let mut row = good_row(root, &raw_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("exit_code".to_string(), serde_json::json!("0"));
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if outcome.failures.is_empty() {
        return vec!["non-integer exit_code: expected failure".to_string()];
    }
    vec![]
}

fn test_invalid_timestamp(root: &str, children: &[Issue]) -> Vec<String> {
    let raw_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    write_log(
        &raw_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 0\nstdout: pass\n",
            root
        ),
    );

    let mut row = good_row(root, &raw_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("timestamp".to_string(), serde_json::json!("yesterday"));
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if outcome.failures.is_empty() {
        return vec!["invalid timestamp: expected failure".to_string()];
    }
    vec![]
}

fn test_split_followup(root: &str, children: &[Issue]) -> Vec<String> {
    let split_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/split.log", root);
    write_log(
        &split_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\n",
            root
        ),
    );

    let mut row = good_row(root, &split_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("exit_code".to_string(), serde_json::json!(1));
        obj.insert("status".to_string(), serde_json::json!("FAIL"));
        obj.insert(
            "resolution_kind".to_string(),
            serde_json::json!("split_followup"),
        );
        obj.insert(
            "resolution_rationale".to_string(),
            serde_json::json!("self-test split followup for vb-jpq7.1"),
        );
        obj.insert("split_bead_id".to_string(), serde_json::json!("vb-rud5"));
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if !outcome.failures.is_empty() {
        return vec![format!(
            "split-followup: expected pass, got {:?}",
            outcome.failures
        )];
    }
    vec![]
}

fn test_approved_waiver(root: &str, children: &[Issue]) -> Vec<String> {
    let waiver_log_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/waiver.log", root);
    write_log(
        &waiver_log_path,
        &format!(
            "command: cargo test\ncwd: {}\ntimestamp: 2026-01-01T00:00:00Z\nexit_code: 1\n",
            root
        ),
    );

    let mut row = good_row(root, &waiver_log_path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("exit_code".to_string(), serde_json::json!(1));
        obj.insert("status".to_string(), serde_json::json!("FAIL"));
        obj.insert(
            "resolution_kind".to_string(),
            serde_json::json!("approved_waiver"),
        );
        obj.insert(
            "resolution_rationale".to_string(),
            serde_json::json!("approved by owner"),
        );
        obj.insert("waiver_id".to_string(), serde_json::json!("W-123"));
    }
    let manifest = manifest_from_rows(&[row]);
    let (rows, _) = crate::closure_evidence::manifest::parse_manifest(&manifest);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    if !outcome.failures.is_empty() {
        return vec![format!(
            "approved-waiver: expected pass, got {:?}",
            outcome.failures
        )];
    }
    vec![]
}

fn test_missing_child_row(children: &[Issue]) -> Vec<String> {
    let empty_rows: Vec<(usize, serde_json::Value)> = Vec::new();
    let outcome = validate_closed_children(Path::new(""), children, &empty_rows);
    if outcome.failures.is_empty() {
        return vec!["missing child row: expected failure".to_string()];
    }
    let has_missing = outcome
        .failures
        .iter()
        .any(|f| f.contains("closed child lacks manifest row"));
    if !has_missing {
        return vec!["missing child row: wrong failure message".to_string()];
    }
    vec![]
}

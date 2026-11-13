use crate::closure_evidence::issues::Issue;
use crate::closure_evidence::validation::validate_closed_children;
use std::path::Path;

fn make_row(row_number: usize, fields: &[(&str, serde_json::Value)]) -> (usize, serde_json::Value) {
    let mut obj = serde_json::Map::new();
    for (key, value) in fields {
        obj.insert(key.to_string(), value.clone());
    }
    (row_number, serde_json::Value::Object(obj))
}

fn make_valid_row(row_number: usize, bead_id: &str) -> (usize, serde_json::Value) {
    make_row(
        row_number,
        &[
            ("bead_id", serde_json::Value::String(bead_id.to_string())),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(0)),
            ),
            ("status", serde_json::Value::String("PASS".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
        ],
    )
}

#[test]
fn nested_object_rejected_marker_detected() {
    let mut nested = serde_json::Map::new();
    nested.insert(
        "inner".to_string(),
        serde_json::Value::String("summary-only".to_string()),
    );
    let fields = vec![
        (
            "bead_id",
            serde_json::Value::String("vb-test-1".to_string()),
        ),
        (
            "command",
            serde_json::Value::String("cargo test".to_string()),
        ),
        ("cwd", serde_json::Value::String("./".to_string())),
        (
            "commit_sha",
            serde_json::Value::String("abc123def456".to_string()),
        ),
        (
            "tool_version",
            serde_json::Value::String("cargo 1.80".to_string()),
        ),
        (
            "timestamp",
            serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
        ),
        (
            "raw_log_path",
            serde_json::Value::String("/dev/null".to_string()),
        ),
        (
            "stdout_summary",
            serde_json::Value::String("ok".to_string()),
        ),
        ("stderr_summary", serde_json::Value::String("".to_string())),
        (
            "exit_code",
            serde_json::Value::Number(serde_json::Number::from(0)),
        ),
        ("status", serde_json::Value::String("PASS".to_string())),
        (
            "evidence_kind",
            serde_json::Value::String("raw-command".to_string()),
        ),
        ("metadata", serde_json::Value::Object(nested)),
    ];
    let row = make_row(1, &fields);
    let issues = vec![Issue {
        bead_id: "vb-test-1".to_string(),
        status: "closed".to_string(),
        close_reason: "fixed".to_string(),
        notes: "verified".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(outcome.failures.iter().any(|f| f.contains("summary-only")));
}

#[test]
fn nested_array_rejected_marker_detected() {
    let fields = vec![
        (
            "bead_id",
            serde_json::Value::String("vb-test-2".to_string()),
        ),
        (
            "command",
            serde_json::Value::String("cargo test".to_string()),
        ),
        ("cwd", serde_json::Value::String("./".to_string())),
        (
            "commit_sha",
            serde_json::Value::String("abc123def456".to_string()),
        ),
        (
            "tool_version",
            serde_json::Value::String("cargo 1.80".to_string()),
        ),
        (
            "timestamp",
            serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
        ),
        (
            "raw_log_path",
            serde_json::Value::String("/dev/null".to_string()),
        ),
        (
            "stdout_summary",
            serde_json::Value::String("ok".to_string()),
        ),
        ("stderr_summary", serde_json::Value::String("".to_string())),
        (
            "exit_code",
            serde_json::Value::Number(serde_json::Number::from(0)),
        ),
        ("status", serde_json::Value::String("PASS".to_string())),
        (
            "evidence_kind",
            serde_json::Value::String("raw-command".to_string()),
        ),
        (
            "tags",
            serde_json::Value::Array(vec![serde_json::Value::String(
                "delegated-only".to_string(),
            )]),
        ),
    ];
    let row = make_row(1, &fields);
    let issues = vec![Issue {
        bead_id: "vb-test-2".to_string(),
        status: "closed".to_string(),
        close_reason: "fixed".to_string(),
        notes: "verified".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(
        outcome
            .failures
            .iter()
            .any(|f| f.contains("delegated-only"))
    );
}

#[test]
fn nested_ordinary_values_not_flagged() {
    let mut nested = serde_json::Map::new();
    nested.insert(
        "status".to_string(),
        serde_json::Value::String("complete".to_string()),
    );
    nested.insert(
        "count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(42)),
    );
    let fields = vec![
        (
            "bead_id",
            serde_json::Value::String("vb-test-3".to_string()),
        ),
        (
            "command",
            serde_json::Value::String("cargo test".to_string()),
        ),
        ("cwd", serde_json::Value::String("./".to_string())),
        (
            "commit_sha",
            serde_json::Value::String("abc123def456".to_string()),
        ),
        (
            "tool_version",
            serde_json::Value::String("cargo 1.80".to_string()),
        ),
        (
            "timestamp",
            serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
        ),
        (
            "raw_log_path",
            serde_json::Value::String("/dev/null".to_string()),
        ),
        (
            "stdout_summary",
            serde_json::Value::String("ok".to_string()),
        ),
        ("stderr_summary", serde_json::Value::String("".to_string())),
        (
            "exit_code",
            serde_json::Value::Number(serde_json::Number::from(0)),
        ),
        ("status", serde_json::Value::String("PASS".to_string())),
        (
            "evidence_kind",
            serde_json::Value::String("raw-command".to_string()),
        ),
        ("metadata", serde_json::Value::Object(nested)),
    ];
    let row = make_row(1, &fields);
    let issues = vec![Issue {
        bead_id: "vb-test-3".to_string(),
        status: "closed".to_string(),
        close_reason: "fixed".to_string(),
        notes: "verified".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(!outcome.failures.iter().any(|f| f.contains("rejected")));
}

#[test]
fn per_row_shape_then_resolution_ordering() {
    let row1 = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-4a".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("not-a-sha".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(1)),
            ),
            ("status", serde_json::Value::String("FAIL".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
        ],
    );
    let row2 = make_row(
        2,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-4b".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("not-a-sha-either".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(0)),
            ),
            ("status", serde_json::Value::String("PASS".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
        ],
    );
    let issues = vec![];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row1, row2]);
    let row1_sha = outcome
        .failures
        .iter()
        .position(|f| f == "manifest row 1: invalid commit_sha not-a-sha");
    let row1_res = outcome
        .failures
        .iter()
        .position(|f| f == "manifest row 1: nonzero row lacks valid resolution_kind");
    let row2_sha = outcome
        .failures
        .iter()
        .position(|f| f == "manifest row 2: invalid commit_sha not-a-sha-either");
    assert!(row1_sha.is_some());
    assert!(row1_res.is_some());
    assert!(row2_sha.is_some());
    let row1_sha = row1_sha.unwrap();
    let row1_res = row1_res.unwrap();
    let row2_sha = row2_sha.unwrap();
    assert!(row1_sha < row1_res);
    assert!(row1_res < row2_sha);
}

#[test]
fn closure_prose_newline_join_no_false_positive() {
    let row = make_valid_row(1, "vb-test-5");
    let issues = vec![Issue {
        bead_id: "vb-test-5".to_string(),
        status: "closed".to_string(),
        close_reason: "summary".to_string(),
        notes: "only".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(!outcome.failures.iter().any(|f| f.contains("closure prose")));
}

#[test]
fn closure_prose_rejected_marker_detected() {
    let row = make_valid_row(1, "vb-test-6");
    let issues = vec![Issue {
        bead_id: "vb-test-6".to_string(),
        status: "closed".to_string(),
        close_reason: "done".to_string(),
        notes: "cached-only for now".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(outcome.failures.iter().any(|f| f.contains("cached-only")));
}

#[test]
fn missing_exit_code_gets_both_diagnostics() {
    let row = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-7".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            ("status", serde_json::Value::String("PASS".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
        ],
    );
    let issues = vec![];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    let has_missing = outcome
        .failures
        .iter()
        .any(|f| f == "manifest row 1: missing required field exit_code");
    let has_integer = outcome
        .failures
        .iter()
        .any(|f| f == "manifest row 1: exit_code must be integer");
    assert!(has_missing);
    assert!(has_integer);
}

#[test]
fn whitespace_only_rationale_not_passing() {
    let row = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-8".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(1)),
            ),
            ("status", serde_json::Value::String("FAIL".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
            (
                "resolution_kind",
                serde_json::Value::String("split_followup".to_string()),
            ),
            (
                "resolution_rationale",
                serde_json::Value::String("   ".to_string()),
            ),
            (
                "split_bead_id",
                serde_json::Value::String("vb-test-8b".to_string()),
            ),
        ],
    );
    let issues = vec![Issue {
        bead_id: "vb-test-8".to_string(),
        status: "closed".to_string(),
        close_reason: "split".to_string(),
        notes: "followup created".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(
        outcome
            .failures
            .iter()
            .any(|f| f.contains("no passing closure row"))
    );
}

#[test]
fn whitespace_only_split_id_not_passing() {
    let row = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-9".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(1)),
            ),
            ("status", serde_json::Value::String("FAIL".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
            (
                "resolution_kind",
                serde_json::Value::String("split_followup".to_string()),
            ),
            (
                "resolution_rationale",
                serde_json::Value::String("legitimate reason".to_string()),
            ),
            (
                "split_bead_id",
                serde_json::Value::String("   ".to_string()),
            ),
        ],
    );
    let issues = vec![Issue {
        bead_id: "vb-test-9".to_string(),
        status: "closed".to_string(),
        close_reason: "split".to_string(),
        notes: "followup created".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(
        outcome
            .failures
            .iter()
            .any(|f| f.contains("no passing closure row"))
    );
}

#[test]
fn valid_split_resolution_passes() {
    let row = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-10".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(1)),
            ),
            ("status", serde_json::Value::String("FAIL".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
            (
                "resolution_kind",
                serde_json::Value::String("split_followup".to_string()),
            ),
            (
                "resolution_rationale",
                serde_json::Value::String("legitimate reason".to_string()),
            ),
            (
                "split_bead_id",
                serde_json::Value::String("vb-test-10b".to_string()),
            ),
        ],
    );
    let issues = vec![Issue {
        bead_id: "vb-test-10".to_string(),
        status: "closed".to_string(),
        close_reason: "split".to_string(),
        notes: "followup created".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(
        !outcome
            .failures
            .iter()
            .any(|f| f.contains("no passing closure row"))
    );
}

#[test]
fn closed_issue_no_rows_skips_prose() {
    let issues = vec![Issue {
        bead_id: "vb-test-11".to_string(),
        status: "closed".to_string(),
        close_reason: "done".to_string(),
        notes: "summary-only".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[]);
    assert_eq!(outcome.failures.len(), 1);
    assert_eq!(
        outcome.failures[0],
        "vb-test-11: closed child lacks manifest row"
    );
}

#[test]
fn whitespace_only_waiver_rationale_not_passing() {
    let row = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-12".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(1)),
            ),
            ("status", serde_json::Value::String("FAIL".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
            (
                "resolution_kind",
                serde_json::Value::String("approved_waiver".to_string()),
            ),
            (
                "resolution_rationale",
                serde_json::Value::String("   ".to_string()),
            ),
            ("waiver_id", serde_json::Value::String("W-100".to_string())),
        ],
    );
    let issues = vec![Issue {
        bead_id: "vb-test-12".to_string(),
        status: "closed".to_string(),
        close_reason: "waived".to_string(),
        notes: "approved".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(
        outcome
            .failures
            .iter()
            .any(|f| f.contains("no passing closure row"))
    );
}

#[test]
fn whitespace_only_waiver_id_not_passing() {
    let row = make_row(
        1,
        &[
            (
                "bead_id",
                serde_json::Value::String("vb-test-13".to_string()),
            ),
            (
                "command",
                serde_json::Value::String("cargo test".to_string()),
            ),
            ("cwd", serde_json::Value::String("./".to_string())),
            (
                "commit_sha",
                serde_json::Value::String("abc123def456".to_string()),
            ),
            (
                "tool_version",
                serde_json::Value::String("cargo 1.80".to_string()),
            ),
            (
                "timestamp",
                serde_json::Value::String("2026-01-01T00:00:00Z".to_string()),
            ),
            (
                "raw_log_path",
                serde_json::Value::String("/dev/null".to_string()),
            ),
            (
                "stdout_summary",
                serde_json::Value::String("ok".to_string()),
            ),
            ("stderr_summary", serde_json::Value::String("".to_string())),
            (
                "exit_code",
                serde_json::Value::Number(serde_json::Number::from(1)),
            ),
            ("status", serde_json::Value::String("FAIL".to_string())),
            (
                "evidence_kind",
                serde_json::Value::String("raw-command".to_string()),
            ),
            (
                "resolution_kind",
                serde_json::Value::String("approved_waiver".to_string()),
            ),
            (
                "resolution_rationale",
                serde_json::Value::String("legitimate waiver".to_string()),
            ),
            ("waiver_id", serde_json::Value::String("   ".to_string())),
        ],
    );
    let issues = vec![Issue {
        bead_id: "vb-test-13".to_string(),
        status: "closed".to_string(),
        close_reason: "waived".to_string(),
        notes: "approved".to_string(),
    }];
    let outcome = validate_closed_children(Path::new("."), &issues, &[row]);
    assert!(
        outcome
            .failures
            .iter()
            .any(|f| f.contains("no passing closure row"))
    );
}

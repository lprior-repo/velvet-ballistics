//! Shape validation test cases for self-test.

use crate::closure_evidence::self_test::self_test_cases::TestCase;
use crate::closure_evidence::self_test::self_test_fixture as fx;

enum ShapeMutation {
    Remove(&'static str),
    SetStr(&'static str, &'static str),
}

struct ShapeCase {
    label: &'static str,
    mutation: ShapeMutation,
    expect: &'static str,
}

const SHAPE_CASES: &[ShapeCase] = &[
    ShapeCase {
        label: "missing required field command",
        mutation: ShapeMutation::Remove("command"),
        expect: "missing required field command",
    },
    ShapeCase {
        label: "bad sha",
        mutation: ShapeMutation::SetStr("commit_sha", "not-a-sha"),
        expect: "invalid commit_sha",
    },
    ShapeCase {
        label: "bad exit non-integer",
        mutation: ShapeMutation::SetStr("exit_code", "0"),
        expect: "exit_code must be integer",
    },
    ShapeCase {
        label: "bad time",
        mutation: ShapeMutation::SetStr("timestamp", "yesterday"),
        expect: "timestamp is not",
    },
    ShapeCase {
        label: "non-utc timestamp",
        mutation: ShapeMutation::SetStr("timestamp", "2026-01-01T01:00:00+01:00"),
        expect: "timestamp is not",
    },
    ShapeCase {
        label: "bad cwd",
        mutation: ShapeMutation::SetStr("cwd", "missing-dir"),
        expect: "cwd is not",
    },
    ShapeCase {
        label: "missing status",
        mutation: ShapeMutation::Remove("status"),
        expect: "missing required field status",
    },
    ShapeCase {
        label: "missing evidence kind",
        mutation: ShapeMutation::Remove("evidence_kind"),
        expect: "missing required field evidence_kind",
    },
    ShapeCase {
        label: "missing raw log path",
        mutation: ShapeMutation::SetStr("raw_log_path", "missing.log"),
        expect: "raw_log_path missing",
    },
    ShapeCase {
        label: "no matching child",
        mutation: ShapeMutation::SetStr("bead_id", "vb-jpq7.9"),
        expect: "closed child lacks",
    },
];

fn apply_mutation(good: &serde_json::Value, mutation: &ShapeMutation) -> serde_json::Value {
    match mutation {
        ShapeMutation::Remove(field) => remove_field(good, field),
        ShapeMutation::SetStr(field, value) => mutate(good, field, value),
    }
}

fn build_shape_cases(good: &serde_json::Value) -> Vec<TestCase> {
    SHAPE_CASES
        .iter()
        .map(|case| {
            let row = apply_mutation(good, &case.mutation);
            test_case(case.label, vec![row], 1, Some(case.expect))
        })
        .collect()
}

pub(crate) fn shape_cases(root: &str, good: &serde_json::Value) -> Result<Vec<TestCase>, String> {
    let mut cases = build_shape_cases(good);
    cases.push(empty_raw_case(root)?);
    Ok(cases)
}

fn test_case(
    label: &'static str,
    rows: Vec<serde_json::Value>,
    code: i32,
    contains: Option<&'static str>,
) -> TestCase {
    TestCase {
        label,
        rows,
        expect_code: code,
        expect_failure_contains: contains,
    }
}

fn empty_raw_case(root: &str) -> Result<TestCase, String> {
    let path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/empty.log", root);
    fx::write_log_file(&path, "")?;
    Ok(test_case(
        "empty raw log",
        vec![fx::make_good_row(root, &path)],
        1,
        Some("raw_log_path is empty"),
    ))
}

fn mutate(row: &serde_json::Value, field: &str, value: &str) -> serde_json::Value {
    fx::mutate_row(row, field, serde_json::json!(value))
}

fn remove_field(row: &serde_json::Value, field: &str) -> serde_json::Value {
    fx::row_without(row, field)
}

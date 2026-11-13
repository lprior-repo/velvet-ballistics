//! Self-test case definitions.

use crate::closure_evidence::self_test::self_test_fixture as fx;

#[path = "self_test_shape_cases.rs"]
mod self_test_shape_cases;

pub(crate) struct TestCase {
    pub(crate) label: &'static str,
    pub(crate) rows: Vec<serde_json::Value>,
    pub(crate) expect_code: i32,
    pub(crate) expect_failure_contains: Option<&'static str>,
}

pub(crate) fn build_cases(root: &str) -> Result<Vec<TestCase>, String> {
    let raw_path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/raw.log", root);
    let good = fx::make_good_row(root, &raw_path);
    fx::write_log_file(&raw_path, &fx::raw_log_text(&good))?;
    let mut cases = Vec::new();
    cases.push(valid_row_case(&good));
    cases.extend(marker_cases(&good));
    cases.extend(self_test_shape_cases::shape_cases(root, &good)?);
    cases.push(split_linked_case(root)?);
    cases.extend(resolution_cases(root, &good)?);
    cases.push(approved_waiver_case(root)?);
    Ok(cases)
}

fn valid_row_case(good: &serde_json::Value) -> TestCase {
    test_case("valid raw row", vec![good.clone()], 0, None)
}

const MARKER_CASES: &[(&str, &str)] = &[
    ("reject subagent-only", "subagent-only"),
    ("reject summary-only", "summary-only"),
    ("reject cached-only", "cached-only"),
    ("reject skipped-only", "skipped-only"),
];

fn marker_cases(good: &serde_json::Value) -> Vec<TestCase> {
    MARKER_CASES
        .iter()
        .map(|(label, marker)| {
            test_case(
                label,
                vec![mutate(
                    good,
                    "stdout_summary",
                    &format!("{} result", marker),
                )],
                1,
                Some(marker),
            )
        })
        .collect()
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

fn split_linked_case(root: &str) -> Result<TestCase, String> {
    let path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/split.log", root);
    let mut row = fx::make_good_row(root, &path);
    set_split(
        &mut row,
        "vb-jpq7.1",
        "self-test split followup for vb-jpq7.1",
    );
    fx::write_log_file(&path, &fx::raw_log_text(&row))?;
    Ok(test_case("split-linked nonzero", vec![row], 0, None))
}

fn resolution_cases(root: &str, good: &serde_json::Value) -> Result<Vec<TestCase>, String> {
    Ok(vec![
        test_case(
            "missing resolution kind",
            vec![fail_row(good)],
            1,
            Some("resolution_kind"),
        ),
        duplicate_split_case(root)?,
        missing_rationale_case(root)?,
    ])
}

fn duplicate_split_case(root: &str) -> Result<TestCase, String> {
    let first_log = format!(
        "{}/.evidence/vb-jpq7.48/scratch/self-test/first-split.log",
        root
    );
    let second_log = format!(
        "{}/.evidence/vb-jpq7.48/scratch/self-test/second-split.log",
        root
    );
    let mut first = fx::make_good_row(root, &first_log);
    let mut second = fx::make_good_row(root, &second_log);
    set_split(&mut first, "vb-jpq7.1", "same reason");
    set_split(&mut second, "vb-jpq7.2", "same reason");
    fx::write_log_file(&first_log, &fx::raw_log_text(&first))?;
    fx::write_log_file(&second_log, &fx::raw_log_text(&second))?;
    Ok(test_case(
        "duplicate split followup",
        vec![first, second],
        1,
        Some("duplicate split_followup"),
    ))
}

fn missing_rationale_case(root: &str) -> Result<TestCase, String> {
    let path = format!(
        "{}/.evidence/vb-jpq7.48/scratch/self-test/no-rationale.log",
        root
    );
    let mut row = fx::make_good_row(root, &path);
    set_split(&mut row, "vb-jpq7.1", "");
    fx::write_log_file(&path, &fx::raw_log_text(&row))?;
    Ok(test_case(
        "missing rationale",
        vec![row],
        1,
        Some("resolution_rationale"),
    ))
}

fn approved_waiver_case(root: &str) -> Result<TestCase, String> {
    let path = format!("{}/.evidence/vb-jpq7.48/scratch/self-test/waiver.log", root);
    let mut row = fx::make_good_row(root, &path);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("exit_code".to_string(), serde_json::json!(1));
        obj.insert("status".to_string(), serde_json::json!("FAIL"));
        obj.insert(
            "resolution_kind".to_string(),
            serde_json::json!("approved_waiver"),
        );
        obj.insert("waiver_id".to_string(), serde_json::json!("W-123"));
        obj.insert(
            "resolution_rationale".to_string(),
            serde_json::json!("approved by owner"),
        );
    }
    fx::write_log_file(&path, &fx::raw_log_text(&row))?;
    Ok(test_case("approved waiver", vec![row], 0, None))
}

fn set_split(row: &mut serde_json::Value, bead_id: &str, rationale: &str) {
    if let Some(obj) = row.as_object_mut() {
        obj.insert("bead_id".to_string(), serde_json::json!(bead_id));
        obj.insert("exit_code".to_string(), serde_json::json!(1));
        obj.insert("status".to_string(), serde_json::json!("FAIL"));
        obj.insert(
            "resolution_kind".to_string(),
            serde_json::json!("split_followup"),
        );
        obj.insert("split_bead_id".to_string(), serde_json::json!("vb-rud5"));
        obj.insert(
            "resolution_rationale".to_string(),
            serde_json::json!(rationale),
        );
    }
}

fn fail_row(good: &serde_json::Value) -> serde_json::Value {
    let mut row = fx::mutate_row(good, "exit_code", serde_json::json!(1));
    if let Some(obj) = row.as_object_mut() {
        obj.insert("status".to_string(), serde_json::json!("FAIL"));
    }
    row
}

fn mutate(row: &serde_json::Value, field: &str, value: &str) -> serde_json::Value {
    fx::mutate_row(row, field, serde_json::json!(value))
}

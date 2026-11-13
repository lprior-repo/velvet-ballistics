//! Self-test logic for closure evidence checker.

use std::fs;
use std::io::Write;
use std::path::Path;

#[path = "self_test_fixture.rs"]
mod self_test_fixture;

#[path = "self_test_cases.rs"]
mod self_test_cases;

use crate::closure_evidence::issues::Issue;
use crate::closure_evidence::manifest::parse_manifest;
use crate::closure_evidence::validation::validate_closed_children;

pub(crate) fn run_self_test() -> Result<i32, String> {
    let root = std::env::current_dir()
        .map(|d| d.to_string_lossy().to_string())
        .map_err(|e| format!("cannot determine cwd: {}", e))?;
    let root = root.trim_end_matches('/');
    fs::create_dir_all(".evidence/vb-jpq7.48/scratch/self-test")
        .map_err(|e| format!("cannot create self-test directory: {}", e))?;
    let children = self_test_fixture::make_children();
    let cases = self_test_cases::build_cases(root)
        .map_err(|e| format!("self-test fixture setup: {}", e))?;
    let mut results = run_test_cases(root, &children, &cases)?;
    results.extend(run_manifest_parse_cases(root, &children)?);
    results.extend(run_missing_child_case(&children)?);
    report_failures(&results)
}

fn report_failures(results: &[CaseResult]) -> Result<i32, String> {
    let failures: Vec<&CaseResult> = results.iter().filter(|r| !r.passed).collect();
    if failures.is_empty() {
        return Ok(0);
    }
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    for result in &failures {
        write_failure_report(&mut handle, result)?;
    }
    Ok(1)
}

fn write_failure_report<W: Write>(mut handle: W, result: &CaseResult) -> Result<(), String> {
    let expected = if result.expected_code == 0 {
        "exit code 0 (pass)".to_string()
    } else {
        format!("exit code 1 ({})", result.expected)
    };
    let detail = describe_failures(&result.actual_failures);
    let actual = if result.actual_code == 0 {
        "exit code 0 (pass)".to_string()
    } else if detail == "exit code 0 (pass)" {
        "exit code 1".to_string()
    } else {
        format!("exit code 1 ({})", detail)
    };
    let line = format!("- {} FAILED\n", result.label);
    handle
        .write_all(line.as_bytes())
        .map_err(|e| format!("cannot write stderr: {}", e))?;
    let msg = format!("  expected: {}\n  actual:   {}\n", expected, actual);
    handle
        .write_all(msg.as_bytes())
        .map_err(|e| format!("cannot write stderr: {}", e))?;
    Ok(())
}

#[derive(Debug)]
struct CaseResult {
    label: &'static str,
    passed: bool,
    expected: String,
    expected_code: i32,
    actual_code: i32,
    actual_failures: Vec<String>,
}

fn run_test_cases(
    root: &str,
    children: &[Issue],
    cases: &[self_test_cases::TestCase],
) -> Result<Vec<CaseResult>, String> {
    let manifest_path = format!(
        "{}/.evidence/vb-jpq7.48/scratch/self-test/manifest.jsonl",
        root
    );
    let p = Path::new(&manifest_path);
    let mut results = Vec::new();
    for case in cases {
        results.push(run_single_case(children, case, p)?);
    }
    Ok(results)
}

fn run_single_case(
    children: &[Issue],
    case: &self_test_cases::TestCase,
    manifest_path: &Path,
) -> Result<CaseResult, String> {
    let text = serialize_rows(&case.rows)?;
    self_test_fixture::write_manifest(manifest_path, &text)?;
    let (rows, row_errors) = parse_manifest(&text);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    let failures = join_row_errors_and_failures(row_errors, outcome.failures);
    let code = if failures.is_empty() { 0 } else { 1 };
    let passed = matches_outcome(
        code,
        &failures,
        case.expect_code,
        case.expect_failure_contains,
    );
    Ok(CaseResult {
        label: case.label,
        passed,
        expected: describe_expected(case.expect_failure_contains),
        expected_code: case.expect_code,
        actual_code: code,
        actual_failures: failures,
    })
}

fn run_manifest_parse_cases(root: &str, children: &[Issue]) -> Result<Vec<CaseResult>, String> {
    let manifest_path = format!(
        "{}/.evidence/vb-jpq7.48/scratch/self-test/manifest.jsonl",
        root
    );
    let p = Path::new(&manifest_path);
    Ok(vec![
        run_invalid_jsonl_case(children, p)?,
        run_empty_manifest_case(children, p)?,
    ])
}

fn run_invalid_jsonl_case(children: &[Issue], manifest_path: &Path) -> Result<CaseResult, String> {
    let text = "{not-json}\n";
    self_test_fixture::write_manifest(manifest_path, text)?;
    let (rows, errors) = parse_manifest(text);
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    let failures = join_row_errors_and_failures(errors, outcome.failures);
    let expected = "invalid JSONL row".to_string();
    let passed = failures.iter().any(|f| f.contains(&expected));
    let code = if failures.is_empty() { 0 } else { 1 };
    Ok(CaseResult {
        label: "invalid json",
        passed,
        expected: expected.clone(),
        expected_code: 1,
        actual_code: code,
        actual_failures: if passed { Vec::new() } else { failures },
    })
}

fn run_empty_manifest_case(children: &[Issue], manifest_path: &Path) -> Result<CaseResult, String> {
    self_test_fixture::write_manifest(manifest_path, "")?;
    let (rows, errors) = parse_manifest("");
    let outcome = validate_closed_children(Path::new(""), children, &rows);
    let failures = join_row_errors_and_failures(errors, outcome.failures);
    let expected = "manifest has no evidence rows".to_string();
    let passed = failures.iter().any(|f| f.contains(&expected));
    let code = if failures.is_empty() { 0 } else { 1 };
    Ok(CaseResult {
        label: "empty manifest",
        passed,
        expected: expected.clone(),
        expected_code: 1,
        actual_code: code,
        actual_failures: if passed { Vec::new() } else { failures },
    })
}

fn run_missing_child_case(children: &[Issue]) -> Result<Vec<CaseResult>, String> {
    let empty_rows: Vec<(usize, serde_json::Value)> = Vec::new();
    let outcome = validate_closed_children(Path::new(""), children, &empty_rows);
    let expected = "closed child lacks".to_string();
    let passed =
        !outcome.failures.is_empty() && outcome.failures.iter().any(|f| f.contains(&expected));
    let code = if outcome.failures.is_empty() { 0 } else { 1 };
    Ok(vec![CaseResult {
        label: "missing child row",
        passed,
        expected: expected.clone(),
        expected_code: 1,
        actual_code: code,
        actual_failures: if passed { Vec::new() } else { outcome.failures },
    }])
}

fn serialize_rows(rows: &[serde_json::Value]) -> Result<String, String> {
    let mut parts = Vec::with_capacity(rows.len());
    for row in rows {
        parts.push(
            serde_json::to_string(row)
                .map_err(|e| format!("manifest row serialization failed: {}", e))?,
        );
    }
    Ok(parts.join("\n"))
}

fn join_row_errors_and_failures(
    row_errors: Vec<String>,
    validation_failures: Vec<String>,
) -> Vec<String> {
    let mut all = row_errors;
    all.extend(validation_failures);
    all
}

fn matches_outcome(
    actual_code: i32,
    actual_failures: &[String],
    expected_code: i32,
    expected_contains: Option<&str>,
) -> bool {
    if actual_code != expected_code {
        return false;
    }
    if let Some(substring) = expected_contains {
        return actual_failures.iter().any(|f| f.contains(substring));
    }
    true
}

fn describe_expected(expected_contains: Option<&str>) -> String {
    match expected_contains {
        Some(substring) => format!("failure containing '{}'", substring),
        None => "exit code 0 (pass)".to_string(),
    }
}

fn describe_failures(failures: &[String]) -> String {
    if failures.is_empty() {
        return "exit code 0 (pass)".to_string();
    }
    let first = match failures.first() {
        Some(f) => f,
        None => return "no failures".to_string(),
    };
    if failures.len() == 1 {
        return format!("'{}'", first);
    }
    format!(
        "'{}' and {} others",
        first,
        failures.len().saturating_sub(1)
    )
}

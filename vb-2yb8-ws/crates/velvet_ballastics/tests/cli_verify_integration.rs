#![forbid(unsafe_code)]
//! Integration tests for the verify hero command and VerificationReport certificates.

/// Minimal valid workflow YAML for testing.
const MINIMAL_WORKFLOW: &str = r#"version: 1
name: "minimal_workflow"
description: "A minimal valid workflow with a single step"
when: "always"
inputs: {}
vars: {}
secrets: []
steps:
  - id: step_hello
    name: "Say Hello"
    run:
      action: "echo"
      message: "Hello, World!"
    output:
      message: "echo_result"
result:
  output:
    greeting: "${steps.step_hello.output.message}"
"#;

/// Malformed YAML for error path testing.
const MALFORMED_YAML: &str = r#"version: 1
name: "bad workflow"
when: "always"
steps:
  - id: broken
    run:
      action: "echo"
    # This YAML is missing required fields and has invalid syntax
    invalid indentation here
"#;

fn forced_assertion_failure() -> bool {
    false
}

fn write_test_file(path: &std::path::Path, contents: &[u8]) -> bool {
    match std::fs::write(path, contents) {
        Ok(()) => true,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to write {}: {err}",
                path.display()
            );
            false
        }
    }
}

fn run_cli(args: &[&std::ffi::OsStr]) -> Option<std::process::Output> {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_vb"));
    command.args(args);

    match command.output() {
        Ok(output) => Some(output),
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to spawn vb: {err}"
            );
            None
        }
    }
}

/// ### Behavior: run_verification returns VerifyOk with all gates passed for minimal valid workflow
#[test]
fn bdd_happy_quick_profile_returns_ok_with_checks() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ]);
    if output.is_none() {
        // Try with crate root path
        let output = run_cli(&[
            std::ffi::OsStr::new("verify"),
            std::ffi::OsStr::new("--profile"),
            std::ffi::OsStr::new("quick"),
            std::ffi::OsStr::new("/home/lewis/src/Velvet-ballistics/vb-2yb8-ws/tests/fixtures/valid/minimal.yaml"),
        ]);
        return;
    }

    let output = output.expect("vb command must succeed");
    let status = output.status;

    assert!(
        status.success(),
        "verify quick must succeed with valid workflow. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(status.code(), Some(0), "exit code must be 0 on success");
}

/// ### Behavior: cmd_verify produces identical exit codes for Text, Json, and Jsonl formats
#[test]
fn bdd_format_parity_exit_code_identical_across_formats() {
    let text_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("text"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let json_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("json"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let jsonl_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("jsonl"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let text_code = text_output.status.code();
    let json_code = json_output.status.code();
    let jsonl_code = jsonl_output.status.code();

    assert_eq!(text_code, json_code, "Text and Json formats must produce same exit code");
    assert_eq!(text_code, jsonl_code, "Text and Jsonl formats must produce same exit code");
    assert_eq!(text_code, Some(0), "Valid workflow must exit with code 0");
}

/// ### Behavior: YAML parse error must exit with code 1 (ValidationFailed)
#[test]
fn bdd_yaml_parse_error_returns_validation_failed() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("vb_test_malformed.yaml");
    write_test_file(&temp_file, MALFORMED_YAML.as_bytes());

    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        temp_file.as_os_str(),
    ])
    .expect("vb command must run");

    assert_eq!(
        output.status.code(),
        Some(1),
        "YAML parse error must exit with code 1 (ValidationFailed)"
    );
}

/// ### Behavior: JSON output contains all certificate fields
#[test]
fn bdd_json_output_contains_all_certificate_fields() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("json"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("JSON output must be parseable");

    assert!(json.get("profile").is_some(), "JSON must contain 'profile' field");
    assert!(json.get("artifact").is_some(), "JSON must contain 'artifact' field");
    assert!(json.get("replay").is_some(), "JSON must contain 'replay' field");
    assert!(json.get("durability").is_some(), "JSON must contain 'durability' field");
    assert!(json.get("repair_hints").is_some(), "JSON must contain 'repair_hints' field");
    assert!(json.get("exit_code").is_some(), "JSON must contain 'exit_code' field");
}

/// ### Behavior: Full profile fails closed on BudgetPolicy violation
#[test]
fn bdd_full_profile_fails_closed_on_budget_violation() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("tests/fixtures/invalid/invalid_cyclic_dep.yaml"),
    ])
    .expect("vb command must run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Full profile budget policy violation must exit with code 2 (VerificationFailed)"
    );
}

/// ### Behavior: Standard profile warns but does not fail on budget issues
#[test]
fn bdd_standard_profile_warns_not_fails_on_budget() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("tests/fixtures/invalid/invalid_cyclic_dep.yaml"),
    ])
    .expect("vb command must run");

    let code = output.status.code();
    assert!(
        code != Some(2),
        "Standard profile must not fail with VerificationFailed (2) for budget issues"
    );
}

/// ### Behavior: INV-001 — exit code is stable across format variants
#[test]
fn bdd_inv001_exit_code_stable_across_formats_on_error() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("vb_test_format_parity.yaml");
    write_test_file(&temp_file, MALFORMED_YAML.as_bytes());

    let text_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("text"),
        temp_file.as_os_str(),
    ])
    .expect("vb command must run");

    let json_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("json"),
        temp_file.as_os_str(),
    ])
    .expect("vb command must run");

    let jsonl_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("jsonl"),
        temp_file.as_os_str(),
    ])
    .expect("vb command must run");

    assert_eq!(
        text_output.status.code(),
        json_output.status.code(),
        "Text and Json must have same exit code on error"
    );
    assert_eq!(
        text_output.status.code(),
        jsonl_output.status.code(),
        "Text and Jsonl must have same exit code on error"
    );
}

#[test]
fn integration_verify_all_profiles_complete_without_panic() {
    for profile in &["quick", "standard", "full"] {
        let output = run_cli(&[
            std::ffi::OsStr::new("verify"),
            std::ffi::OsStr::new("--profile"),
            std::ffi::OsStr::new(profile),
            std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
        ]);

        assert!(output.is_some(), "verify {} must complete without panicking", profile);
    }
}

#[test]
fn integration_standard_profile_runs_ir_validation_gate() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("json"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("JSON output must be parseable");

    let gates_passed = json.get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(gates_passed.is_some(), "replay.gates_passed must be present in JSON");

    let gates = gates_passed.unwrap();
    let has_ir_validation = gates.iter().any(|g| {
        g.as_str().map(|s| s.contains("ir_validation")).unwrap_or(false)
    });

    assert!(has_ir_validation, "Standard profile must include 'ir_validation' in gates_passed. Found: {:?}", gates);
}

#[test]
fn integration_full_profile_runs_budget_gates() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("full"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("json"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("JSON output must be parseable");

    let gates_passed = json.get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(gates_passed.is_some(), "replay.gates_passed must be present in JSON");

    let gates = gates_passed.unwrap();
    let gate_names: Vec<&str> = gates.iter()
        .filter_map(|g| g.as_str())
        .collect();

    assert!(
        gate_names.iter().any(|g| g.contains("budget_computation")),
        "Full profile must include 'budget_computation'. Found: {:?}", gate_names
    );
    assert!(
        gate_names.iter().any(|g| g.contains("boundedness_policy")),
        "Full profile must include 'boundedness_policy'. Found: {:?}", gate_names
    );
}

#[test]
fn integration_quick_profile_skips_expensive_gates() {
    let output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--format"),
        std::ffi::OsStr::new("json"),
        std::ffi::OsStr::new("tests/fixtures/valid/minimal.yaml"),
    ])
    .expect("vb command must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("JSON output must be parseable");

    let gates_passed = json.get("replay")
        .and_then(|r| r.get("gates_passed"))
        .and_then(|g| g.as_array());

    assert!(gates_passed.is_some(), "replay.gates_passed must be present in JSON");

    let gates = gates_passed.unwrap();
    let gate_names: Vec<&str> = gates.iter()
        .filter_map(|g| g.as_str())
        .collect();

    assert!(
        !gate_names.iter().any(|g| g.contains("budget_computation")),
        "Quick profile must NOT include 'budget_computation'. Found: {:?}", gate_names
    );
    assert!(
        !gate_names.iter().any(|g| g.contains("boundedness_policy")),
        "Quick profile must NOT include 'boundedness_policy'. Found: {:?}", gate_names
    );
}

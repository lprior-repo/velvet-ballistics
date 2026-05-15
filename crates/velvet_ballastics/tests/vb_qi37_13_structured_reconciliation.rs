#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::process::Output;

use serde_json::Value;

const EXPECTED_UNKNOWN_COMMAND_MADEUP: &str = "unknown command: madeup (expected one of: help, version, agent-context, ai-context, status, action, validate, verify, explain, compile, run, run-compiled, ipc-serve, inspect, events, replay, trace, retry, resume, bench-run, doctor, answer, graph, diff, incident, submit, simulate, cancel)";
const EXPECTED_UNKNOWN_EMIT_XML: &str =
    "unknown emit target: xml (expected: ir, rust, yaml, postcard)";
const EXPECTED_STATUS_POSTCARD_EMIT: &str =
    "invalid status argument: postcard emit is not supported for status";
const VALID_WORKFLOW: &str = r"version: velvet-ballastics/v1
name: structured_matrix
when:
  manual: {}
steps:
  - id: build_result
    save:
      value: 42
  - id: done
    finish:
      result: 0
";

fn run_cli(args: &[&OsStr]) -> Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"));
    command.args(args);
    match command.output() {
        Ok(output) => output,
        Err(error) => panic!("failed to execute velvet-ballastics: {error}"),
    }
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn parse_json(bytes: &[u8], channel: &str) -> Value {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(value) => value,
        Err(error) => panic!(
            "{channel} must contain valid JSON: {error}; bytes={}",
            String::from_utf8_lossy(bytes)
        ),
    }
}

fn write_file(path: &std::path::Path, bytes: &[u8]) {
    if let Err(error) = std::fs::write(path, bytes) {
        panic!("failed to write {}: {error}", path.display());
    }
}

fn assert_success_channel_contract(output: &Output, command_name: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{command_name} must exit 0; stderr={}",
        stderr_text(output)
    );
    assert_eq!(
        stderr_text(output),
        "",
        "{command_name} success must not emit diagnostics on stderr"
    );
    let parsed = parse_json(&output.stdout, "stdout");
    assert_ne!(
        parsed.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string())),
        "{command_name} success stdout must not be a diagnostic envelope"
    );
    parsed
}

fn assert_structured_validation_diagnostic(output: &Output, command_name: &str, expected: &str) {
    assert_structured_diagnostic(output, command_name, "ValidationFailed", 1, expected);
}

fn assert_structured_diagnostic(
    output: &Output,
    command_name: &str,
    code: &str,
    exit_code: i64,
    expected: &str,
) {
    assert_eq!(
        output.status.code(),
        Some(i32::try_from(exit_code).unwrap_or(255)),
        "{command_name} must exit {code}={exit_code}; stdout={} stderr={}",
        stdout_text(output),
        stderr_text(output)
    );
    assert_eq!(
        stdout_text(output),
        "",
        "{command_name} failure must not emit success payload on stdout"
    );

    let diagnostic = parse_json(&output.stderr, "stderr");
    assert_eq!(
        diagnostic.get("schema_version"),
        Some(&Value::String(
            "velvet-ballastics/cli-output/v1".to_string()
        )),
        "{command_name} diagnostic must use stable structured schema"
    );
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string())),
        "{command_name} diagnostic must declare DiagnosticReport kind"
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String(code.to_string())),
        "{command_name} diagnostic code must be stable"
    );
    assert_eq!(
        diagnostic.get("exit_code"),
        Some(&Value::Number(exit_code.into())),
        "{command_name} diagnostic must carry public exit code {exit_code}"
    );
    assert_eq!(
        diagnostic.get("message"),
        Some(&Value::String(expected.to_string())),
        "{command_name} diagnostic message must exactly match the stable contract: {diagnostic}"
    );
}

#[test]
fn cli_public_exit_code_matrix_is_exactly_zero_through_eight_in_agent_context() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let context = assert_success_channel_contract(&output, "agent-context");

    assert_eq!(
        context.get("schema_version"),
        Some(&Value::String("1".to_string()))
    );
    assert_eq!(
        context.get("kind"),
        Some(&Value::String("AgentContext".to_string()))
    );

    let exit_codes = match context.get("exit_codes") {
        Some(Value::Object(codes)) => codes,
        other => panic!("exit_codes must be a JSON object, got {other:?}"),
    };
    let observed: Vec<_> = exit_codes.keys().map(String::as_str).collect();
    assert_eq!(observed, vec!["0", "1", "2", "3", "4", "5", "6", "7", "8"]);
    assert_eq!(
        exit_codes.get("0"),
        Some(&Value::String("success".to_string()))
    );
    assert_eq!(
        exit_codes.get("1"),
        Some(&Value::String("validation failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("2"),
        Some(&Value::String("verification failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("3"),
        Some(&Value::String("compile failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("4"),
        Some(&Value::String("runtime failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("5"),
        Some(&Value::String("storage error".to_string()))
    );
    assert_eq!(
        exit_codes.get("6"),
        Some(&Value::String("ipc error".to_string()))
    );
    assert_eq!(
        exit_codes.get("7"),
        Some(&Value::String("action policy error".to_string()))
    );
    assert_eq!(
        exit_codes.get("8"),
        Some(&Value::String("replay divergence".to_string()))
    );
    assert_eq!(exit_codes.get("9"), None);
}

#[test]
fn structured_success_matrix_writes_only_payloads_to_stdout() {
    let json = run_cli(&[OsStr::new("status"), OsStr::new("--json")]);
    let json_payload = assert_success_channel_contract(&json, "status --json");
    assert_eq!(
        json_payload.get("schema_version"),
        Some(&Value::String(
            "velvet-ballastics/cli-output/v1".to_string()
        ))
    );
    assert_eq!(
        json_payload.get("kind"),
        Some(&Value::String("CliStatus".to_string()))
    );
    assert_eq!(
        json_payload.get("status"),
        Some(&Value::String("running".to_string()))
    );

    let jsonl = run_cli(&[OsStr::new("status"), OsStr::new("--jsonl")]);
    assert_eq!(jsonl.status.code(), Some(0));
    assert_eq!(stderr_text(&jsonl), "");
    let stdout = stdout_text(&jsonl);
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "status --jsonl must emit exactly one JSON line"
    );
    let parsed_line = parse_json(lines[0].as_bytes(), "stdout jsonl line");
    assert_eq!(
        parsed_line.get("schema_version"),
        Some(&Value::String(
            "velvet-ballastics/cli-output/v1".to_string()
        ))
    );
    assert_eq!(
        parsed_line.get("kind"),
        Some(&Value::String("CliStatus".to_string()))
    );
    assert_eq!(
        parsed_line.get("status"),
        Some(&Value::String("running".to_string()))
    );
}

#[test]
fn unknown_command_json_emits_structured_validation_diagnostic_to_stderr_only() {
    let output = run_cli(&[OsStr::new("madeup"), OsStr::new("--json")]);
    assert_structured_validation_diagnostic(
        &output,
        "madeup --json",
        EXPECTED_UNKNOWN_COMMAND_MADEUP,
    );
}

#[test]
fn unknown_command_jsonl_emits_one_structured_validation_diagnostic_line_to_stderr_only() {
    let output = run_cli(&[OsStr::new("madeup"), OsStr::new("--jsonl")]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    let stderr = stderr_text(&output);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "JSONL diagnostic stderr must be exactly one line"
    );
    let diagnostic = parse_json(lines[0].as_bytes(), "stderr jsonl line");
    assert_eq!(
        diagnostic.get("schema_version"),
        Some(&Value::String(
            "velvet-ballastics/cli-output/v1".to_string()
        ))
    );
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(1.into())));
    assert_eq!(
        diagnostic.get("message"),
        Some(&Value::String(EXPECTED_UNKNOWN_COMMAND_MADEUP.to_string())),
        "JSONL diagnostic must carry the exact stable unknown-command message: {diagnostic}"
    );
}

#[test]
fn unsupported_emit_mode_json_emits_structured_validation_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("workflow.yaml");
    let out = dir.path().join("out.bin");
    let output = run_cli(&[
        OsStr::new("compile"),
        workflow.as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("xml"),
        OsStr::new("--out"),
        out.as_os_str(),
        OsStr::new("--json"),
    ]);

    assert_structured_validation_diagnostic(
        &output,
        "compile --emit xml --json",
        EXPECTED_UNKNOWN_EMIT_XML,
    );
}

#[test]
fn unsupported_status_emit_mode_json_emits_structured_validation_diagnostic_to_stderr_only() {
    let output = run_cli(&[
        OsStr::new("status"),
        OsStr::new("--emit"),
        OsStr::new("postcard"),
        OsStr::new("--json"),
    ]);

    assert_structured_validation_diagnostic(
        &output,
        "status --emit postcard --json",
        EXPECTED_STATUS_POSTCARD_EMIT,
    );
}

#[test]
fn missing_file_validate_json_emits_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("missing.yaml");
    let expected = format!("error reading {}: ", workflow.display());
    let output = run_cli(&[
        OsStr::new("validate"),
        workflow.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    let diagnostic = parse_json(&output.stderr, "stderr");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(1.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => panic!("diagnostic message missing: {diagnostic}"),
    };
    assert!(message.starts_with(&expected), "message was {message}");
}

#[test]
fn malformed_yaml_validate_jsonl_emits_one_diagnostic_line() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("workflow.yaml");
    write_file(&workflow, b"{{{not-yaml");
    let output = run_cli(&[
        OsStr::new("validate"),
        workflow.as_os_str(),
        OsStr::new("--jsonl"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    let stderr = stderr_text(&output);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines.len(), 1);
    let diagnostic = parse_json(lines[0].as_bytes(), "stderr jsonl line");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(1.into())));
}

#[test]
fn invalid_utf8_verify_json_emits_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("invalid-utf8.yaml");
    write_file(&workflow, &[0xff, 0xfe, 0xfd]);
    let output = run_cli(&[
        OsStr::new("verify"),
        workflow.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    let diagnostic = parse_json(&output.stderr, "stderr");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(1.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => panic!("diagnostic message missing: {diagnostic}"),
    };
    assert!(
        message.starts_with("file is not valid UTF-8: "),
        "message was {message}"
    );
}

#[test]
fn invalid_utf8_verify_jsonl_emits_one_diagnostic_line_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("invalid-utf8.yaml");
    write_file(&workflow, &[0xff, 0xfe, 0xfd]);
    let output = run_cli(&[
        OsStr::new("verify"),
        workflow.as_os_str(),
        OsStr::new("--jsonl"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    let stderr = stderr_text(&output);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines.len(), 1);
    let diagnostic = parse_json(lines[0].as_bytes(), "stderr jsonl line");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(1.into())));
}

#[test]
fn invalid_run_inspect_json_emits_validation_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let db = dir.path().join("db");
    let output = run_cli(&[
        OsStr::new("inspect"),
        OsStr::new("not-a-run"),
        OsStr::new("--db"),
        db.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    let diagnostic = parse_json(&output.stderr, "stderr");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(1.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => panic!("diagnostic message missing: {diagnostic}"),
    };
    assert!(
        message.starts_with("invalid run_id 'not-a-run': "),
        "message was {message}"
    );
}

#[test]
fn missing_file_compile_json_emits_compile_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("missing.yaml");
    let out = dir.path().join("out.ir");
    let output = run_cli(&[
        OsStr::new("compile"),
        workflow.as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("ir"),
        OsStr::new("--out"),
        out.as_os_str(),
        OsStr::new("--json"),
    ]);
    let expected_prefix = format!("error reading {}: ", workflow.display());
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout_text(&output), "");
    let diagnostic = parse_json(&output.stderr, "stderr");
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("CompileFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(3.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => panic!("diagnostic message missing: {diagnostic}"),
    };
    assert!(
        message.starts_with(&expected_prefix),
        "message was {message}"
    );
}

#[test]
fn runtime_input_decode_json_emits_runtime_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let workflow = dir.path().join("workflow.yaml");
    let input = dir.path().join("input.bin");
    write_file(&workflow, VALID_WORKFLOW.as_bytes());
    write_file(&input, b"not-postcard");
    let output = run_cli(&[
        OsStr::new("run"),
        workflow.as_os_str(),
        OsStr::new("--input-bin"),
        input.as_os_str(),
        OsStr::new("--durability"),
        OsStr::new("none"),
        OsStr::new("--json"),
    ]);
    assert_structured_diagnostic(
        &output,
        "run invalid input --json",
        "RuntimeFailed",
        4,
        "INPUT_MAPPING_FAILED: input-bin decode failed",
    );
}

#[test]
fn storage_open_json_emits_storage_diagnostic_to_stderr_only() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("tempdir must be available: {error}"),
    };
    let db = dir.path().join("not-a-directory");
    write_file(&db, b"not a fjall directory");
    let blocked_db = db.join("child");
    let output = run_cli(&[
        OsStr::new("inspect"),
        OsStr::new("1"),
        OsStr::new("--db"),
        blocked_db.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(stdout_text(&output), "");
    let diagnostic = parse_json(&output.stderr, "stderr");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("StorageError".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(5.into())));
}

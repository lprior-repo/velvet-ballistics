#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::process::Output;

use serde_json::Value;

const EXPECTED_UNKNOWN_COMMAND_MADEUP: &str = "unknown command: madeup (expected one of: help, version, agent-context, ai-context, status, system, action, validate, verify, explain, compile, run, run-compiled, ipc-serve, inspect, events, replay, trace, retry, resume, bench-run, doctor, answer, graph, diff, incident, submit, simulate, cancel)";
const EXPECTED_UNKNOWN_EMIT_XML: &str = "unknown emit target: xml (expected: ir, yaml, postcard)";
const EXPECTED_STATUS_POSTCARD_EMIT: &str =
    "invalid status argument: postcard emit is not supported for status";
const VALID_WORKFLOW: &str = r"version: velvet-ballastics/v1
name: structured_matrix
when:
  manual: {}
steps:
  - id: build_result
    save:
      output: saved
      value: '42'
  - id: done
    finish:
      result: saved
";

fn run_cli(args: &[&OsStr]) -> Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"));
    command.args(args);
    let output = command.output();
    assert!(
        output.is_ok(),
        "failed to execute velvet-ballastics: {output:?}"
    );
    output.unwrap_or_else(|_| std::process::abort())
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn parse_json(bytes: &[u8], channel: &str) -> Value {
    let parsed = serde_json::from_slice::<Value>(bytes);
    assert!(
        parsed.is_ok(),
        "{channel} must contain valid JSON; bytes={}",
        String::from_utf8_lossy(bytes)
    );
    parsed.unwrap_or(Value::Null)
}

fn write_file(path: &std::path::Path, bytes: &[u8]) {
    let written = std::fs::write(path, bytes);
    assert!(
        written.is_ok(),
        "failed to write {}: {written:?}",
        path.display()
    );
}

fn tempdir() -> tempfile::TempDir {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/structured-reconciliation-tmp");
    let root_ready = std::fs::create_dir_all(&root);
    assert!(
        root_ready.is_ok(),
        "temp root must be available: {root_ready:?}"
    );
    let dir = tempfile::Builder::new()
        .prefix("vb-structured-")
        .tempdir_in(root);
    assert!(dir.is_ok(), "tempdir must be available: {dir:?}");
    dir.unwrap_or_else(|_| std::process::abort())
}

fn first_line<'a>(lines: &'a [&'a str]) -> &'a str {
    lines.first().copied().unwrap_or_default()
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
    assert_structured_diagnostic(output, command_name, "ValidationFailed", 2, expected);
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
        Some(&Value::String(
            "velvet-ballastics/cli-output/v1".to_string()
        ))
    );
    assert_eq!(
        context.get("kind"),
        Some(&Value::String("AgentContext".to_string()))
    );

    let exit_codes = match context.get("exit_codes") {
        Some(Value::Object(codes)) => codes,
        other => {
            assert!(
                matches!(other, Some(Value::Object(_))),
                "exit_codes must be a JSON object, got {other:?}"
            );
            return;
        }
    };
    let observed: Vec<_> = exit_codes.keys().map(String::as_str).collect();
    assert_eq!(observed, vec!["0", "1", "2", "3", "4", "5", "6", "7", "8"]);
    assert_eq!(
        exit_codes.get("0"),
        Some(&Value::String("success".to_string()))
    );
    assert_eq!(
        exit_codes.get("1"),
        Some(&Value::String("runtime failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("2"),
        Some(&Value::String("validation failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("3"),
        Some(&Value::String("compile failed".to_string()))
    );
    assert_eq!(
        exit_codes.get("4"),
        Some(&Value::String("verification failed".to_string()))
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
fn agent_context_matches_canonical_operator_surface() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let context = assert_success_channel_contract(&output, "agent-context");

    assert_eq!(
        context.get("cli"),
        Some(&Value::String("velvet-ballastics".to_string()))
    );
    assert_eq!(
        context.get("package"),
        Some(&Value::String("velvet-ballastics".to_string()))
    );
    assert_eq!(
        context.get("binary_aliases"),
        Some(&Value::Array(vec![Value::String(
            "velvet-ballastics".to_string()
        )]))
    );
    assert_eq!(
        context
            .get("agent_contract")
            .and_then(|contract| contract.get("structured_output_flag")),
        Some(&Value::String("--emit".to_string()))
    );
    assert_eq!(
        context
            .get("vocabulary_policy")
            .and_then(|policy| policy.get("canonical_output_flag")),
        Some(&Value::String("--emit".to_string()))
    );

    let commands = context
        .get("commands")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for command in [
        "agent-context",
        "ai-context",
        "status",
        "system status",
        "action list",
        "action inspect",
        "cancel",
        "validate",
        "verify",
        "events",
        "trace",
        "replay",
        "diff",
        "explain",
    ] {
        assert!(
            commands.contains_key(command),
            "agent-context omitted {command}"
        );
    }

    let validate_flags = commands
        .get("validate")
        .and_then(|command| command.get("flags"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(validate_flags.contains_key("--emit"));
    assert!(!validate_flags.contains_key("--json"));
    assert!(!validate_flags.contains_key("--jsonl"));

    let compile_emit_values = commands
        .get("compile")
        .and_then(|command| command.get("flags"))
        .and_then(|flags| flags.get("--emit"))
        .and_then(|emit| emit.get("values"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        compile_emit_values,
        vec![
            Value::String("ir".to_string()),
            Value::String("yaml".to_string()),
            Value::String("postcard".to_string())
        ]
    );

    let advertised_commands = Value::Object(commands).to_string();
    assert!(!advertised_commands.contains("--format=json"));
    assert!(!advertised_commands.contains("--output=json"));
}

#[test]
fn agent_context_examples_are_executable() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let context = assert_success_channel_contract(&output, "agent-context");
    let examples = context
        .get("examples")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !examples.is_empty(),
        "agent-context must advertise runnable examples"
    );

    for example in examples {
        let args = example
            .get("args")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let expected_exit = example
            .get("expect_exit")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"));
        for arg in &args {
            let Some(raw) = arg.as_str() else {
                continue;
            };
            command.arg(raw);
        }
        let actual = command.output();
        assert!(actual.is_ok(), "example must execute: {args:?}");
        let actual = actual.unwrap_or_else(|_| std::process::abort());
        assert_eq!(
            actual.status.code(),
            Some(i32::try_from(expected_exit).unwrap_or(0)),
            "example {args:?} exited unexpectedly; stdout={} stderr={}",
            stdout_text(&actual),
            stderr_text(&actual)
        );
    }
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
    let parsed_line = parse_json(first_line(&lines).as_bytes(), "stdout jsonl line");
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
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout_text(&output), "");
    let stderr = stderr_text(&output);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "JSONL diagnostic stderr must be exactly one line"
    );
    let diagnostic = parse_json(first_line(&lines).as_bytes(), "stderr jsonl line");
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
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(2.into())));
    assert_eq!(
        diagnostic.get("message"),
        Some(&Value::String(EXPECTED_UNKNOWN_COMMAND_MADEUP.to_string())),
        "JSONL diagnostic must carry the exact stable unknown-command message: {diagnostic}"
    );
}

#[test]
fn unsupported_emit_mode_json_emits_structured_validation_diagnostic_to_stderr_only() {
    let dir = tempdir();
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
    let dir = tempdir();
    let workflow = dir.path().join("missing.yaml");
    let expected = format!("error reading {}: ", workflow.display());
    let output = run_cli(&[
        OsStr::new("validate"),
        workflow.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(2));
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
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(2.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => {
            assert!(
                diagnostic.get("message").is_some(),
                "diagnostic message missing: {diagnostic}"
            );
            return;
        }
    };
    assert!(message.starts_with(&expected), "message was {message}");
}

#[test]
fn malformed_yaml_validate_jsonl_emits_one_diagnostic_line() {
    let dir = tempdir();
    let workflow = dir.path().join("workflow.yaml");
    write_file(&workflow, b"{{{not-yaml");
    let output = run_cli(&[
        OsStr::new("validate"),
        workflow.as_os_str(),
        OsStr::new("--jsonl"),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout_text(&output), "");
    let stderr = stderr_text(&output);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines.len(), 1);
    let diagnostic = parse_json(first_line(&lines).as_bytes(), "stderr jsonl line");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(2.into())));
}

#[test]
fn invalid_utf8_verify_json_emits_diagnostic_to_stderr_only() {
    let dir = tempdir();
    let workflow = dir.path().join("invalid-utf8.yaml");
    write_file(&workflow, &[0xff, 0xfe, 0xfd]);
    let output = run_cli(&[
        OsStr::new("verify"),
        workflow.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(2));
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
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(2.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => {
            assert!(
                diagnostic.get("message").is_some(),
                "diagnostic message missing: {diagnostic}"
            );
            return;
        }
    };
    assert!(
        message.starts_with("file is not valid UTF-8: "),
        "message was {message}"
    );
}

#[test]
fn invalid_utf8_verify_jsonl_emits_one_diagnostic_line_to_stderr_only() {
    let dir = tempdir();
    let workflow = dir.path().join("invalid-utf8.yaml");
    write_file(&workflow, &[0xff, 0xfe, 0xfd]);
    let output = run_cli(&[
        OsStr::new("verify"),
        workflow.as_os_str(),
        OsStr::new("--jsonl"),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout_text(&output), "");
    let stderr = stderr_text(&output);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines.len(), 1);
    let diagnostic = parse_json(first_line(&lines).as_bytes(), "stderr jsonl line");
    assert_eq!(
        diagnostic.get("kind"),
        Some(&Value::String("DiagnosticReport".to_string()))
    );
    assert_eq!(
        diagnostic.get("code"),
        Some(&Value::String("ValidationFailed".to_string()))
    );
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(2.into())));
}

#[test]
fn invalid_run_inspect_json_emits_validation_diagnostic_to_stderr_only() {
    let dir = tempdir();
    let db = dir.path().join("db");
    let output = run_cli(&[
        OsStr::new("inspect"),
        OsStr::new("not-a-run"),
        OsStr::new("--db"),
        db.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(2));
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
    assert_eq!(diagnostic.get("exit_code"), Some(&Value::Number(2.into())));
    let message = match diagnostic.get("message").and_then(Value::as_str) {
        Some(message) => message,
        None => {
            assert!(
                diagnostic.get("message").is_some(),
                "diagnostic message missing: {diagnostic}"
            );
            return;
        }
    };
    assert!(
        message.starts_with("invalid run_id 'not-a-run': "),
        "message was {message}"
    );
}

#[test]
fn missing_file_compile_json_emits_compile_diagnostic_to_stderr_only() {
    let dir = tempdir();
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
        None => {
            assert!(
                diagnostic.get("message").is_some(),
                "diagnostic message missing: {diagnostic}"
            );
            return;
        }
    };
    assert!(
        message.starts_with(&expected_prefix),
        "message was {message}"
    );
}

#[test]
fn runtime_input_decode_json_emits_runtime_diagnostic_to_stderr_only() {
    let dir = tempdir();
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
        1,
        "INPUT_MAPPING_FAILED: input-bin decode failed",
    );
}

#[test]
fn storage_open_json_emits_storage_diagnostic_to_stderr_only() {
    let dir = tempdir();
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

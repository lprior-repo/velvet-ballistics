#![forbid(unsafe_code)]
//! Snapshot and determinism tests for agent-context command.

use serde_json::Value;
use std::ffi::OsStr;
use std::process::Output;

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

fn parse_json(bytes: &[u8], channel: &str) -> Value {
    let parsed = serde_json::from_slice::<Value>(bytes);
    assert!(
        parsed.is_ok(),
        "{channel} must contain valid JSON; bytes={}",
        String::from_utf8_lossy(bytes)
    );
    parsed.unwrap_or(Value::Null)
}

#[test]
fn agent_context_output_is_deterministic() {
    let first = run_cli(&[OsStr::new("agent-context")]);
    let second = run_cli(&[OsStr::new("agent-context")]);

    assert_eq!(first.status.code(), Some(0), "agent-context must exit 0");
    assert_eq!(second.status.code(), Some(0), "agent-context must exit 0");

    let first_stdout = String::from_utf8_lossy(&first.stdout);
    let second_stdout = String::from_utf8_lossy(&second.stdout);

    assert_eq!(
        first_stdout, second_stdout,
        "agent-context output must be deterministic across runs"
    );
}

#[test]
fn agent_context_matches_snapshot() {
    let output = run_cli(&[OsStr::new("agent-context")]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "agent-context must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = parse_json(&output.stdout, "stdout");

    let snapshot_bytes = include_bytes!("snapshots/agent_context.json");
    let snapshot = parse_json(snapshot_bytes, "snapshot");

    assert_eq!(
        actual, snapshot,
        "agent-context output must match stored snapshot"
    );
}

#[test]
fn agent_context_has_required_top_level_fields() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let actual = parse_json(&output.stdout, "stdout");

    assert!(actual.get("schema_version").is_some());
    assert!(actual.get("kind").is_some());
    assert!(actual.get("cli").is_some());
    assert!(actual.get("version").is_some());
    assert!(actual.get("active_gates").is_some());
    assert!(actual.get("known_blockers").is_some());
}

#[test]
fn agent_context_has_agent_context_command() {
    let output = run_cli(&[OsStr::new("agent-context")]);
    let actual = parse_json(&output.stdout, "stdout");

    let commands = actual
        .get("commands")
        .and_then(Value::as_object)
        .expect("commands must be an object");
    assert!(
        commands.contains_key("agent-context"),
        "commands must include agent-context"
    );
}

#[test]
fn agent_context_stderr_is_empty_on_success() {
    let output = run_cli(&[OsStr::new("agent-context")]);

    assert_eq!(output.status.code(), Some(0), "agent-context must exit 0");
    assert!(
        output.stderr.is_empty(),
        "agent-context success must not emit on stderr"
    );
}

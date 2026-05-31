use crate::args::ActionRegistryMode, Command, EmitTarget, OutputFormat, ParseError, StepTarget, VerifyProfile, parse_args;
use crate::commands_journal::TraceStatus;
use std::ffi::OsString;
use std::path::PathBuf;

pub fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

#[test]
fn parse_answer_rejects_invalid_step_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "run-1",
        "--step",
        "not-a-step",
        "--value-file",
        "value.bin",
        "--db",
        "test-db",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidStep(ref s)) if s == "not-a-step"),
        "expected InvalidStep(not-a-step), got {parsed:?}"
    );
}

#[test]
fn parse_inspect_includes_output_format() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Inspect { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Inspect {
        run_id, db, output, ..
    }) = parsed
    {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_help_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_version_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "--version"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_agent_context_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context"]));
    assert!(matches!(
        parsed,
        Ok(Command::AgentContext { deliver: None })
    ));
}

#[test]
fn parse_agent_context_deliver_target() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "agent-context",
        "--deliver",
        "file:/tmp/out.jsonl",
    ]));
    assert!(
        matches!(parsed, Ok(Command::AgentContext { deliver: Some(ref target) }) if target == "file:/tmp/out.jsonl")
    );
}

#[test]
fn parse_agent_context_rejects_missing_deliver_target() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--deliver"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "--deliver requires stdout or file:<absolute-path>")
    );
}

#[test]
fn parse_agent_context_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--bogus"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "unknown flag --bogus")
    );
}

#[test]
fn parse_diff_requires_both_run_ids_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "1",
        "2",
        "--db",
        "test-db",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff {
        run_a,
        run_b,
        db,
        output,
    }) = parsed
    {
        assert_eq!(run_a, "1");
        assert_eq!(run_b, "2");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_diff_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "10",
        "20",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_diff_requires_db_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "diff", "1", "2"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_doctor_without_db_is_stateless_text_mode() {
    let parsed = parse_args(&args(&["velvet-ballistics", "doctor"]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_doctor_accepts_optional_db_and_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "doctor",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, Some(PathBuf::from("journal-db")));
        assert_eq!(output, OutputFormat::Yaml);
    }
}


use super::*;

#[test]
fn parse_answer_rejects_invalid_slot_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "run-1",
        "--slot",
        "not-a-slot",
        "--value",
        "value.bin",
        "--db",
        "test-db",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidSlot(ref s)) if s == "not-a-slot"),
        "expected InvalidSlot(not-a-slot), got {parsed:?}"
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
fn parse_agent_context_accepts_webhook_deliver_target_shape() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "agent-context",
        "--deliver",
        "webhook:https://example.invalid/hook",
    ]));
    assert!(
        matches!(parsed, Ok(Command::AgentContext { deliver: Some(ref target) }) if target == "webhook:https://example.invalid/hook")
    );
}

#[test]
fn parse_agent_context_rejects_missing_deliver_target() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--deliver"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "--deliver requires stdout, file:<absolute-path>, or webhook:<url>")
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
        diff_mode: DiffMode::RunAgainst { run_a, run_b, db },
        output,
    }) = parsed
    {
        assert_eq!(run_a, "1".to_string());
        assert_eq!(run_b, "2".to_string());
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
fn parse_diff_allows_workflow_against_workflow_without_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "current.yaml",
        "--against",
        "previous.yaml",
    ]));
    match parsed {
        Ok(Command::Diff {
            diff_mode: DiffMode::WorkflowAgainst { workflow, against },
            output,
        }) => {
            assert_eq!(workflow, PathBuf::from("current.yaml"));
            assert_eq!(against, PathBuf::from("previous.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected workflow diff without db to parse, got {other:?}"),
    }
}

#[test]
fn parse_diff_rejects_workflow_against_with_db_hidden_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "current.yaml",
        "--against",
        "123",
        "--db",
        "test-db",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidArgument(ref reason)) if reason == "diff accepts either workflow --against <old-workflow> without --db, or two run IDs plus --db"),
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

use super::*;

#[test]
fn parse_answer_rejects_legacy_step_before_validating_its_value() {
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

    assert_eq!(
        parsed,
        Err(ParseError::UnknownFlag {
            command: "answer",
            flag: "--step".into(),
        })
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
    assert!(matches!(parsed, Ok(Command::Help { command: None })));
}

#[test]
fn parse_subcommand_help_carries_command_name() {
    let parsed = parse_args(&args(&["velvet-ballistics", "validate", "--help"]));
    assert!(
        matches!(parsed, Ok(Command::Help { command: Some(ref name) }) if name == "validate"),
        "expected Help for validate, got {parsed:?}"
    );
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

fn workflow_diff(extra: &[&str]) -> Result<Command, ParseError> {
    let mut argv = vec!["velvet-ballistics", "diff", "new.yaml"];
    argv.extend_from_slice(extra);
    parse_args(&args(&argv))
}

#[test]
fn parse_workflow_diff_against_file_defaults_to_text() {
    assert_eq!(
        workflow_diff(&["--against", "old.yaml"]),
        Ok(Command::WorkflowDiff {
            workflow: PathBuf::from("new.yaml"),
            against: Some(PathBuf::from("old.yaml")),
            db: None,
            output: OutputFormat::Text,
        })
    );
}

#[test]
fn parse_workflow_diff_db_baseline_with_flags_before_and_after_the_path() {
    let expected = Ok(Command::WorkflowDiff {
        workflow: PathBuf::from("new.yaml"),
        against: None,
        db: Some(PathBuf::from("journal")),
        output: OutputFormat::Yaml,
    });
    assert_eq!(
        workflow_diff(&["--db", "journal", "--emit", "yaml"]),
        expected
    );
    assert_eq!(
        parse_args(&args(&[
            "velvet-ballistics",
            "diff",
            "--emit",
            "yaml",
            "--db",
            "journal",
            "new.yaml"
        ])),
        expected
    );
}

#[test]
fn parse_workflow_diff_accepts_postcard_emit() {
    assert!(matches!(
        workflow_diff(&["--against", "old.yaml", "--emit", "postcard"]),
        Ok(Command::WorkflowDiff {
            output: OutputFormat::Postcard,
            ..
        })
    ));
}

#[test]
fn parse_workflow_diff_requires_exactly_one_baseline() {
    assert!(matches!(
        workflow_diff(&[]),
        Err(ParseError::MissingArgument(message)) if message.contains("--against") && message.contains("--db")
    ));
    assert!(matches!(
        workflow_diff(&["--against", "old.yaml", "--db", "journal"]),
        Err(ParseError::InvalidArgument(message)) if message.contains("mutually exclusive")
    ));
}

#[test]
fn parse_workflow_diff_rejects_json_flags_and_run_pair_with_against() {
    for flag in ["--json", "--jsonl"] {
        assert!(
            matches!(
                workflow_diff(&["--against", "old.yaml", flag]),
                Err(ParseError::InvalidArgument(message)) if message.contains("--emit")
            ),
            "{flag} must be refused"
        );
    }
    let mixed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "1",
        "2",
        "--against",
        "old.yaml",
        "--db",
        "journal",
    ]));
    assert!(
        matches!(mixed, Err(ParseError::InvalidArgument(ref message)) if message.contains("run pair")),
        "unexpected: {mixed:?}"
    );
}

#[test]
fn parse_run_diff_is_unchanged_by_the_workflow_variant() {
    assert_eq!(
        parse_args(&args(&["velvet-ballistics", "diff", "1", "2", "--db", "j"])),
        Ok(Command::Diff {
            run_a: "1".into(),
            run_b: "2".into(),
            db: PathBuf::from("j"),
            output: OutputFormat::Text,
        })
    );
}

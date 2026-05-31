#[test]
fn parse_system_status_rejects_unknown_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--profile",
        "deep",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownProfile(ref p)) if p == "deep"));
}

#[test]
fn parse_system_status_rejects_unknown_server_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
        "remote",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref m)) if m == "remote"));
}

#[test]
fn parse_system_status_rejects_unprobed_server_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
        "strict",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref m)) if m == "strict"));
}

#[test]
fn parse_no_command_returns_error() {
    let parsed = parse_args(&args(&["velvet-ballistics"]));
    assert!(matches!(parsed, Err(ParseError::NoCommand)));
}

#[test]
fn parse_unknown_command_returns_error() {
    let parsed = parse_args(&args(&["velvet-ballistics", "foobar"]));
    assert!(matches!(parsed, Err(ParseError::UnknownCommand(_))));
}

#[test]
fn unknown_command_error_enumerates_valid_commands() {
    let err = ParseError::UnknownCommand(String::from("foobar"));
    let rendered = err.to_string();

    assert!(rendered.contains("expected one of"));
    assert!(rendered.contains("agent-context"));
}

#[test]
fn parse_verify_defaults_to_standard_profile() {
    let parsed = parse_args(&args(&["velvet-ballistics", "verify", "workflow.yaml"]));
    assert!(
        matches!(parsed, Ok(Command::Verify { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Verify {
        workflow,
        profile,
        output,
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(profile, VerifyProfile::Standard);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_verify_accepts_quick_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--profile",
        "quick",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Verify { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Verify { profile, .. }) = parsed {
        assert_eq!(profile, VerifyProfile::Quick);
    }
}

#[test]
fn parse_verify_accepts_full_profile_with_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--profile",
        "full",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Verify { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Verify {
        profile, output, ..
    }) = parsed
    {
        assert_eq!(profile, VerifyProfile::Full);
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_verify_rejects_unknown_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--profile",
        "thorough",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::UnknownProfile(_))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_graph_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballistics", "graph", "workflow.yaml"]));
    assert!(
        matches!(parsed, Ok(Command::Graph { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Graph { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_graph_accepts_yaml_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "graph",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Graph { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Graph { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_simulate_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballistics", "simulate", "workflow.yaml"]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_simulate_accepts_yaml_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_simulate_accepts_postcard_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--emit",
        "postcard",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Postcard);
    }
}

#[test]
fn parse_action_list_accepts_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "list",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::ActionList { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
        assert_eq!(registry, ActionRegistryMode::Registered);
    }
}

// --- Cancel command parsing tests ---

#[test]
fn parse_cancel_accepts_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Cancel {
        run_id,
        db,
        reason,
        output,
    }) = parsed
    {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("journal-db"));
        assert_eq!(reason, None);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_cancel_accepts_reason() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        "user request",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Cancel { reason, .. }) = parsed {
        assert_eq!(reason, Some("user request".to_string()));
    }
}

#[test]
fn parse_cancel_accepts_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_cancel_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballistics", "cancel", "42"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_cancel_rejects_reason_longer_than_256_bytes() {
    let long_reason = "a".repeat(257);
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        &long_reason,
    ]));
    assert!(
        matches!(parsed, Err(ParseError::ReasonTooLong)),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_cancel_accepts_reason_exactly_256_bytes() {
    let reason = "a".repeat(256);
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        &reason,
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected: {parsed:?}"
    );
}

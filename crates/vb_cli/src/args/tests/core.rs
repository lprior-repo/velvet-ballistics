use super::args;
use crate::args::{Command, ParseError, parse_args};

#[test]
fn parse_help_command_via_help() {
    let parsed = parse_args(&args(&["velvet-ballastics", "help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_help_command_via_long_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "--help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_help_command_via_short_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "-h"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_version_command_via_version() {
    let parsed = parse_args(&args(&["velvet-ballastics", "version"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_version_command_via_long_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "--version"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_version_command_via_short_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "-V"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_agent_context_command_defaults_to_no_deliver() {
    let parsed = parse_args(&args(&["velvet-ballastics", "agent-context"]));
    assert!(matches!(
        parsed,
        Ok(Command::AgentContext { deliver: None })
    ));
}

#[test]
fn parse_subcommand_help_returns_help() {
    let parsed = parse_args(&args(&["velvet-ballastics", "verify", "--help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_subcommand_help_with_short_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "run", "-h"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_no_command_returns_error() {
    let parsed = parse_args(&args(&["velvet-ballastics"]));
    assert!(matches!(parsed, Err(ParseError::NoCommand)));
}

#[test]
fn parse_unknown_command_returns_error() {
    let parsed = parse_args(&args(&["velvet-ballastics", "foobar"]));
    assert!(matches!(parsed, Err(ParseError::UnknownCommand(_))));
}

#[test]
fn parse_unknown_command_error_display_contains_expected_commands() {
    let err = ParseError::UnknownCommand(String::from("foobar"));
    let rendered = err.to_string();
    assert!(rendered.contains("expected one of"));
    assert!(rendered.contains("agent-context"));
}

#[test]
fn parse_validate_missing_workflow_produces_missing_argument_error() {
    let parsed = parse_args(&args(&["velvet-ballastics", "validate"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("workflow.yaml"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_verify_missing_workflow_produces_missing_argument_error() {
    let parsed = parse_args(&args(&["velvet-ballastics", "verify"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("workflow.yaml"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_explain_missing_workflow_produces_missing_argument_error() {
    let parsed = parse_args(&args(&["velvet-ballastics", "explain"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("workflow.yaml"))),
        "unexpected: {parsed:?}"
    );
}

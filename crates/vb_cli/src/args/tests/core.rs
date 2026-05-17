use super::args;
use crate::args::{Command, ParseError, parse_args};

#[test]
fn parse_help_command() {
    let parsed = parse_args(&args(&["velvet-ballastics", "help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_version_command() {
    let parsed = parse_args(&args(&["velvet-ballastics", "--version"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_agent_context_command() {
    let parsed = parse_args(&args(&["velvet-ballastics", "agent-context"]));
    assert!(matches!(parsed, Ok(Command::AgentContext)));
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
fn unknown_command_error_enumerates_valid_commands() {
    let err = ParseError::UnknownCommand(String::from("foobar"));
    let rendered = err.to_string();

    assert!(rendered.contains("expected one of"));
    assert!(rendered.contains("agent-context"));
}

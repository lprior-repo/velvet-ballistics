//! Action command parsers.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use super::error::ParseError;
use super::flag_spec::{ActionInspectParseState, ActionListParseState};
use super::types::{ActionRegistryMode, Command, OutputFormat};

pub(super) fn parse_action(args: &[OsString]) -> Result<Command, ParseError> {
    let action_command = args
        .get(2)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::MissingArgument("action subcommand"))?;
    let action_args = match args.get(3..) {
        Some(values) => values,
        None => &[],
    };
    if action_command == "inspect" {
        return parse_action_inspect(action_args);
    }
    if action_command != "list" {
        return Err(ParseError::UnknownActionCommand(action_command.into()));
    }
    let parsed = parse_action_list_args(
        action_args,
        ActionListParseState {
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        },
    )?;
    Ok(Command::ActionList {
        output: parsed.output,
        registry: parsed.registry,
    })
}

fn parse_action_inspect(args: &[OsString]) -> Result<Command, ParseError> {
    let (raw_name, rest) = args
        .split_first()
        .ok_or(ParseError::MissingArgument("action_name"))?;
    let action_name = raw_name
        .to_str()
        .ok_or_else(|| ParseError::InvalidActionName(format!("{raw_name:?}")))?
        .trim()
        .to_string();
    if action_name.is_empty() {
        return Err(ParseError::InvalidActionName("action name is empty".into()));
    }
    if action_name.len() > 64 {
        return Err(ParseError::InvalidActionName(
            "action name exceeds maximum length of 64 characters".into(),
        ));
    }
    if action_name.chars().any(|c| c.is_whitespace()) {
        return Err(ParseError::InvalidActionName(
            "action name contains whitespace".into(),
        ));
    }
    let parsed = parse_action_inspect_args(
        rest,
        ActionInspectParseState {
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        },
    )?;
    Ok(Command::ActionInspect {
        action_name,
        output: parsed.output,
        registry: parsed.registry,
    })
}

fn parse_action_inspect_args(
    args: &[OsString],
    state: ActionInspectParseState,
) -> Result<ActionInspectParseState, ParseError> {
    match args.split_first() {
        None => Ok(state),
        Some((raw, rest)) => match raw.to_str() {
            Some("--emit") => parse_action_inspect_emit(rest, state),
            Some("--registry") => parse_action_inspect_registry_arg(rest, state),
            Some(flag) if flag.starts_with("--") => {
                Err(ParseError::UnknownActionInspectFlag(flag.into()))
            }
            Some(arg) => Err(ParseError::UnexpectedActionInspectArgument(arg.into())),
            None => Err(ParseError::UnexpectedActionInspectArgument(format!(
                "{raw:?}"
            ))),
        },
    }
}

fn parse_action_inspect_emit(
    args: &[OsString],
    state: ActionInspectParseState,
) -> Result<ActionInspectParseState, ParseError> {
    match args.split_first() {
        Some((raw, rest)) => match raw.to_str() {
            Some("yaml") => parse_action_inspect_args(
                rest,
                ActionInspectParseState {
                    output: OutputFormat::Yaml,
                    ..state
                },
            ),
            Some("postcard") => parse_action_inspect_args(
                rest,
                ActionInspectParseState {
                    output: OutputFormat::Postcard,
                    ..state
                },
            ),
            Some("text") => parse_action_inspect_args(rest, state),
            Some(value) => Err(ParseError::InvalidActionInspectArgument(format!(
                "unknown emit mode {value}"
            ))),
            None => Err(ParseError::MissingArgument("--emit")),
        },
        None => Err(ParseError::MissingArgument("--emit")),
    }
}

fn parse_action_list_args(
    args: &[OsString],
    state: ActionListParseState,
) -> Result<ActionListParseState, ParseError> {
    match args.split_first() {
        None => Ok(state),
        Some((raw, rest)) => match raw.to_str() {
            Some("--emit") => parse_action_list_emit(rest, state),
            Some("--registry") => parse_action_registry_arg(rest, state),
            Some(flag) if flag.starts_with("--") => {
                Err(ParseError::UnknownActionListFlag(flag.into()))
            }
            Some(arg) => Err(ParseError::UnexpectedActionListArgument(arg.into())),
            None => Err(ParseError::UnexpectedActionListArgument(format!("{raw:?}"))),
        },
    }
}

fn parse_action_list_emit(
    args: &[OsString],
    state: ActionListParseState,
) -> Result<ActionListParseState, ParseError> {
    match args.split_first() {
        Some((raw, rest)) => match raw.to_str() {
            Some("yaml") => parse_action_list_args(
                rest,
                ActionListParseState {
                    output: OutputFormat::Yaml,
                    ..state
                },
            ),
            Some("postcard") => parse_action_list_args(
                rest,
                ActionListParseState {
                    output: OutputFormat::Postcard,
                    ..state
                },
            ),
            Some("text") => parse_action_list_args(rest, state),
            Some(value) => Err(ParseError::InvalidActionListArgument(format!(
                "unknown emit mode {value}"
            ))),
            None => Err(ParseError::MissingArgument("--emit")),
        },
        None => Err(ParseError::MissingArgument("--emit")),
    }
}

fn parse_action_registry_arg(
    args: &[OsString],
    state: ActionListParseState,
) -> Result<ActionListParseState, ParseError> {
    match args.split_first() {
        Some((raw, rest)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingActionRegistryValue),
            Some(value) => parse_action_registry_mode(value).and_then(|registry| {
                parse_action_list_args(rest, ActionListParseState { registry, ..state })
            }),
            None => Err(ParseError::MissingActionRegistryValue),
        },
        None => Err(ParseError::MissingActionRegistryValue),
    }
}

fn parse_action_inspect_registry_arg(
    args: &[OsString],
    state: ActionInspectParseState,
) -> Result<ActionInspectParseState, ParseError> {
    match args.split_first() {
        Some((raw, rest)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingActionRegistryValue),
            Some(value) => parse_action_registry_mode(value).and_then(|registry| {
                parse_action_inspect_args(rest, ActionInspectParseState { registry, ..state })
            }),
            None => Err(ParseError::MissingActionRegistryValue),
        },
        None => Err(ParseError::MissingActionRegistryValue),
    }
}

fn parse_action_registry_mode(value: &str) -> Result<ActionRegistryMode, ParseError> {
    match value {
        "registered" => Ok(ActionRegistryMode::Registered),
        "empty" => Ok(ActionRegistryMode::Empty),
        "uninitialized" => Ok(ActionRegistryMode::Uninitialized),
        other => Err(ParseError::UnknownActionRegistry(other.into())),
    }
}

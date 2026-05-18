use std::ffi::OsString;

use super::{ActionRegistryMode, Command, OutputFormat, ParseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionListParseState {
    output: OutputFormat,
    registry: ActionRegistryMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionInspectParseState {
    output: OutputFormat,
    registry: ActionRegistryMode,
}

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
    let (raw_id, rest) = args
        .split_first()
        .ok_or(ParseError::MissingArgument("action_id"))?;
    let id = raw_id
        .to_str()
        .ok_or_else(|| ParseError::InvalidActionId(format!("{raw_id:?}")))?
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidActionId(raw_id.to_string_lossy().into_owned()))?;
    let parsed = parse_action_inspect_args(
        rest,
        ActionInspectParseState {
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        },
    )?;
    Ok(Command::ActionInspect {
        action_id: id,
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
            Some("--json") => parse_action_inspect_args(
                rest,
                ActionInspectParseState {
                    output: OutputFormat::Json,
                    ..state
                },
            ),
            Some("--jsonl") => parse_action_inspect_args(
                rest,
                ActionInspectParseState {
                    output: OutputFormat::Jsonl,
                    ..state
                },
            ),
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

fn parse_action_list_args(
    args: &[OsString],
    state: ActionListParseState,
) -> Result<ActionListParseState, ParseError> {
    match args.split_first() {
        None => Ok(state),
        Some((raw, rest)) => match raw.to_str() {
            Some("--json") => parse_action_list_args(
                rest,
                ActionListParseState {
                    output: OutputFormat::Json,
                    ..state
                },
            ),
            Some("--jsonl") => parse_action_list_args(
                rest,
                ActionListParseState {
                    output: OutputFormat::Jsonl,
                    ..state
                },
            ),
            Some("--registry") => parse_action_registry_arg(rest, state),
            Some(flag) if flag.starts_with("--") => {
                Err(ParseError::UnknownActionListFlag(flag.into()))
            }
            Some(arg) => Err(ParseError::UnexpectedActionListArgument(arg.into())),
            None => Err(ParseError::UnexpectedActionListArgument(format!("{raw:?}"))),
        },
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

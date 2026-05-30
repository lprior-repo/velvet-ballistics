use std::ffi::OsString;

use super::{ActionRegistryMode, Command, OutputFormat, ParseError};

// DESIGN NOTE: These structs are structurally identical (same fields, same derives).
// However, they are kept separate for semantic type safety - one represents
// the parsing state for `action list` subcommand, the other for `action inspect`.
// This prevents mixing up context in match arms and provides compile-time
// guarantees that the correct state type is used for each subcommand.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn os(val: &str) -> OsString {
        OsString::from(val)
    }

    #[test]
    fn parse_action_list_returns_ok() {
        let args = [os("vb"), os("action"), os("list")];
        let result = parse_action(&args).unwrap();
        match result {
            Command::ActionList { .. } => {}
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_action_list_with_json_flag() {
        let args = [os("vb"), os("action"), os("list"), os("--json")];
        let result = parse_action(&args);
        assert!(
            matches!(result.unwrap_err(), ParseError::UnknownActionListFlag(ref flag) if flag == "--json"),
            "expected UnknownActionListFlag, got {result:?}"
        );
    }

    #[test]
    fn parse_action_list_with_registry() {
        let args = [os("vb"), os("action"), os("list"), os("--registry"), os("empty")];
        let result = parse_action(&args).unwrap();
        match result {
            Command::ActionList { registry, .. } => assert_eq!(registry, ActionRegistryMode::Empty),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_action_inspect_returns_ok() {
        let args = [os("vb"), os("action"), os("inspect"), os("42")];
        let result = parse_action(&args).unwrap();
        match result {
            Command::ActionInspect { action_id, .. } => assert_eq!(action_id, 42),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_action_inspect_rejects_non_numeric_id() {
        let args = [os("vb"), os("action"), os("inspect"), os("abc")];
        let result = parse_action(&args);
        assert!(matches!(result.unwrap_err(), ParseError::InvalidActionId(_)));
    }

    #[test]
    fn parse_action_inspect_missing_id() {
        let args = [os("vb"), os("action"), os("inspect")];
        let result = parse_action(&args);
        assert!(matches!(result.unwrap_err(), ParseError::MissingArgument(_)));
    }

    #[test]
    fn parse_action_rejects_unknown_subcommand() {
        let args = [os("vb"), os("action"), os("delete")];
        let result = parse_action(&args);
        assert!(matches!(result.unwrap_err(), ParseError::UnknownActionCommand(_)));
    }

    #[test]
    fn parse_action_registry_mode_registered() {
        assert_eq!(parse_action_registry_mode("registered").unwrap(), ActionRegistryMode::Registered);
    }

    #[test]
    fn parse_action_registry_mode_empty() {
        assert_eq!(parse_action_registry_mode("empty").unwrap(), ActionRegistryMode::Empty);
    }

    #[test]
    fn parse_action_registry_mode_uninitialized() {
        assert_eq!(parse_action_registry_mode("uninitialized").unwrap(), ActionRegistryMode::Uninitialized);
    }

    #[test]
    fn parse_action_registry_mode_rejects_unknown() {
        assert!(matches!(
            parse_action_registry_mode("bad").unwrap_err(),
            ParseError::UnknownActionRegistry(_)
        ));
    }
}

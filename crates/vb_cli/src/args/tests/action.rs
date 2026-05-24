use super::args;
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, parse_args};

#[test]
fn parse_action_list_defaults_to_text_and_registered() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list"]));
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Text);
        assert_eq!(registry, ActionRegistryMode::Registered);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

// NOTE: --json and --jsonl CLI flags were removed when OutputFormat::Json
// and OutputFormat::Jsonl variants were removed. The corresponding tests
// have been deleted.

#[test]
fn parse_action_list_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
        assert_eq!(registry, ActionRegistryMode::Registered);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_list_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--emit",
        "postcard",
    ]));
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Postcard);
        assert_eq!(registry, ActionRegistryMode::Registered);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_list_accepts_registry_empty() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
        "empty",
    ]));
    if let Ok(Command::ActionList { registry, .. }) = parsed {
        assert_eq!(registry, ActionRegistryMode::Empty);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_list_accepts_registry_uninitialized() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
        "uninitialized",
    ]));
    if let Ok(Command::ActionList { registry, .. }) = parsed {
        assert_eq!(registry, ActionRegistryMode::Uninitialized);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_list_rejects_unknown_registry_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
        "corrupted",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::UnknownActionRegistry(ref v)) if v == "corrupted"),
        "expected UnknownActionRegistry(corrupted), got {parsed:?}"
    );
}

#[test]
fn parse_action_list_rejects_missing_registry_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "list",
        "--registry",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingActionRegistryValue)
    ));
}

#[test]
fn parse_action_list_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list", "--bogus"]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownActionListFlag(ref f)) if f == "--bogus"
    ));
}

#[test]
fn parse_action_list_rejects_positional_argument() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "list", "extra"]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnexpectedActionListArgument(ref a)) if a == "extra"
    ));
}

#[test]
fn parse_action_inspect_accepts_valid_id_and_defaults() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "inspect", "42"]));
    if let Ok(Command::ActionInspect {
        action_id,
        output,
        registry,
    }) = parsed
    {
        assert_eq!(action_id, 42);
        assert_eq!(output, OutputFormat::Text);
        assert_eq!(registry, ActionRegistryMode::Registered);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_inspect_accepts_registry_and_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "inspect",
        "7",
        "--registry",
        "empty",
        "--emit",
        "postcard",
    ]));
    if let Ok(Command::ActionInspect {
        action_id,
        output,
        registry,
    }) = parsed
    {
        assert_eq!(action_id, 7);
        assert_eq!(output, OutputFormat::Postcard);
        assert_eq!(registry, ActionRegistryMode::Empty);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_inspect_rejects_missing_action_id() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "inspect"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("action_id"))
    ));
}

#[test]
fn parse_action_inspect_rejects_non_numeric_action_id() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "inspect", "abc"]));
    assert!(matches!(parsed, Err(ParseError::InvalidActionId(_))));
}

#[test]
fn parse_action_inspect_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "action",
        "inspect",
        "1",
        "--bogus",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownActionInspectFlag(ref f)) if f == "--bogus"
    ));
}

#[test]
fn parse_action_rejects_unknown_subcommand() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action", "delete"]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownActionCommand(ref c)) if c == "delete"
    ));
}

#[test]
fn parse_action_rejects_missing_subcommand() {
    let parsed = parse_args(&args(&["velvet-ballastics", "action"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("action subcommand"))
    ));
}

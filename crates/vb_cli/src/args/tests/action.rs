use super::args;
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, parse_args};

#[test]
fn parse_action_list_defaults_to_text_and_registered() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list"]));
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Text);
        assert_eq!(registry, ActionRegistryMode::Registered);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_list_legacy_json_flag_is_rejected() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list", "--json"]));
    assert!(matches!(parsed, Err(ParseError::UnknownActionListFlag(flag)) if flag == "--json"));
}

#[test]
fn parse_action_list_legacy_jsonl_flag_is_rejected() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list", "--jsonl"]));
    assert!(matches!(parsed, Err(ParseError::UnknownActionListFlag(flag)) if flag == "--jsonl"));
}

#[test]
fn parse_action_list_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
        "velvet-ballistics",
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
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list", "--bogus"]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownActionListFlag(ref f)) if f == "--bogus"
    ));
}

#[test]
fn parse_action_list_rejects_positional_argument() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "list", "extra"]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnexpectedActionListArgument(ref a)) if a == "extra"
    ));
}

#[test]
fn parse_action_inspect_accepts_valid_name_and_defaults() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "inspect",
        "send_email",
    ]));
    if let Ok(Command::ActionInspect {
        action_name,
        output,
        registry,
    }) = parsed
    {
        assert_eq!(action_name, "send_email");
        assert_eq!(output, OutputFormat::Text);
        assert_eq!(registry, ActionRegistryMode::Registered);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_inspect_accepts_registry_and_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "inspect",
        "send_email",
        "--registry",
        "empty",
        "--emit",
        "postcard",
    ]));
    if let Ok(Command::ActionInspect {
        action_name,
        output,
        registry,
    }) = parsed
    {
        assert_eq!(action_name, "send_email");
        assert_eq!(output, OutputFormat::Postcard);
        assert_eq!(registry, ActionRegistryMode::Empty);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_action_inspect_rejects_missing_action_name() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "inspect"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("action_name"))
    ));
}

#[test]
fn parse_action_inspect_rejects_whitespace_action_name() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "inspect",
        "bad name",
    ]));
    assert!(matches!(parsed, Err(ParseError::InvalidActionName(_))));
}

#[test]
fn parse_action_inspect_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
    let parsed = parse_args(&args(&["velvet-ballistics", "action", "delete"]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownActionCommand(ref c)) if c == "delete"
    ));
}

#[test]
fn parse_action_rejects_missing_subcommand() {
    let parsed = parse_args(&args(&["velvet-ballistics", "action"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("action subcommand"))
    ));
}

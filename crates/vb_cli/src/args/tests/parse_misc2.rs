use super::*;
use crate::args::run_ops::CANCEL_REASON_MAX_CHARS;
use crate::args::shared::{named_flag, parse_output_format, positional_str};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

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
    match parsed {
        Ok(Command::Verify {
            workflow,
            profile,
            output,
            legacy_json,
        }) => {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(profile, VerifyProfile::Standard);
            assert_eq!(output, OutputFormat::Text);
            assert_eq!(legacy_json, LegacyJsonOutput::Disabled);
        }
        other => panic!("expected Command::Verify, got {other:?}"),
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
    match parsed {
        Ok(Command::Verify { profile, .. }) => {
            assert_eq!(profile, VerifyProfile::Quick);
        }
        other => panic!("expected Command::Verify, got {other:?}"),
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
    match parsed {
        Ok(Command::Verify {
            profile, output, ..
        }) => {
            assert_eq!(profile, VerifyProfile::Full);
            assert_eq!(output, OutputFormat::Yaml);
        }
        other => panic!("expected Command::Verify, got {other:?}"),
    }
}

#[test]
fn parse_verify_legacy_json_flag_requests_machine_json() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--json",
    ]));
    match parsed {
        Ok(Command::Verify {
            output,
            legacy_json,
            ..
        }) => {
            assert_eq!(output, OutputFormat::Text);
            assert_eq!(legacy_json, LegacyJsonOutput::Json);
        }
        other => panic!("expected Command::Verify, got {other:?}"),
    }
}

#[test]
fn parse_verify_legacy_jsonl_flag_requests_machine_jsonl() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--jsonl",
    ]));
    match parsed {
        Ok(Command::Verify {
            output,
            legacy_json,
            ..
        }) => {
            assert_eq!(output, OutputFormat::Text);
            assert_eq!(legacy_json, LegacyJsonOutput::Jsonl);
        }
        other => panic!("expected Command::Verify, got {other:?}"),
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
    match parsed {
        Ok(Command::Graph { workflow, output }) => {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected Command::Graph, got {other:?}"),
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
    match parsed {
        Ok(Command::Graph { output, .. }) => {
            assert_eq!(output, OutputFormat::Yaml);
        }
        other => panic!("expected Command::Graph, got {other:?}"),
    }
}

#[test]
fn parse_simulate_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballistics", "simulate", "workflow.yaml"]));
    match parsed {
        Ok(Command::Simulate { workflow, output }) => {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected Command::Simulate, got {other:?}"),
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
    match parsed {
        Ok(Command::Simulate { output, .. }) => {
            assert_eq!(output, OutputFormat::Yaml);
        }
        other => panic!("expected Command::Simulate, got {other:?}"),
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
    match parsed {
        Ok(Command::Simulate { output, .. }) => {
            assert_eq!(output, OutputFormat::Postcard);
        }
        other => panic!("expected Command::Simulate, got {other:?}"),
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
    match parsed {
        Ok(Command::ActionList { output, registry }) => {
            assert_eq!(output, OutputFormat::Yaml);
            assert_eq!(registry, ActionRegistryMode::Registered);
        }
        other => panic!("expected Command::ActionList, got {other:?}"),
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
    match parsed {
        Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) => {
            assert_eq!(run_id, "42");
            assert_eq!(db, PathBuf::from("journal-db"));
            assert_eq!(reason, None);
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected Command::Cancel, got {other:?}"),
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
    match parsed {
        Ok(Command::Cancel { reason, .. }) => {
            assert_eq!(reason, Some("user request".to_string()));
        }
        other => panic!("expected Command::Cancel, got {other:?}"),
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
    match parsed {
        Ok(Command::Cancel { output, .. }) => {
            assert_eq!(output, OutputFormat::Yaml);
        }
        other => panic!("expected Command::Cancel, got {other:?}"),
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
fn parse_cancel_rejects_reason_longer_than_256_chars() {
    let long_reason = "a".repeat(CANCEL_REASON_MAX_CHARS.saturating_add(1));
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
fn parse_cancel_accepts_reason_exactly_256_chars() {
    let reason = "a".repeat(CANCEL_REASON_MAX_CHARS);
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

// ---------------------------------------------------------------------------
// Shared-module edge-case tests (parse_output_format, positional_str, named_flag)
// Recovered from stash@{6} (feat(vb-chaah): add AI PR contract gate with self-tests)
// ---------------------------------------------------------------------------

#[test]
fn parse_args_empty_vector_returns_no_command() {
    let parsed = parse_args(&[]);
    assert!(
        matches!(parsed, Err(ParseError::NoCommand)),
        "expected NoCommand, got {parsed:?}"
    );
}

#[test]
fn parse_args_binary_only_returns_no_command() {
    let parsed = parse_args(&args(&["velvet-ballistics"]));
    assert!(
        matches!(parsed, Err(ParseError::NoCommand)),
        "expected NoCommand, got {parsed:?}"
    );
}

#[test]
fn parse_args_empty_vec_returns_no_command_not_unknown() {
    let parsed = parse_args(&[]);
    match parsed {
        Err(ParseError::NoCommand) => {}
        #[allow(clippy::assertions_on_constants)]
        other => panic!("placeholder"),
    }
}

#[test]
fn parse_output_format_empty_vec_returns_text_default() {
    let format = parse_output_format(&[]);
    assert_eq!(format, OutputFormat::Text);
}

#[test]
fn parse_output_format_binary_only_returns_text_default() {
    let format = parse_output_format(&args(&["velvet-ballistics"]));
    assert_eq!(format, OutputFormat::Text);
}

#[test]
fn parse_output_format_emit_yaml_returns_yaml() {
    let format = parse_output_format(&args(&["--emit", "yaml"]));
    assert_eq!(format, OutputFormat::Yaml);
}

#[test]
fn parse_output_format_emit_postcard_returns_postcard() {
    let format = parse_output_format(&args(&["--emit", "postcard"]));
    assert_eq!(format, OutputFormat::Postcard);
}

#[test]
fn parse_output_format_emit_text_returns_text() {
    let format = parse_output_format(&args(&["--emit", "text"]));
    assert_eq!(format, OutputFormat::Text);
}

#[test]
fn parse_output_format_emit_unknown_value_returns_text() {
    let format = parse_output_format(&args(&["--emit", "cson"]));
    assert_eq!(format, OutputFormat::Text);
}

#[test]
fn parse_output_format_json_flag_returns_text() {
    let format = parse_output_format(&args(&["--json"]));
    assert_eq!(format, OutputFormat::Text);
}

#[test]
fn parse_output_format_jsonl_flag_returns_text() {
    let format = parse_output_format(&args(&["--jsonl"]));
    assert_eq!(format, OutputFormat::Text);
}

#[test]
fn positional_str_beyond_bounds_returns_missing_argument() {
    let result = positional_str(&args(&["one", "two"]), 5, "target");
    assert!(
        matches!(result, Err(ParseError::MissingArgument("target"))),
        "expected MissingArgument(\"target\"), got {result:?}"
    );
}

#[test]
fn positional_str_exact_last_index_succeeds() {
    let val = positional_str(&args(&["one", "two"]), 1, "arg")
        .expect("positional_str on 'one two' at last index must succeed");
    assert_eq!(val, "two");
}

#[test]
fn positional_str_one_past_end_returns_missing_argument() {
    let result = positional_str(&args(&["one"]), 1, "target");
    assert!(
        matches!(result, Err(ParseError::MissingArgument("target"))),
        "expected MissingArgument(\"target\"), got {result:?}"
    );
}

#[test]
#[cfg(unix)]
fn positional_str_non_utf8_returns_missing_argument() {
    use std::os::unix::ffi::OsStrExt;
    let non_utf8: Vec<OsString> = vec![
        OsString::from("valid"),
        std::ffi::OsStr::from_bytes(b"\xff").to_os_string(),
    ];
    let result = positional_str(&non_utf8, 1, "target");
    assert!(
        matches!(result, Err(ParseError::MissingArgument("target"))),
        "expected MissingArgument(\"target\"), got {result:?}"
    );
}

#[test]
fn named_flag_at_last_position_no_value_returns_none() {
    let result = named_flag(&args(&["--emit"]), "--emit");
    assert!(
        result.is_none(),
        "expected None when flag is the last argument, got {result:?}"
    );
}

#[test]
fn named_flag_with_value_returns_some() {
    let result = named_flag(&args(&["--emit", "yaml"]), "--emit");
    assert_eq!(result, Some("yaml".to_string()));
}

#[test]
fn named_flag_not_present_returns_none() {
    let result = named_flag(&args(&["one", "two"]), "--missing");
    assert!(result.is_none());
}

#[test]
fn named_flag_value_starting_with_double_dash_returns_value() {
    let result = named_flag(&args(&["--db", "--some-other-flag"]), "--db");
    assert_eq!(result, Some("--some-other-flag".to_string()));
}

#[test]
fn named_flag_switch_at_end_returns_none() {
    let result = named_flag(&args(&["run", "w.yaml", "--dry-run"]), "--dry-run");
    assert!(result.is_none());
}

#[test]
fn named_flag_first_occurrence_wins() {
    let result = named_flag(&args(&["--emit", "yaml", "--emit", "postcard"]), "--emit");
    assert_eq!(result, Some("yaml".to_string()));
}

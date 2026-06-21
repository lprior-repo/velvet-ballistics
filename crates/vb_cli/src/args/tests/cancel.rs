use std::path::PathBuf;

use super::args;
use crate::args::run_ops::CANCEL_REASON_MAX_CHARS;
use crate::args::{Command, OutputFormat, ParseError, parse_args};

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
fn parse_cancel_legacy_json_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--json",
    ]));
    match parsed {
        Ok(Command::Cancel { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Cancel, got {other:?}"),
    }
}

#[test]
fn parse_cancel_legacy_jsonl_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--jsonl",
    ]));
    match parsed {
        Ok(Command::Cancel { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Cancel, got {other:?}"),
    }
}

#[test]
fn parse_cancel_accepts_emit_yaml() {
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
fn parse_cancel_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--emit",
        "postcard",
    ]));
    match parsed {
        Ok(Command::Cancel { output, .. }) => {

            assert_eq!(output, OutputFormat::Postcard);

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
fn parse_cancel_rejects_missing_run_id() {
    let parsed = parse_args(&args(&["velvet-ballistics", "cancel"]));
    assert!(matches!(parsed, Err(ParseError::MissingArgument("run_id"))));
}

#[test]
fn parse_cancel_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--bogus",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownFlag {
            command: "cancel",
            ..
        })
    ));
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
    match parsed {
        Ok(Command::Cancel { .. }) => {


        }
        other => panic!("expected Command::Cancel, got {other:?}"),
    }
}

#[test]
fn parse_cancel_accepts_multibyte_reason_at_char_limit() {
    let reason = "é".repeat(CANCEL_REASON_MAX_CHARS);
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        &reason,
    ]));
    match parsed {
        Ok(Command::Cancel { reason: parsed_reason, .. }) => {

            assert_eq!(parsed_reason, Some(reason));

        }
        other => panic!("expected Command::Cancel, got {other:?}"),
    }
}

#[test]
fn parse_cancel_rejects_multibyte_reason_over_char_limit() {
    let reason = "é".repeat(CANCEL_REASON_MAX_CHARS.saturating_add(1));
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        &reason,
    ]));
    assert!(matches!(parsed, Err(ParseError::ReasonTooLong)));
}

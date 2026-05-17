use std::path::PathBuf;

use super::args;
use crate::args::{Command, OutputFormat, ParseError, parse_args};

#[test]
fn parse_cancel_accepts_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
        "velvet-ballastics",
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
fn parse_cancel_accepts_json_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_cancel_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballastics", "cancel", "42"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_cancel_rejects_reason_longer_than_256_bytes() {
    let long_reason = "a".repeat(257);
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
        "velvet-ballastics",
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

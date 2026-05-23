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
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Cancel { reason, .. }) = parsed {
        assert_eq!(reason, Some("user request".to_string()));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_cancel_accepts_jsonl_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--jsonl",
    ]));
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_cancel_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_cancel_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--emit",
        "postcard",
    ]));
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Postcard);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
fn parse_cancel_rejects_missing_run_id() {
    let parsed = parse_args(&args(&["velvet-ballastics", "cancel"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("run_id"))
    ));
}

#[test]
fn parse_cancel_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--bogus",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownFlag { command: "cancel", .. })
    ));
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
    if let Ok(Command::Cancel { .. }) = parsed {
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

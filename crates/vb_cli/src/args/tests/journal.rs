use std::path::PathBuf;

use super::args;
use crate::args::{Command, OutputFormat, ParseError, parse_args};

#[test]
fn parse_inspect_includes_output_format() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Inspect { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Inspect { run_id, db, output }) = parsed {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_answer_rejects_invalid_step_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "answer",
        "run-1",
        "--step",
        "not-a-step",
        "--value-file",
        "value.bin",
        "--db",
        "test-db",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidStep(ref s)) if s == "not-a-step"),
        "expected InvalidStep(not-a-step), got {parsed:?}"
    );
}

#[test]
fn parse_diff_requires_both_run_ids_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "diff",
        "1",
        "2",
        "--db",
        "test-db",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff {
        run_a,
        run_b,
        db,
        output,
    }) = parsed
    {
        assert_eq!(run_a, "1");
        assert_eq!(run_b, "2");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_diff_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "diff",
        "10",
        "20",
        "--db",
        "test-db",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_diff_requires_db_flag() {
    let parsed = parse_args(&args(&["velvet-ballastics", "diff", "1", "2"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_doctor_without_db_is_stateless_text_mode() {
    let parsed = parse_args(&args(&["velvet-ballastics", "doctor"]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_doctor_accepts_optional_db_and_json_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "doctor",
        "--db",
        "journal-db",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, Some(PathBuf::from("journal-db")));
        assert_eq!(output, OutputFormat::Json);
    }
}

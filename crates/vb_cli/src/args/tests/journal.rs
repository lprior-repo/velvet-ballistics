use std::path::PathBuf;

use super::args;
use crate::args::{Command, OutputFormat, ParseError, parse_args};
use crate::commands_journal::TraceStatus;

#[test]
fn parse_inspect_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "inspect",
        "42",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Inspect { run_id, db, output }) = parsed {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_inspect_accepts_json_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--json",
    ]));
    if let Ok(Command::Inspect { run_id, db, output }) = parsed {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Json);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_inspect_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Inspect { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_inspect_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballastics", "inspect", "42"]));
    assert!(matches!(parsed, Err(ParseError::MissingArgument("--db"))));
}

#[test]
fn parse_events_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "events",
        "run-1",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Events {
        run_id,
        db,
        output,
        status,
        limit,
    }) = parsed
    {
        assert_eq!(run_id, "run-1");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
        assert_eq!(status, None);
        assert_eq!(limit, None);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_events_accepts_status_filter() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "events",
        "run-1",
        "--db",
        "test-db",
        "--status",
        "completed",
    ]));
    if let Ok(Command::Events { status, .. }) = parsed {
        assert!(status.is_some());
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_events_accepts_limit_filter() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "events",
        "run-1",
        "--db",
        "test-db",
        "--limit",
        "100",
    ]));
    if let Ok(Command::Events { limit, .. }) = parsed {
        assert_eq!(limit, Some(100));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_events_rejects_unknown_status() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "events",
        "run-1",
        "--db",
        "test-db",
        "--status",
        "xyz",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownEventStatus(ref s)) if s == "xyz"
    ));
}

#[test]
fn parse_replay_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "replay",
        "99",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Replay { run_id, db, output }) = parsed {
        assert_eq!(run_id, "99");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_trace_defaults_to_no_filters() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "trace",
        "7",
        "--db",
        "journal-db",
    ]));
    if let Ok(Command::Trace { filters, .. }) = parsed {
        assert_eq!(filters.step, None);
        assert_eq!(filters.action, None);
        assert_eq!(filters.status, None);
        assert_eq!(filters.since_seq, None);
        assert_eq!(filters.until_seq, None);
        assert_eq!(filters.limit, None);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_trace_accepts_all_filters() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--step",
        "4",
        "--action",
        "9",
        "--status",
        "active",
        "--since-seq",
        "10",
        "--until-seq",
        "20",
        "--limit",
        "3",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Trace {
        output, filters, ..
    }) = parsed
    {
        assert_eq!(output, OutputFormat::Yaml);
        assert_eq!(filters.step, Some(4));
        assert_eq!(filters.action, Some(9));
        assert_eq!(filters.status, Some(TraceStatus::Active));
        assert_eq!(filters.since_seq, Some(10));
        assert_eq!(filters.until_seq, Some(20));
        assert_eq!(filters.limit, Some(3));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_trace_rejects_invalid_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--step",
        "not-a-step",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "--step must be a valid u16"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_trace_rejects_invalid_since_seq() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--since-seq",
        "not-a-seq",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "--since-seq must be a valid u64"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_trace_rejects_unknown_trace_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--severity",
        "error",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "unknown trace flag: --severity"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_retry_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "retry",
        "123",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Retry { run_id, db, output }) = parsed {
        assert_eq!(run_id, "123");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_resume_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "resume",
        "456",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Resume { run_id, db, output }) = parsed {
        assert_eq!(run_id, "456");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_incident_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "incident",
        "7",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Incident { run_id, db, output }) = parsed {
        assert_eq!(run_id, "7");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
fn parse_answer_accepts_valid_step_and_input() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "answer",
        "run-1",
        "--step",
        "3",
        "--value-file",
        "value.bin",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Answer {
        run_id,
        step,
        value_file,
        db,
        output,
    }) = parsed
    {
        assert_eq!(run_id, "run-1");
        assert_eq!(step, 3);
        assert_eq!(value_file, PathBuf::from("value.bin"));
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
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
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_diff_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "diff",
        "10",
        "20",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Diff { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_diff_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballastics", "diff", "1", "2"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_diff_rejects_missing_run_id() {
    let parsed = parse_args(&args(&["velvet-ballastics", "diff", "1"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument(_))),
        "expected MissingArgument, got {parsed:?}"
    );
}

#[test]
fn parse_doctor_without_db_is_stateless_text_mode() {
    let parsed = parse_args(&args(&["velvet-ballastics", "doctor"]));
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_doctor_accepts_optional_db_and_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "doctor",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, Some(PathBuf::from("journal-db")));
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

use std::path::PathBuf;

use super::args;
use crate::args::{Command, DiffMode, EventStatus, OutputFormat, ParseError, parse_args};
use crate::commands_journal::TraceStatus;

#[test]
fn parse_inspect_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Inspect command, got {parsed:?}");
    }
}

#[test]
fn parse_inspect_legacy_json_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--json",
    ]));
    if let Ok(Command::Inspect { run_id, db, output }) = parsed {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        panic!("expected Inspect command, got {parsed:?}");
    }
}

#[test]
fn parse_inspect_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Inspect command, got {parsed:?}");
    }
}

#[test]
fn parse_inspect_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballistics", "inspect", "42"]));
    assert_eq!(parsed, Err(ParseError::MissingArgument("--db")));
}

#[test]
fn parse_events_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Events command, got {parsed:?}");
    }
}

#[test]
fn parse_events_accepts_status_filter() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "events",
        "run-1",
        "--db",
        "test-db",
        "--status",
        "completed",
    ]));
    if let Ok(Command::Events { status, .. }) = parsed {
        assert_eq!(status, Some(EventStatus::Completed));
    } else {
        panic!("expected Events command, got {parsed:?}");
    }
}

#[test]
fn parse_events_accepts_limit_filter() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Events command, got {parsed:?}");
    }
}

#[test]
fn parse_events_rejects_unknown_status() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "events",
        "run-1",
        "--db",
        "test-db",
        "--status",
        "xyz",
    ]));
    assert_eq!(
        parsed,
        Err(ParseError::UnknownEventStatus(String::from("xyz")))
    );
}

#[test]
fn parse_replay_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Replay command, got {parsed:?}");
    }
}

#[test]
fn parse_trace_defaults_to_no_filters() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Trace command, got {parsed:?}");
    }
}

#[test]
fn parse_trace_accepts_all_filters() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Trace command, got {parsed:?}");
    }
}

#[test]
fn parse_trace_rejects_invalid_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--step",
        "not-a-step",
    ]));
    assert_eq!(
        parsed,
        Err(ParseError::InvalidTraceArgument(String::from(
            "--step must be a valid u16"
        )))
    );
}

#[test]
fn parse_trace_rejects_invalid_since_seq() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--since-seq",
        "not-a-seq",
    ]));
    assert_eq!(
        parsed,
        Err(ParseError::InvalidTraceArgument(String::from(
            "--since-seq must be a valid u64"
        )))
    );
}

#[test]
fn parse_trace_rejects_unknown_trace_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--severity",
        "error",
    ]));
    assert_eq!(
        parsed,
        Err(ParseError::InvalidTraceArgument(String::from(
            "unknown trace flag: --severity"
        )))
    );
}

#[test]
fn parse_retry_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "retry",
        "123",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Retry {
        run_id,
        step,
        db,
        output,
    }) = parsed
    {
        assert_eq!(run_id, "123");
        assert_eq!(step, None);
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        panic!("expected Retry command, got {parsed:?}");
    }
}

#[test]
fn parse_resume_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Resume command, got {parsed:?}");
    }
}

#[test]
fn parse_incident_requires_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Incident command, got {parsed:?}");
    }
}

#[test]
fn parse_answer_rejects_invalid_slot_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "run-1",
        "--slot",
        "not-a-slot",
        "--value",
        "value.bin",
        "--db",
        "test-db",
    ]));
    assert_eq!(
        parsed,
        Err(ParseError::InvalidSlot(String::from("not-a-slot")))
    );
}

#[test]
fn parse_answer_accepts_valid_slot_and_input() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "run-1",
        "--slot",
        "3",
        "--value",
        "value.bin",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Answer {
        run_id,
        slot,
        value,
        db,
        output,
    }) = parsed
    {
        assert_eq!(run_id, "run-1");
        assert_eq!(slot, 3);
        assert_eq!(value, PathBuf::from("value.bin"));
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        panic!("expected Answer command, got {parsed:?}");
    }
}

#[test]
fn parse_diff_requires_both_run_ids_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "1",
        "2",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::Diff {
        diff_mode: DiffMode::RunAgainst { run_a, run_b, db },
        output,
    }) = parsed
    {
        assert_eq!(run_a, String::from("1"));
        assert_eq!(run_b, String::from("2"));
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        panic!("expected run Diff command, got {parsed:?}");
    }
}

#[test]
fn parse_diff_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Diff command, got {parsed:?}");
    }
}

#[test]
fn parse_diff_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballistics", "diff", "1", "2"]));
    assert_eq!(parsed, Err(ParseError::MissingArgument("--db")));
}

#[test]
fn parse_diff_allows_workflow_against_workflow_without_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "current.yaml",
        "--against",
        "previous.yaml",
    ]));
    match parsed {
        Ok(Command::Diff {
            diff_mode: DiffMode::WorkflowAgainst { workflow, against },
            output,
        }) => {
            assert_eq!(workflow, PathBuf::from("current.yaml"));
            assert_eq!(against, PathBuf::from("previous.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected workflow Diff command without db, got {other:?}"),
    }
}

#[test]
fn parse_diff_rejects_workflow_against_with_db_hidden_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "current.yaml",
        "--against",
        "123",
        "--db",
        "test-db",
    ]));
    assert_eq!(
        parsed,
        Err(ParseError::InvalidArgument(String::from(
            "diff accepts either workflow --against <old-workflow> without --db, or two run IDs plus --db"
        )))
    );
}

#[test]
fn parse_diff_rejects_missing_run_id() {
    let parsed = parse_args(&args(&["velvet-ballistics", "diff", "1"]));
    assert_eq!(parsed, Err(ParseError::MissingArgument("run_b")));
}

#[test]
fn parse_doctor_without_db_is_stateless_text_mode() {
    let parsed = parse_args(&args(&["velvet-ballistics", "doctor"]));
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    } else {
        panic!("expected Doctor command, got {parsed:?}");
    }
}

#[test]
fn parse_doctor_accepts_optional_db_and_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
        panic!("expected Doctor command, got {parsed:?}");
    }
}

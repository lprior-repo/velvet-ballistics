use super::*;

#[test]
fn step_target_holds_step_id_and_path() {
    let target = StepTarget {
        step_id: 5,
        step_input: PathBuf::from("data.bin"),
    };
    assert_eq!(target.step_id, 5);
    assert_eq!(target.step_input, PathBuf::from("data.bin"));
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

    match parsed {
        Ok(Command::Trace {
            run_id,
            db,
            output,
            filters,
        }) => {
            assert_eq!(run_id, "7");
            assert_eq!(db, PathBuf::from("journal-db"));
            assert_eq!(output, OutputFormat::Text);
            assert_eq!(filters.step, None);
            assert_eq!(filters.action, None);
            assert_eq!(filters.status, None);
            assert_eq!(filters.since_seq, None);
            assert_eq!(filters.until_seq, None);
            assert_eq!(filters.limit, None);
        }
        other => panic!("expected Command::Trace, got {other:?}"),
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

    match parsed {
        Ok(Command::Trace {
            output, filters, ..
        }) => {
            assert_eq!(output, OutputFormat::Yaml);
            assert_eq!(filters.step, Some(4));
            assert_eq!(filters.action, Some(9));
            assert_eq!(filters.status, Some(TraceStatus::Active));
            assert_eq!(filters.since_seq, Some(10));
            assert_eq!(filters.until_seq, Some(20));
            assert_eq!(filters.limit, Some(3));
        }
        other => panic!("expected Command::Trace, got {other:?}"),
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

    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "--step must be a valid u16"),
        "unexpected parse result: {parsed:?}"
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

    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "--since-seq must be a valid u64"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_trace_rejects_missing_until_seq_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--until-seq",
        "--emit",
        "yaml",
    ]));

    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--until-seq"))
    ));
}

#[test]
fn parse_trace_rejects_missing_limit_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--limit",
        "--emit",
        "yaml",
    ]));

    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--limit"))
    ));
}

#[test]
fn parse_trace_rejects_unknown_filter_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
fn parse_status_accepts_no_runtime_defaults() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--emit", "yaml"]));
    match parsed {
        Ok(Command::Status { options, output }) => {
            assert_eq!(options.active_runs, None);
            assert_eq!(options.queue_depth, None);
            assert_eq!(options.trace_dropped, None);
            assert_eq!(output, OutputFormat::Yaml);
        }
        other => panic!("expected Command::Status, got {other:?}"),
    }
}

#[test]
fn parse_status_accepts_diagnostic_counters() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "5",
        "--queue-depth",
        "3",
        "--trace-dropped",
        "0",
    ]));
    match parsed {
        Ok(Command::Status { options, output }) => {
            assert_eq!(options.active_runs, Some(5));
            assert_eq!(options.queue_depth, Some(3));
            assert_eq!(options.trace_dropped, Some(0));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected Command::Status, got {other:?}"),
    }
}

#[test]
fn parse_status_rejects_invalid_numeric_argument() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--queue-depth",
        "many",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be a usize"),
        "expected InvalidStatusArgument(--queue-depth must be a usize), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_missing_queue_depth_value() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--queue-depth"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--queue-depth"))
    ));
}

#[test]
fn parse_status_rejects_missing_active_runs_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "--emit",
        "yaml",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--active-runs"))
    ));
}

#[test]
fn parse_status_rejects_missing_trace_dropped_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--trace-dropped",
        "--queue-depth",
        "1",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--trace-dropped"))
    ));
}

#[test]
fn parse_status_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--bogus"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unknown flag --bogus"),
        "expected InvalidStatusArgument(unknown flag --bogus), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_extra_positional_argument() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "extra"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unexpected positional argument extra"),
        "expected InvalidStatusArgument(unexpected positional argument extra), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_out_of_range_queue_depth() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--queue-depth",
        "1025",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be <= 1024"),
        "expected InvalidStatusArgument(--queue-depth must be <= 1024), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_out_of_range_active_runs() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "1025",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--active-runs must be <= 1024"),
        "expected InvalidStatusArgument(--active-runs must be <= 1024), got {parsed:?}"
    );
}

#[test]
fn parse_system_status_defaults_to_standard_none_text() {
    let parsed = parse_args(&args(&["velvet-ballistics", "system", "status"]));
    match parsed {
        Ok(Command::SystemStatus { options, output }) => {
            assert_eq!(options.profile, VerifyProfile::Standard);
            assert_eq!(options.server, DurabilityMode::None);
            assert!(!options.emit_yaml);
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected Command::SystemStatus, got {other:?}"),
    }
}

#[test]
fn parse_system_status_accepts_profile_server_and_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--profile",
        "full",
        "--server",
        "none",
        "--emit",
        "yaml",
    ]));
    match parsed {
        Ok(Command::SystemStatus { options, output }) => {
            assert_eq!(options.profile, VerifyProfile::Full);
            assert_eq!(options.server, DurabilityMode::None);
            assert!(options.emit_yaml);
            assert_eq!(output, OutputFormat::Yaml);
        }
        other => panic!("expected Command::SystemStatus, got {other:?}"),
    }
}

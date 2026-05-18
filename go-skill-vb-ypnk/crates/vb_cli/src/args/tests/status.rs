use super::args;
use crate::args::{Command, OutputFormat, ParseError, parse_args};

#[test]
fn parse_status_accepts_no_runtime_defaults() {
    let parsed = parse_args(&args(&["velvet-ballastics", "status", "--json"]));
    assert!(matches!(parsed, Ok(Command::Status { .. })));
    if let Ok(Command::Status { options, output }) = parsed {
        assert_eq!(options.active_runs, None);
        assert_eq!(options.queue_depth, None);
        assert_eq!(options.trace_dropped, None);
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_status_accepts_diagnostic_counters() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "status",
        "--active-runs",
        "5",
        "--queue-depth",
        "3",
        "--trace-dropped",
        "0",
    ]));
    assert!(matches!(parsed, Ok(Command::Status { .. })));
    if let Ok(Command::Status { options, output }) = parsed {
        assert_eq!(options.active_runs, Some(5));
        assert_eq!(options.queue_depth, Some(3));
        assert_eq!(options.trace_dropped, Some(0));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_status_rejects_invalid_numeric_argument() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
    let parsed = parse_args(&args(&["velvet-ballastics", "status", "--queue-depth"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--queue-depth"))
    ));
}

#[test]
fn parse_status_rejects_missing_active_runs_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "status",
        "--active-runs",
        "--json",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--active-runs"))
    ));
}

#[test]
fn parse_status_rejects_missing_trace_dropped_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
    let parsed = parse_args(&args(&["velvet-ballastics", "status", "--bogus"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unknown flag --bogus"),
        "expected InvalidStatusArgument(unknown flag --bogus), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_extra_positional_argument() {
    let parsed = parse_args(&args(&["velvet-ballastics", "status", "extra"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unexpected positional argument extra"),
        "expected InvalidStatusArgument(unexpected positional argument extra), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_out_of_range_queue_depth() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
        "velvet-ballastics",
        "status",
        "--active-runs",
        "1025",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--active-runs must be <= 1024"),
        "expected InvalidStatusArgument(--active-runs must be <= 1024), got {parsed:?}"
    );
}

use super::args;
use crate::args::{Command, DurabilityMode, OutputFormat, ParseError, VerifyProfile, parse_args};

#[test]
fn parse_system_status_defaults_to_standard_none_text() {
    let parsed = parse_args(&args(&["velvet-ballistics", "system", "status"]));
    if let Ok(Command::SystemStatus { options, output }) = parsed {
        assert_eq!(options.profile, VerifyProfile::Standard);
        assert_eq!(options.server, DurabilityMode::None);
        assert!(!options.emit_yaml);
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_system_status_accepts_full_profile_server_none_and_emit_yaml() {
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
    if let Ok(Command::SystemStatus { options, output }) = parsed {
        assert_eq!(options.profile, VerifyProfile::Full);
        assert_eq!(options.server, DurabilityMode::None);
        assert!(options.emit_yaml);
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

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
fn parse_system_status_rejects_strict_without_probe() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
        "strict",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref mode)) if mode == "strict"));
}

#[test]
fn parse_system_status_rejects_journaled_without_probe() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
        "journaled",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref mode)) if mode == "journaled"));
}

#[test]
fn parse_system_status_rejects_missing_profile_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--profile",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--profile"))
    ));
}

#[test]
fn parse_system_status_rejects_missing_server_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--server"))
    ));
}

#[test]
fn parse_system_rejects_unknown_subcommand() {
    let parsed = parse_args(&args(&["velvet-ballistics", "system", "bogus"]));
    assert!(matches!(
        parsed,
        Err(ParseError::InvalidSystemStatusArgument(ref s)) if s == "unknown system command bogus"
    ));
}

#[test]
fn parse_system_rejects_missing_subcommand() {
    let parsed = parse_args(&args(&["velvet-ballistics", "system"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("system subcommand"))
    ));
}

#[test]
fn parse_status_accepts_no_runtime_defaults() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--emit", "yaml"]));
    if let Ok(Command::Status { options, output }) = parsed {
        assert_eq!(options.active_runs, None);
        assert_eq!(options.queue_depth, None);
        assert_eq!(options.trace_dropped, None);
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Status { options, output }) = parsed {
        assert_eq!(options.active_runs, Some(5));
        assert_eq!(options.queue_depth, Some(3));
        assert_eq!(options.trace_dropped, Some(0));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_status_rejects_postcard_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--emit",
        "postcard",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "postcard emit is not supported for status"),
        "expected InvalidStatusArgument, got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_unknown_emit_mode() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--emit", "binary"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unknown emit mode binary"),
        "expected InvalidStatusArgument, got {parsed:?}"
    );
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
fn parse_status_accepts_queue_depth_at_maximum() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--queue-depth",
        "1024",
    ]));
    if let Ok(Command::Status { options, .. }) = parsed {
        assert_eq!(options.queue_depth, Some(1024));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_status_accepts_active_runs_at_maximum() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "1024",
    ]));
    if let Ok(Command::Status { options, .. }) = parsed {
        assert_eq!(options.active_runs, Some(1024));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_status_accepts_trace_dropped_with_large_u64() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--trace-dropped",
        "18446744073709551615",
    ]));
    if let Ok(Command::Status { options, .. }) = parsed {
        assert_eq!(options.trace_dropped, Some(18446744073709551615));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

use std::ffi::OsString;

use super::shared::parse_output_format;
use super::{Command, ParseError, StatusOptions};

pub(super) fn parse_status(args: &[OsString]) -> Result<Command, ParseError> {
    let tokens = args.get(2..).ok_or(ParseError::NoCommand)?;
    let options = parse_status_options(tokens, StatusOptions::default())?;
    let output = parse_output_format(args);
    Ok(Command::Status { options, output })
}

fn parse_status_options(
    args: &[OsString],
    options: StatusOptions,
) -> Result<StatusOptions, ParseError> {
    match args.split_first() {
        None => validate_status_options(options),
        Some((flag, rest)) => match flag.to_str() {
            Some("--json" | "--jsonl") => parse_status_options(rest, options),
            Some("--emit") => parse_status_emit(rest, options),
            Some("--active-runs") => {
                let parsed = parse_status_usize_value(rest, "--active-runs")?;
                parse_status_options(
                    parsed.remaining,
                    StatusOptions {
                        active_runs: Some(parsed.value),
                        ..options
                    },
                )
            }
            Some("--queue-depth") => {
                let parsed = parse_status_usize_value(rest, "--queue-depth")?;
                parse_status_options(
                    parsed.remaining,
                    StatusOptions {
                        queue_depth: Some(parsed.value),
                        ..options
                    },
                )
            }
            Some("--trace-dropped") => {
                let parsed = parse_status_u64_value(rest, "--trace-dropped")?;
                parse_status_options(
                    parsed.remaining,
                    StatusOptions {
                        trace_dropped: Some(parsed.value),
                        ..options
                    },
                )
            }
            Some(other) if other.starts_with('-') => Err(ParseError::InvalidStatusArgument(
                format!("unknown flag {other}"),
            )),
            Some(other) => Err(ParseError::InvalidStatusArgument(format!(
                "unexpected positional argument {other}"
            ))),
            None => Err(ParseError::InvalidStatusArgument(
                "argument is not valid UTF-8".into(),
            )),
        },
    }
}

fn parse_status_emit(
    args: &[OsString],
    options: StatusOptions,
) -> Result<StatusOptions, ParseError> {
    match args.split_first() {
        Some((emit, remaining)) => match emit.to_str() {
            Some("yaml") => parse_status_options(
                remaining,
                StatusOptions {
                    emit_yaml: true,
                    ..options
                },
            ),
            Some("text") => parse_status_options(remaining, options),
            Some("postcard") => Err(ParseError::InvalidStatusArgument(
                "postcard emit is not supported for status".into(),
            )),
            Some(other) => Err(ParseError::InvalidStatusArgument(format!(
                "unknown emit mode {other}"
            ))),
            None => Err(ParseError::InvalidStatusArgument(
                "emit mode is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--emit")),
    }
}

struct ParsedStatusValue<'a, T> {
    value: T,
    remaining: &'a [OsString],
}

fn parse_status_usize_value<'a>(
    args: &'a [OsString],
    flag: &'static str,
) -> Result<ParsedStatusValue<'a, usize>, ParseError> {
    parse_status_value(args, flag).and_then(|parsed| {
        parsed
            .value
            .parse::<usize>()
            .map(|value| ParsedStatusValue {
                value,
                remaining: parsed.remaining,
            })
            .map_err(|_| ParseError::InvalidStatusArgument(format!("{flag} must be a usize")))
    })
}

fn parse_status_u64_value<'a>(
    args: &'a [OsString],
    flag: &'static str,
) -> Result<ParsedStatusValue<'a, u64>, ParseError> {
    parse_status_value(args, flag).and_then(|parsed| {
        parsed
            .value
            .parse::<u64>()
            .map(|value| ParsedStatusValue {
                value,
                remaining: parsed.remaining,
            })
            .map_err(|_| ParseError::InvalidStatusArgument(format!("{flag} must be a u64")))
    })
}

fn parse_status_value<'a>(
    args: &'a [OsString],
    flag: &'static str,
) -> Result<ParsedStatusValue<'a, &'a str>, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument(flag)),
            Some(value) => Ok(ParsedStatusValue { value, remaining }),
            None => Err(ParseError::InvalidStatusArgument(format!(
                "{flag} value is not valid UTF-8"
            ))),
        },
        None => Err(ParseError::MissingArgument(flag)),
    }
}

fn validate_status_options(options: StatusOptions) -> Result<StatusOptions, ParseError> {
    let config = vb_runtime::shard::ShardConfig::default();
    validate_status_usize_limit(
        options.queue_depth,
        config.command_queue_capacity,
        "--queue-depth",
    )?;
    validate_status_usize_limit(options.active_runs, config.max_active_runs, "--active-runs")?;
    Ok(options)
}

fn validate_status_usize_limit(
    value: Option<usize>,
    max: usize,
    flag: &'static str,
) -> Result<(), ParseError> {
    match value {
        Some(actual) if actual > max => Err(ParseError::InvalidStatusArgument(format!(
            "{flag} must be <= {max}"
        ))),
        Some(_) | None => Ok(()),
    }
}

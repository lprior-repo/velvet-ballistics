//! Status command parsers.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use super::error::ParseError;
use super::shared::parse_output_format;
use super::status_value::{parse_status_u64_value, parse_status_usize_value};
use super::types::{Command, DurabilityMode, StatusOptions, SystemStatusOptions, VerifyProfile};

pub(super) fn parse_status(args: &[OsString]) -> Result<Command, ParseError> {
    let tokens = args.get(2..).ok_or(ParseError::NoCommand)?;
    let options = parse_status_options(tokens, StatusOptions::default())?;
    let output = parse_output_format(args);
    Ok(Command::Status { options, output })
}

/// Handle `--active-runs` for `parse_status_options`.
fn parse_status_active_runs(
    args: &[OsString],
    options: StatusOptions,
) -> Result<(StatusOptions, &[OsString]), ParseError> {
    let parsed = parse_status_usize_value(args, "--active-runs")?;
    Ok((
        StatusOptions {
            active_runs: Some(parsed.value),
            ..options
        },
        parsed.remaining,
    ))
}

/// Handle `--queue-depth` for `parse_status_options`.
fn parse_status_queue_depth(
    args: &[OsString],
    options: StatusOptions,
) -> Result<(StatusOptions, &[OsString]), ParseError> {
    let parsed = parse_status_usize_value(args, "--queue-depth")?;
    Ok((
        StatusOptions {
            queue_depth: Some(parsed.value),
            ..options
        },
        parsed.remaining,
    ))
}

/// Handle `--trace-dropped` for `parse_status_options`.
fn parse_status_trace_dropped(
    args: &[OsString],
    options: StatusOptions,
) -> Result<(StatusOptions, &[OsString]), ParseError> {
    let parsed = parse_status_u64_value(args, "--trace-dropped")?;
    Ok((
        StatusOptions {
            trace_dropped: Some(parsed.value),
            ..options
        },
        parsed.remaining,
    ))
}

fn parse_status_options(
    mut args: &[OsString],
    mut options: StatusOptions,
) -> Result<StatusOptions, ParseError> {
    loop {
        match args.split_first() {
            None => return validate_status_options(options),
            Some((flag, rest)) => match flag.to_str() {
                Some("--json" | "--jsonl") => {
                    args = rest;
                }
                Some("--emit") => match rest.split_first() {
                    Some((emit, remaining)) => match emit.to_str() {
                        Some("yaml") => {
                            options = StatusOptions {
                                emit_yaml: true,
                                ..options
                            };
                            args = remaining;
                        }
                        Some("text") => {
                            args = remaining;
                        }
                        Some("postcard") => {
                            return Err(ParseError::InvalidStatusArgument(
                                "postcard emit is not supported for status".into(),
                            ));
                        }
                        Some(other) => {
                            return Err(ParseError::InvalidStatusArgument(format!(
                                "unknown emit mode {other}"
                            )));
                        }
                        None => {
                            return Err(ParseError::InvalidStatusArgument(
                                "emit mode is not valid UTF-8".into(),
                            ));
                        }
                    },
                    None => {
                        return Err(ParseError::MissingArgument("--emit"));
                    }
                },
                Some("--active-runs") => {
                    let (updated, remaining) = parse_status_active_runs(rest, options)?;
                    options = updated;
                    args = remaining;
                }
                Some("--queue-depth") => {
                    let (updated, remaining) = parse_status_queue_depth(rest, options)?;
                    options = updated;
                    args = remaining;
                }
                Some("--trace-dropped") => {
                    let (updated, remaining) = parse_status_trace_dropped(rest, options)?;
                    options = updated;
                    args = remaining;
                }
                Some(other) if other.starts_with('-') => {
                    return Err(ParseError::InvalidStatusArgument(format!(
                        "unknown flag {other}"
                    )));
                }
                Some(other) => {
                    return Err(ParseError::InvalidStatusArgument(format!(
                        "unexpected positional argument {other}"
                    )));
                }
                None => {
                    return Err(ParseError::InvalidStatusArgument(
                        "argument is not valid UTF-8".into(),
                    ));
                }
            },
        }
    }
}

pub(super) fn parse_system(args: &[OsString]) -> Result<Command, ParseError> {
    match args.get(2).and_then(|value| value.to_str()) {
        Some("status") => parse_system_status_tokens(args.get(3..).ok_or(ParseError::NoCommand)?),
        Some(other) => Err(ParseError::InvalidSystemStatusArgument(format!(
            "unknown system command {other}"
        ))),
        None => Err(ParseError::MissingArgument("system subcommand")),
    }
}

fn parse_system_status_tokens(tokens: &[OsString]) -> Result<Command, ParseError> {
    let options = parse_system_status_options(tokens, SystemStatusOptions::default())?;
    let output = parse_output_format(tokens);
    Ok(Command::SystemStatus { options, output })
}

fn parse_system_status_options(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        None => Ok(options),
        Some((flag, rest)) => match flag.to_str() {
            Some("--json" | "--jsonl") => parse_system_status_options(rest, options),
            Some("--emit") => parse_system_status_emit(rest, options),
            Some("--profile") => parse_system_status_profile(rest, options),
            Some("--server") => parse_system_status_server(rest, options),
            Some(other) if other.starts_with('-') => Err(ParseError::InvalidSystemStatusArgument(
                format!("unknown flag {other}"),
            )),
            Some(other) => Err(ParseError::InvalidSystemStatusArgument(format!(
                "unexpected positional argument {other}"
            ))),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "argument is not valid UTF-8".into(),
            )),
        },
    }
}

fn parse_system_status_emit(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some("yaml") => parse_system_status_options(
                remaining,
                SystemStatusOptions {
                    emit_yaml: true,
                    ..options
                },
            ),
            Some("text") => parse_system_status_options(remaining, options),
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument("--emit")),
            Some(other) => Err(ParseError::InvalidSystemStatusArgument(format!(
                "unknown emit mode {other}"
            ))),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "emit mode is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--emit")),
    }
}

fn parse_system_status_profile(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some("quick") => parse_system_status_options(
                remaining,
                SystemStatusOptions {
                    profile: VerifyProfile::Quick,
                    ..options
                },
            ),
            Some("standard") => parse_system_status_options(remaining, options),
            Some("full") => parse_system_status_options(
                remaining,
                SystemStatusOptions {
                    profile: VerifyProfile::Full,
                    ..options
                },
            ),
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument("--profile")),
            Some(other) => Err(ParseError::UnknownProfile(other.into())),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "profile is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--profile")),
    }
}

fn parse_system_status_server(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument("--server")),
            Some(value) => parse_server_mode(value).and_then(|server| {
                parse_system_status_options(remaining, SystemStatusOptions { server, ..options })
            }),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "server mode is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--server")),
    }
}

fn parse_server_mode(raw: &str) -> Result<DurabilityMode, ParseError> {
    match raw {
        "none" => Ok(DurabilityMode::None),
        other => Err(ParseError::UnknownServerMode(other.into())),
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

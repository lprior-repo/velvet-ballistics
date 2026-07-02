//! System status command parser (subcommand of `system`).
#![forbid(unsafe_code)]

use std::ffi::OsString;

use super::error::ParseError;
use super::shared::parse_output_format;
use super::types::{Command, DurabilityMode, SystemStatusOptions, VerifyProfile};

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

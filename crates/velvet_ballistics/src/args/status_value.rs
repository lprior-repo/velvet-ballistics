//! Numeric value parsing helpers shared by status flags (`--active-runs`,
//! `--queue-depth`, `--trace-dropped`).
#![forbid(unsafe_code)]

use std::ffi::OsString;

use super::error::ParseError;

pub(super) struct ParsedStatusValue<'a, T> {
    pub(super) value: T,
    pub(super) remaining: &'a [OsString],
}

pub(super) fn parse_status_value<'a>(
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

pub(super) fn parse_status_usize_value<'a>(
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

pub(super) fn parse_status_u64_value<'a>(
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

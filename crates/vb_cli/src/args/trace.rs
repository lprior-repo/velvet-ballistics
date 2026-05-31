//! Trace command parser.
#![forbid(unsafe_code)]

use std::ffi::OsString;

use super::run_ops::parse_run_db_args;
use super::types::ParseError;
use crate::commands_journal::{TraceFilters, TraceStatus};

pub(super) fn parse_trace(args: &[OsString]) -> Result<super::types::Command, ParseError> {
    validate_trace_args(args)?;
    let a = parse_run_db_args(args, "trace")?;
    let filters = parse_trace_filters(args)?;
    Ok(super::types::Command::Trace {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
        filters,
    })
}

fn validate_trace_args(args: &[OsString]) -> Result<(), ParseError> {
    let mut index = 3_usize;
    while index < args.len() {
        let Some(raw) = args.get(index).and_then(|arg| arg.to_str()) else {
            return Err(ParseError::InvalidTraceArgument(
                "argument is not valid UTF-8".into(),
            ));
        };
        match raw {
            "--db" | "--step" | "--action" | "--status" | "--since-seq" | "--until-seq"
            | "--limit" | "--emit" => {
                let Some(value) = args
                    .get(index.saturating_add(1))
                    .and_then(|arg| arg.to_str())
                else {
                    return Err(ParseError::MissingArgument(match raw {
                        "--db" => "--db",
                        "--step" => "--step",
                        "--action" => "--action",
                        "--status" => "--status",
                        "--since-seq" => "--since-seq",
                        "--until-seq" => "--until-seq",
                        "--limit" => "--limit",
                        "--emit" => "--emit",
                        _ => "trace flag value",
                    }));
                };
                if value.starts_with("--") {
                    return Err(ParseError::MissingArgument(match raw {
                        "--db" => "--db",
                        "--step" => "--step",
                        "--action" => "--action",
                        "--status" => "--status",
                        "--since-seq" => "--since-seq",
                        "--until-seq" => "--until-seq",
                        "--limit" => "--limit",
                        "--emit" => "--emit",
                        _ => "trace flag value",
                    }));
                }
                if raw == "--emit" {
                    validate_trace_emit(value)?;
                }
                index = index.saturating_add(2);
            }
            other if other.starts_with("--") => {
                return Err(ParseError::InvalidTraceArgument(format!(
                    "unknown trace flag: {other}"
                )));
            }
            other => {
                return Err(ParseError::InvalidTraceArgument(format!(
                    "unexpected positional argument: {other}"
                )));
            }
        }
    }
    Ok(())
}

fn validate_trace_emit(value: &str) -> Result<(), ParseError> {
    matches!(value, "text" | "yaml" | "postcard")
        .then_some(())
        .ok_or_else(|| ParseError::InvalidArgument(format!("unknown emit mode for trace: {value}")))
}

fn parse_trace_filters(args: &[OsString]) -> Result<TraceFilters, ParseError> {
    let step = match optional_named_flag(args, "--step")? {
        Some(raw) => Some(parse_trace_u16("--step", &raw)?),
        None => None,
    };
    let action = match optional_named_flag(args, "--action")? {
        Some(raw) => Some(parse_trace_u16("--action", &raw)?),
        None => None,
    };
    let status = match optional_named_flag(args, "--status")? {
        Some(raw) => Some(parse_trace_status(&raw)?),
        None => None,
    };
    let since_seq = match optional_named_flag(args, "--since-seq")? {
        Some(raw) => Some(parse_trace_u64("--since-seq", &raw)?),
        None => None,
    };
    let until_seq = match optional_named_flag(args, "--until-seq")? {
        Some(raw) => Some(parse_trace_u64("--until-seq", &raw)?),
        None => None,
    };
    let limit = match optional_named_flag(args, "--limit")? {
        Some(raw) => Some(parse_trace_limit(&raw)?),
        None => None,
    };

    Ok(TraceFilters {
        step,
        action,
        status,
        since_seq,
        until_seq,
        limit,
    })
}

fn optional_named_flag(
    args: &[OsString],
    flag: &'static str,
) -> Result<Option<String>, ParseError> {
    for (index, arg) in args.iter().enumerate() {
        if arg == flag {
            let value = args
                .get(index.saturating_add(1))
                .and_then(|raw| raw.to_str())
                .ok_or(ParseError::MissingArgument(flag))?;
            if value.starts_with("--") {
                return Err(ParseError::MissingArgument(flag));
            }
            return Ok(Some(value.to_string()));
        }
    }
    Ok(None)
}

fn parse_trace_u16(flag: &'static str, raw: &str) -> Result<u16, ParseError> {
    raw.parse::<u16>()
        .map_err(|_| ParseError::InvalidTraceArgument(format!("{flag} must be a valid u16")))
}

fn parse_trace_limit(raw: &str) -> Result<usize, ParseError> {
    raw.parse::<usize>()
        .map_err(|_| ParseError::InvalidTraceArgument("--limit must be a valid usize".into()))
}

fn parse_trace_u64(flag: &'static str, raw: &str) -> Result<u64, ParseError> {
    raw.parse::<u64>()
        .map_err(|_| ParseError::InvalidTraceArgument(format!("{flag} must be a valid u64")))
}

fn parse_trace_status(raw: &str) -> Result<TraceStatus, ParseError> {
    match raw {
        "pending" => Ok(TraceStatus::Pending),
        "active" => Ok(TraceStatus::Active),
        "waiting_answer" => Ok(TraceStatus::WaitingAnswer),
        "cancelled" => Ok(TraceStatus::Cancelled),
        "completed" => Ok(TraceStatus::Completed),
        "failed" => Ok(TraceStatus::Failed),
        other => Err(ParseError::UnknownEventStatus(other.into())),
    }
}

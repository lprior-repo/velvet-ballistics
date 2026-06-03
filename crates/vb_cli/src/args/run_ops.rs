//! Run operation parsers (inspect, events, replay, retry, resume, cancel, incident, answer).
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::shared::{find_positional, named_flag, parse_output_format, validate_known_flags};
use super::types::{Command, DurabilityMode, EventStatus, OutputFormat, StepTarget};

pub(super) struct RunDbArgs {
    pub(super) run_id: String,
    pub(super) db: PathBuf,
    pub(super) output: OutputFormat,
}

pub(super) fn parse_run_db_args(
    args: &[OsString],
    command: &'static str,
) -> Result<RunDbArgs, ParseError> {
    let run_id = find_positional(args, 2)
        .and_then(|path| path.to_str().map(String::from))
        .ok_or(ParseError::MissingArgument("run_id"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(RunDbArgs {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

pub(super) fn parse_inspect(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args, "inspect")?;
    Ok(Command::Inspect {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_events(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "events")?;
    let a = parse_run_db_args(args, "events")?;
    let status = match named_flag(args, "--status") {
        Some(raw) => Some(parse_event_status(&raw)?),
        None => None,
    };
    let limit = match named_flag(args, "--limit") {
        Some(raw) => Some(parse_event_limit(&raw)?),
        None => None,
    };
    Ok(Command::Events {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
        status,
        limit,
    })
}

fn parse_event_status(raw: &str) -> Result<EventStatus, ParseError> {
    match raw {
        "pending" => Ok(EventStatus::Pending),
        "active" => Ok(EventStatus::Active),
        "waiting_answer" => Ok(EventStatus::WaitingAnswer),
        "cancelled" => Ok(EventStatus::Cancelled),
        "completed" => Ok(EventStatus::Completed),
        "failed" => Ok(EventStatus::Failed),
        other => Err(ParseError::UnknownEventStatus(other.into())),
    }
}

fn parse_event_limit(raw: &str) -> Result<i64, ParseError> {
    raw.parse::<i64>()
        .map_err(|_| ParseError::InvalidStatusArgument("--limit must be an integer".into()))
}

pub(super) fn parse_replay(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "replay")?;
    let a = parse_run_db_args(args, "replay")?;
    Ok(Command::Replay {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_retry(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "retry")?;
    let a = parse_run_db_args(args, "retry")?;
    let step = named_flag(args, "--step")
        .map(|s| {
            s.parse::<u16>()
                .map_err(|_| ParseError::InvalidStep(s))
        })
        .transpose()?;
    Ok(Command::Retry {
        run_id: a.run_id,
        step,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_resume(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "resume")?;
    let a = parse_run_db_args(args, "resume")?;
    Ok(Command::Resume {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_cancel(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "cancel")?;
    let run_id = find_positional(args, 2)
        .and_then(|path| path.to_str().map(String::from))
        .ok_or(ParseError::MissingArgument("run_id"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let reason = named_flag(args, "--reason");
    if reason.as_ref().is_some_and(|r| r.len() > 256) {
        return Err(ParseError::ReasonTooLong);
    }
    let output = parse_output_format(args);
    Ok(Command::Cancel {
        run_id,
        db: PathBuf::from(db),
        reason,
        output,
    })
}

pub(super) fn parse_incident(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "incident")?;
    let a = parse_run_db_args(args, "incident")?;
    Ok(Command::Incident {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_answer(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "answer")?;
    let run_id = find_positional(args, 2)
        .and_then(|path| path.to_str().map(String::from))
        .ok_or(ParseError::MissingArgument("run_id"))?;
    let slot_raw = named_flag(args, "--slot").ok_or(ParseError::MissingArgument("--slot"))?;
    let slot = slot_raw
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidStep(slot_raw))?;
    let value =
        named_flag(args, "--value").ok_or(ParseError::MissingArgument("--value"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Answer {
        run_id,
        slot,
        value: PathBuf::from(value),
        db: PathBuf::from(db),
        output,
    })
}

use std::ffi::OsString;
use std::path::PathBuf;

use super::shared::{named_flag, parse_output_format, positional_str};
use super::{Command, EventStatus, ParseError};

pub(super) fn parse_ai_context(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::AiContext {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

/// Common arguments for commands that operate on a run database entry.
struct RunDbArgs {
    run_id: String,
    db: PathBuf,
    output: super::OutputFormat,
}

fn parse_run_db_args(args: &[OsString]) -> Result<RunDbArgs, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(RunDbArgs {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

pub(super) fn parse_inspect(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Inspect {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_events(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
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
    let a = parse_run_db_args(args)?;
    Ok(Command::Replay {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_trace(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Trace {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_retry(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Retry {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_resume(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Resume {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

pub(super) fn parse_cancel(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let reason = named_flag(args, "--reason");
    if let Some(ref r) = reason
        && r.len() > 256
    {
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

pub(super) fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    let db = named_flag(args, "--db").map(PathBuf::from);
    let output = parse_output_format(args);
    Ok(Command::Doctor { db, output })
}

pub(super) fn parse_answer(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let step_raw = named_flag(args, "--step").ok_or(ParseError::MissingArgument("--step"))?;
    let step = step_raw
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidStep(step_raw))?;
    let value_file =
        named_flag(args, "--value-file").ok_or(ParseError::MissingArgument("--value-file"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Answer {
        run_id,
        step,
        value_file: PathBuf::from(value_file),
        db: PathBuf::from(db),
        output,
    })
}

pub(super) fn parse_diff(args: &[OsString]) -> Result<Command, ParseError> {
    let run_a = positional_str(args, 2, "run_a")?;
    let run_b = positional_str(args, 3, "run_b")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Diff {
        run_a,
        run_b,
        db: PathBuf::from(db),
        output,
    })
}

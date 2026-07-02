//! Run operation parsers (inspect, events, replay, retry, resume, cancel, incident, answer).
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::shared::{
    collect_all_named_flags, find_positional, has_flag, named_flag, parse_output_format,
    validate_known_flags,
};
use super::types::{Command, EventStatus, OutputFormat};
use vb_core::{ActionId, StepIdx, WorkflowDigest};

pub(super) struct RunDbArgs {
    pub(super) run_id: String,
    pub(super) db: PathBuf,
    pub(super) output: OutputFormat,
}

pub(super) fn parse_run_db_args(
    args: &[OsString],
    command: &'static str,
) -> Result<RunDbArgs, ParseError> {
    validate_known_flags(args, command)?;
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
    let expected_action_abi = parse_expected_action_abi_specs(args)?;
    let expected_policy_digests = parse_expected_policy_digest_specs(args)?;
    let allow_empty_expectations = has_flag(args, "--allow-empty-expectations");
    Ok(Command::Replay {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
        expected_action_abi,
        expected_policy_digests,
        allow_empty_expectations,
    })
}

/// Parses repeatable `--expected-action-abi <action_id>=<hex64>` specs.
///
/// Each value is `<u16>=<64 hex characters>` (lowercase or uppercase).
/// Multiple occurrences accumulate. Empty values or values without the `=`
/// separator are rejected with [`ParseError::InvalidReplayDigest`].
fn parse_expected_action_abi_specs(
    args: &[OsString],
) -> Result<Vec<(ActionId, WorkflowDigest)>, ParseError> {
    let raw_values = collect_all_named_flags(args, "--expected-action-abi");
    let mut entries: Vec<(ActionId, WorkflowDigest)> = Vec::with_capacity(raw_values.len());
    for raw in raw_values {
        let (id_part, hex_part) = split_replay_digest_spec("--expected-action-abi", &raw)?;
        let action_id = parse_replay_id::<u16>("--expected-action-abi action_id", id_part)?;
        let digest = parse_replay_workflow_digest("--expected-action-abi digest", hex_part)?;
        entries.push((ActionId::new(action_id), digest));
    }
    Ok(entries)
}

/// Parses repeatable `--expected-policy-digest <step>=<hex64>` specs.
///
/// Each value is `<u16>=<64 hex characters>` (lowercase or uppercase).
/// Multiple occurrences accumulate. Empty values or values without the `=`
/// separator are rejected with [`ParseError::InvalidReplayDigest`].
fn parse_expected_policy_digest_specs(
    args: &[OsString],
) -> Result<Vec<(StepIdx, WorkflowDigest)>, ParseError> {
    let raw_values = collect_all_named_flags(args, "--expected-policy-digest");
    let mut entries: Vec<(StepIdx, WorkflowDigest)> = Vec::with_capacity(raw_values.len());
    for raw in raw_values {
        let (id_part, hex_part) = split_replay_digest_spec("--expected-policy-digest", &raw)?;
        let step = parse_replay_id::<u16>("--expected-policy-digest step", id_part)?;
        let digest = parse_replay_workflow_digest("--expected-policy-digest digest", hex_part)?;
        entries.push((StepIdx::new(step), digest));
    }
    Ok(entries)
}

/// Splits a `<id>=<hex>` replay digest spec at the first `=` separator.
fn split_replay_digest_spec<'a>(
    flag: &'static str,
    raw: &'a str,
) -> Result<(&'a str, &'a str), ParseError> {
    if raw.is_empty() {
        return Err(ParseError::InvalidReplayDigest(format!(
            "{flag} value must be in <id>=<hex64> form, got empty value"
        )));
    }
    raw.split_once('=').ok_or_else(|| {
        ParseError::InvalidReplayDigest(format!(
            "{flag} value must be in <id>=<hex64> form, got {raw:?}"
        ))
    })
}

/// Parses the `<id>` half of a replay digest spec into the requested integer
/// type via `FromStr`, returning a typed [`ParseError::InvalidReplayDigest`]
/// on failure.
fn parse_replay_id<T>(label: &'static str, raw: &str) -> Result<T, ParseError>
where
    T: std::str::FromStr,
{
    if raw.is_empty() {
        return Err(ParseError::InvalidReplayDigest(format!(
            "{label} must not be empty"
        )));
    }
    raw.parse::<T>().map_err(|_| {
        ParseError::InvalidReplayDigest(format!("{label} must be a valid integer, got {raw:?}"))
    })
}

/// Parses the `<hex>` half of a replay digest spec into a [`WorkflowDigest`].
///
/// Requires exactly 64 hex characters (32 bytes); the resulting bytes are
/// returned as `WorkflowDigest::from_bytes`. Both lowercase and uppercase
/// hex characters are accepted. All arithmetic is checked; the input is
/// only indexed after length validation, so no slice can panic on a UTF-8
/// boundary or out-of-range access.
fn parse_replay_workflow_digest(
    label: &'static str,
    raw: &str,
) -> Result<WorkflowDigest, ParseError> {
    if raw.len() != 64 {
        return Err(ParseError::InvalidReplayDigest(format!(
            "{label} must be exactly 64 hex characters (32 bytes), got {} characters",
            raw.len()
        )));
    }
    let mut bytes = [0u8; 32];
    let mut index = 0_usize;
    while index < 32 {
        let lo = usize::checked_mul(index, 2).ok_or_else(|| {
            ParseError::InvalidReplayDigest(format!("{label} index overflow at byte {index}"))
        })?;
        let hi = usize::checked_add(lo, 2).ok_or_else(|| {
            ParseError::InvalidReplayDigest(format!("{label} index overflow at byte {index}"))
        })?;
        let pair = raw.get(lo..hi).ok_or_else(|| {
            ParseError::InvalidReplayDigest(format!(
                "{label} hex pair at byte {index} out of range"
            ))
        })?;
        let value = u8::from_str_radix(pair, 16).map_err(|_| {
            ParseError::InvalidReplayDigest(format!(
                "{label} contains non-hex characters at byte index {index}: {pair:?}"
            ))
        })?;
        let slot = bytes.get_mut(index).ok_or_else(|| {
            ParseError::InvalidReplayDigest(format!("{label} byte slot {index} out of range"))
        })?;
        *slot = value;
        index = index.saturating_add(1);
    }
    Ok(WorkflowDigest::from_bytes(bytes))
}

pub(super) fn parse_retry(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "retry")?;
    let a = parse_run_db_args(args, "retry")?;
    Ok(Command::Retry {
        run_id: a.run_id,
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

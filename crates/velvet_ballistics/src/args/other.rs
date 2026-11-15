//! Other command parsers (agent-context, ai-context, doctor, diff).
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::shared::{
    find_positional, named_flag, parse_output_format, positional_str, validate_known_flags,
};
use super::types::{Command, OutputFormat};

pub(super) fn parse_agent_context(args: &[OsString]) -> Result<Command, ParseError> {
    let mut deliver = None;
    let mut index = 2usize;
    while index < args.len() {
        let token = args
            .get(index)
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                ParseError::InvalidAgentContextArgument(String::from("invalid UTF-8 argument"))
            })?;
        match token {
            "--deliver" => {
                if deliver.is_some() {
                    return Err(ParseError::InvalidAgentContextArgument(String::from(
                        "duplicate --deliver",
                    )));
                }
                let value = args
                    .get(index.saturating_add(1))
                    .and_then(|raw| raw.to_str())
                    .filter(|raw| !raw.starts_with('-'))
                    .ok_or_else(|| {
                        ParseError::InvalidAgentContextArgument(String::from(
                            "--deliver requires stdout or file:<absolute-path>",
                        ))
                    })?;
                deliver = Some(String::from(value));
                index = index.saturating_add(2);
            }
            other if other.starts_with('-') => {
                return Err(ParseError::InvalidAgentContextArgument(format!(
                    "unknown flag {other}"
                )));
            }
            other => {
                return Err(ParseError::InvalidAgentContextArgument(format!(
                    "unexpected positional argument {other}"
                )));
            }
        }
    }
    Ok(Command::AgentContext { deliver })
}

pub(super) fn parse_ai_context(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "ai-context")?;
    let a = parse_ai_context_args(args)?;
    Ok(Command::AiContext {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

struct AiContextArgs {
    run_id: String,
    db: PathBuf,
    output: OutputFormat,
}

fn parse_ai_context_args(args: &[OsString]) -> Result<AiContextArgs, ParseError> {
    let run_id = find_positional(args, 2)
        .and_then(|path| path.to_str().map(String::from))
        .ok_or(ParseError::MissingArgument("run_id"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(AiContextArgs {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

pub(super) fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "doctor")?;
    let db = named_flag(args, "--db").map(PathBuf::from);
    let output = parse_output_format(args);
    Ok(Command::Doctor { db, output })
}

pub(super) fn parse_diff(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "diff")?;
    if named_flag(args, "--against").is_some() && named_flag(args, "--db").is_some() {
        return Err(ParseError::InvalidArgument(
            "--against and --db are alternate baseline modes".into(),
        ));
    }
    let run_a = positional_str(args, 2, "run_a")?;
    if has_run_pair(args) {
        parse_run_diff(args, run_a)
    } else {
        parse_workflow_diff(args, run_a)
    }
}

fn has_run_pair(args: &[OsString]) -> bool {
    args.get(3)
        .is_some_and(|value| value.to_str().is_some_and(|value| !value.starts_with('-')))
}

fn parse_workflow_diff(args: &[OsString], workflow: String) -> Result<Command, ParseError> {
    if args
        .iter()
        .any(|value| value == "--json" || value == "--jsonl")
    {
        return Err(ParseError::InvalidArgument(
            "workflow diff supports --emit text|yaml|postcard".into(),
        ));
    }
    Ok(Command::WorkflowDiff {
        workflow: PathBuf::from(workflow),
        against: named_flag(args, "--against").map(PathBuf::from),
        db: named_flag(args, "--db").map(PathBuf::from),
        output: parse_output_format(args),
    })
}

fn parse_run_diff(args: &[OsString], run_a: String) -> Result<Command, ParseError> {
    if named_flag(args, "--against").is_some() {
        return Err(ParseError::InvalidArgument(
            "--against cannot be combined with run comparison".into(),
        ));
    }
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

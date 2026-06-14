//! Other command parsers (agent-context, ai-context, doctor, diff).
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::run_ops::RunDbArgs;
use super::shared::{
    find_positional, named_flag, parse_output_format, positional_str, validate_known_flags,
};
use super::types::{Command, DiffMode, OutputFormat};

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
                            "--deliver requires stdout, file:<absolute-path>, or webhook:<url>",
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
    let output = parse_output_format(args);

    // Check if --against flag is present (workflow-vs-workflow mode)
    if let Some(against) = named_flag(args, "--against") {
        if named_flag(args, "--db").is_some() {
            return Err(ParseError::InvalidArgument(String::from(
                "diff accepts either workflow --against <old-workflow> without --db, or two run IDs plus --db",
            )));
        }
        // Workflow diff mode: diff <workflow> --against <old-workflow>
        let workflow = find_positional(args, 2)
            .and_then(|p| p.to_str().map(String::from))
            .ok_or(ParseError::MissingArgument("workflow.yaml"))?;
        Ok(Command::Diff {
            diff_mode: DiffMode::WorkflowAgainst {
                workflow: PathBuf::from(workflow),
                against: PathBuf::from(against),
            },
            output,
        })
    } else {
        // Run vs run mode: diff <run_a> <run_b> --db <path>
        let run_a = positional_str(args, 2, "run_a")?;
        let run_b = positional_str(args, 3, "run_b")?;
        let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
        Ok(Command::Diff {
            diff_mode: DiffMode::RunAgainst {
                run_a,
                run_b,
                db: PathBuf::from(db),
            },
            output,
        })
    }
}

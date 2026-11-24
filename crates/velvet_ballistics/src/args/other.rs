//! Other command parsers (agent-context, ai-context, doctor, diff).
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::flag_spec::{FlagSpec, known_flag_spec};
use super::shared::{
    find_positional, has_flag, named_flag, parse_output_format, validate_known_flags,
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

/// Positional arguments of `diff`, skipping flags and their values.
fn diff_positionals(args: &[OsString]) -> Vec<String> {
    let mut found = Vec::new();
    let mut skip_value = false;
    for token in args.iter().skip(2).filter_map(|arg| arg.to_str()) {
        if skip_value {
            skip_value = false;
        } else if token.starts_with('-') {
            let (name, attached) = token
                .split_once('=')
                .map_or((token, false), |(name, _)| (name, true));
            skip_value =
                !attached && matches!(known_flag_spec("diff", name), Some(FlagSpec::Value(_)));
        } else {
            found.push(token.to_owned());
        }
    }
    found
}

/// `diff <run_a> <run_b> --db <path>` compares runs; `diff <workflow.yaml>
/// (--against <old.yaml> | --db <path>)` compares workflow definitions.
pub(super) fn parse_diff(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "diff")?;
    let mut positionals = diff_positionals(args).into_iter();
    let first = positionals
        .next()
        .ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    match positionals.next() {
        Some(run_b) => parse_run_diff(args, first, run_b),
        None => parse_workflow_diff(args, first),
    }
}

fn parse_run_diff(args: &[OsString], run_a: String, run_b: String) -> Result<Command, ParseError> {
    if named_flag(args, "--against").is_some() {
        return Err(ParseError::InvalidArgument(
            "--against compares workflow files and cannot be combined with a run pair".into(),
        ));
    }
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Diff {
        run_a,
        run_b,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_workflow_diff(args: &[OsString], workflow: String) -> Result<Command, ParseError> {
    if has_flag(args, "--json") || has_flag(args, "--jsonl") {
        return Err(ParseError::InvalidArgument(
            "workflow diff supports --emit text|yaml|postcard, not --json/--jsonl".into(),
        ));
    }
    let against = named_flag(args, "--against").map(PathBuf::from);
    let db = named_flag(args, "--db").map(PathBuf::from);
    if against.is_some() && db.is_some() {
        return Err(ParseError::InvalidArgument(
            "--against and --db are mutually exclusive baselines; pass exactly one".into(),
        ));
    }
    if against.is_none() && db.is_none() {
        return Err(ParseError::MissingArgument(
            "baseline: pass --against <old.yaml> or --db <path>",
        ));
    }
    Ok(Command::WorkflowDiff {
        workflow: PathBuf::from(workflow),
        against,
        db,
        output: parse_output_format(args),
    })
}

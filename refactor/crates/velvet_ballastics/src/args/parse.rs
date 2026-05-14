//! Argument parsing dispatch and common helpers for velvet_ballastics.
//!
//! This module re-exports `parse_args` and contains the top-level dispatch
//! plus shared helpers used by per-command parsers.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::{
    ActionListParseState, ActionRegistryMode, ActionInspectParseState, Command, EmitTarget,
    ParseError, ParseError as Error, StatusOptions, VerifyProfile,
};

// --- Top-level dispatch ---

pub(crate) fn parse_args(args: &[OsString]) -> Result<Command, ParseError> {
    let subcommand = args
        .get(1)
        .and_then(|s| s.to_str())
        .ok_or(Error::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "agent-context" => Ok(Command::AgentContext),
        "ai-context" => parse_ai_context(args),
        "status" => parse_status(args),
        "action" => parse_action(args),
        "verify" => parse_verify(args),
        "validate" => parse_validate(args),
        "explain" => parse_explain(args),
        "compile" => parse_compile(args),
        "run" => parse_run(args),
        "run-compiled" => parse_run_compiled(args),
        "ipc-serve" => parse_ipc_serve(args),
        "inspect" => parse_inspect(args),
        "events" => parse_events(args),
        "replay" => parse_replay(args),
        "trace" => parse_trace(args),
        "retry" => parse_retry(args),
        "resume" => parse_resume(args),
        "bench-run" => parse_bench_run(args),
        "doctor" => parse_doctor(args),
        "answer" => parse_answer(args),
        "graph" => parse_graph(args),
        "diff" => parse_diff(args),
        "incident" => parse_incident(args),
        "simulate" => parse_simulate(args),
        "submit" => parse_submit(args),
        "cancel" => parse_cancel(args),
        other => Err(Error::UnknownCommand(other.into())),
    }
}

// --- Common helpers ---

/// Check if args contain a specific flag.
pub(crate) fn contains_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

/// Parse --json or --jsonl output format flags.
/// Returns OutputFormat::Text by default.
pub(crate) fn parse_output_format(args: &[OsString]) -> super::OutputFormat {
    if contains_flag(args, "--jsonl") {
        super::OutputFormat::Jsonl
    } else if contains_flag(args, "--json") {
        super::OutputFormat::Json
    } else {
        super::OutputFormat::Text
    }
}

pub(crate) fn positional(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<PathBuf, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(PathBuf::from)
        .ok_or(ParseError::MissingArgument(name))
}

pub(crate) fn positional_str(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<String, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(ParseError::MissingArgument(name))
}

pub(crate) fn named_flag(args: &[OsString], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            return args
                .get(i.checked_add(1)?)
                .and_then(|v| v.to_str())
                .map(String::from);
        }
    }
    None
}

/// Find the first positional argument (not starting with `--`) starting at `start_idx`.
pub(crate) fn find_positional(args: &[OsString], start_idx: usize) -> Option<PathBuf> {
    let mut i = start_idx;
    while i < args.len() {
        let arg = args.get(i)?.to_str()?;
        if arg.starts_with("--") {
            i = i.saturating_add(2);
        } else {
            return Some(PathBuf::from(arg));
        }
    }
    None
}

/// Parse durability mode from string.
pub(crate) fn parse_durability(raw: &str) -> Result<super::DurabilityMode, ParseError> {
    match raw {
        "strict" => Ok(super::DurabilityMode::Strict),
        "journaled" => Ok(super::DurabilityMode::Journaled),
        "none" => Ok(super::DurabilityMode::None),
        other => Err(ParseError::UnknownDurability(other.into())),
    }
}

// --- RunDbArgs helper ---

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

/// Parse optional --db for run commands (required when durability != None).
fn parse_optional_run_db(
    args: &[OsString],
    durability: super::DurabilityMode,
) -> Result<Option<PathBuf>, ParseError> {
    let db = named_flag(args, "--db").map(PathBuf::from);
    if durability == super::DurabilityMode::None {
        return Ok(db);
    }
    match db {
        Some(path) => Ok(Some(path)),
        None => Err(ParseError::MissingArgument("--db")),
    }
}

// --- Re-export parse helpers used by submodules ---

mod parse_commands;

pub(crate) use parse_commands::*;

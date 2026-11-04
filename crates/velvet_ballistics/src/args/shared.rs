//! Shared parsing utilities.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::types::OutputFormat;

/// Parse --emit text|yaml|postcard output format flags.
pub(super) fn parse_output_format(args: &[OsString]) -> OutputFormat {
    if has_flag(args, "--json") || has_flag(args, "--jsonl") {
        return OutputFormat::Text;
    }
    match named_flag(args, "--emit").as_deref() {
        Some("yaml") => OutputFormat::Yaml,
        Some("postcard") => OutputFormat::Postcard,
        Some("text") | Some(_) | None => OutputFormat::Text,
    }
}

pub(super) fn parse_compile_output_format(_args: &[OsString]) -> OutputFormat {
    OutputFormat::Text
}

pub(super) fn positional_str(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<String, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(ParseError::MissingArgument(name))
}

pub(super) fn named_flag(args: &[OsString], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        let Some(token) = arg.to_str() else {
            continue;
        };
        if token == flag {
            return args
                .get(i.checked_add(1)?)
                .and_then(|v| v.to_str())
                .map(String::from);
        }
        if let Some(value) = token
            .strip_prefix(flag)
            .and_then(|rest| rest.strip_prefix('='))
        {
            return Some(String::from(value));
        }
    }
    None
}

pub(super) fn has_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

/// Find the first positional argument (not starting with `--`) starting at `start_idx`.
/// This correctly skips over named flags and their values to locate the workflow path.
pub(super) fn find_positional(args: &[OsString], start_idx: usize) -> Option<PathBuf> {
    let command = args.get(1)?.to_str()?;
    let mut index = start_idx;
    while index < args.len() {
        let arg = args.get(index)?.to_str()?;
        if arg.starts_with('-') {
            let step = match super::flag_spec::known_flag_spec(command, arg) {
                Some(super::flag_spec::FlagSpec::Switch) | None => 1_usize,
                Some(super::flag_spec::FlagSpec::Value(_)) => 2_usize,
            };
            index = index.checked_add(step)?;
        } else {
            return Some(PathBuf::from(arg));
        }
    }
    None
}

pub(super) fn has_subcommand_help(args: &[OsString]) -> bool {
    match args.get(2..) {
        Some(rest) => rest.iter().any(|arg| arg == "--help" || arg == "-h"),
        None => false,
    }
}

/// Maximum positional arguments accepted per command, when the command's
/// parser is positional-countable. Commands with bespoke parsers return None.
fn max_positionals(command: &str) -> Option<usize> {
    match command {
        "validate" | "verify" | "explain" | "compile" | "bench-run" | "graph" | "simulate"
        | "run" | "run-compiled" | "submit" | "inspect" | "events" | "replay" | "resume"
        | "incident" | "retry" | "trace" | "cancel" | "answer" | "ai-context" => Some(1),
        "diff" => Some(2),
        "ipc-serve" | "doctor" => Some(0),
        _ => None,
    }
}

/// Value flags whose value is a filesystem path and must not be empty.
const PATH_VALUE_FLAGS: &[&str] = &[
    "--db",
    "--out",
    "--input-bin",
    "--socket",
    "--value-file",
    "--step-input",
];

pub(super) fn validate_known_flags(
    args: &[OsString],
    command: &'static str,
) -> Result<(), ParseError> {
    let mut index = 2_usize;
    let mut seen_value_flags: Vec<&str> = Vec::new();
    let mut positionals = 0_usize;
    while index < args.len() {
        let raw = args.get(index).ok_or_else(argument_index_overflow)?;
        let token = raw
            .to_str()
            .ok_or_else(|| ParseError::InvalidArgument("invalid UTF-8 argument".into()))?;
        if token.starts_with('-') {
            let (flag_name, attached) = match token.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (token, None),
            };
            let spec = super::flag_spec::known_flag_spec(command, flag_name).ok_or_else(|| {
                ParseError::UnknownFlag {
                    command,
                    flag: token.into(),
                }
            })?;
            if let super::flag_spec::FlagSpec::Value(name) = spec {
                if seen_value_flags.contains(&name) {
                    return Err(ParseError::InvalidArgument(format!(
                        "duplicate flag: {name}"
                    )));
                }
                seen_value_flags.push(name);
                if let Some(value) = attached {
                    if attached_value_invalid(name, value) {
                        return Err(invalid_flag_value(command, name, value));
                    }
                    validate_flag_value_domain(command, name, value)?;
                    index = advance_arg_index(index, 1_usize)?;
                    continue;
                }
            } else if attached.is_some() {
                return Err(ParseError::InvalidArgument(format!(
                    "flag {flag_name} does not take a value"
                )));
            }
            index = validate_flag_value(args, index, command, spec)?;
        } else {
            positionals = positionals
                .checked_add(1)
                .ok_or_else(argument_index_overflow)?;
            if let Some(max) = max_positionals(command)
                && positionals > max
            {
                return Err(ParseError::InvalidArgument(format!(
                    "unexpected argument: {token}"
                )));
            }
            index = advance_arg_index(index, 1_usize)?;
        }
    }
    Ok(())
}

fn attached_value_invalid(name: &str, value: &str) -> bool {
    value.is_empty() && PATH_VALUE_FLAGS.contains(&name)
}

fn invalid_flag_value(command: &str, name: &str, value: &str) -> ParseError {
    let _ = command;
    ParseError::InvalidArgument(format!(
        "empty value for {name}: a non-empty value is required (got {value:?})"
    ))
}

fn validate_flag_value(
    args: &[OsString],
    index: usize,
    command: &'static str,
    spec: super::flag_spec::FlagSpec,
) -> Result<usize, ParseError> {
    match spec {
        super::flag_spec::FlagSpec::Switch => advance_arg_index(index, 1_usize),
        super::flag_spec::FlagSpec::Value(name) => {
            let value_index = advance_arg_index(index, 1_usize)?;
            let value = args
                .get(value_index)
                .and_then(|raw| raw.to_str())
                .ok_or(ParseError::MissingArgument(name))?;
            if value.starts_with("--") {
                return Err(ParseError::MissingArgument(name));
            }
            if value.is_empty() && PATH_VALUE_FLAGS.contains(&name) {
                return Err(invalid_flag_value(command, name, value));
            }
            validate_flag_value_domain(command, name, value)?;
            advance_arg_index(index, 2_usize)
        }
    }
}

fn validate_flag_value_domain(
    command: &'static str,
    name: &'static str,
    value: &str,
) -> Result<(), ParseError> {
    if name != "--emit" {
        return Ok(());
    }
    if command == "compile" {
        return Ok(());
    }
    let valid = matches!(value, "text" | "yaml" | "postcard");
    if valid {
        Ok(())
    } else {
        Err(ParseError::InvalidArgument(format!(
            "unknown emit mode for {command}: {value}"
        )))
    }
}

fn advance_arg_index(index: usize, amount: usize) -> Result<usize, ParseError> {
    index
        .checked_add(amount)
        .ok_or_else(argument_index_overflow)
}

fn argument_index_overflow() -> ParseError {
    ParseError::InvalidArgument("argument index overflow".into())
}

/// Main entry point for argument parsing.
pub(crate) fn parse_args(args: &[OsString]) -> Result<super::types::Command, ParseError> {
    let subcommand = args
        .get(1)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(super::types::Command::Help { command: None }),
        "version" | "--version" | "-V" => Ok(super::types::Command::Version),
        "agent-context" | "ai-context" | "status" | "system" | "action" | "verify" | "validate"
        | "explain" | "compile" | "run" | "run-compiled" | "ipc-serve" | "inspect" | "events"
        | "replay" | "trace" | "retry" | "resume" | "bench-run" | "doctor" | "answer" | "graph"
        | "diff" | "incident" | "simulate" | "submit" | "cancel"
            if has_subcommand_help(args) =>
        {
            Ok(super::types::Command::Help {
                command: Some(subcommand.into()),
            })
        }
        "agent-context" => super::other::parse_agent_context(args),
        "ai-context" => super::other::parse_ai_context(args),
        "status" => super::status::parse_status(args),
        "system" => super::status::parse_system(args),
        "action" => super::action::parse_action(args),
        "verify" => super::workflow::parse_verify(args),
        "validate" => super::workflow::parse_validate(args),
        "explain" => super::workflow::parse_explain(args),
        "compile" => super::workflow::parse_compile(args),
        "run" => super::workflow::parse_run(args),
        "run-compiled" => super::workflow::parse_run_compiled(args),
        "ipc-serve" => super::workflow::parse_ipc_serve(args),
        "inspect" => super::run_ops::parse_inspect(args),
        "events" => super::run_ops::parse_events(args),
        "replay" => super::run_ops::parse_replay(args),
        "trace" => super::trace::parse_trace(args),
        "retry" => super::run_ops::parse_retry(args),
        "resume" => super::run_ops::parse_resume(args),
        "bench-run" => super::workflow::parse_bench_run(args),
        "doctor" => super::other::parse_doctor(args),
        "answer" => super::run_ops::parse_answer(args),
        "graph" => super::workflow::parse_graph(args),
        "diff" => super::other::parse_diff(args),
        "incident" => super::run_ops::parse_incident(args),
        "simulate" => super::workflow::parse_simulate(args),
        "submit" => super::workflow::parse_submit(args),
        "cancel" => super::run_ops::parse_cancel(args),
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

//! Argument parsing for velvet_ballistics.
#![forbid(unsafe_code)]

mod action;
mod error;
mod flag_spec;
mod other;
mod run;
mod run_ops;
mod shared;
mod status;
mod trace;
mod types;

pub(crate) use types::{
    ActionRegistryMode, Command, DurabilityMode, EmitTarget, EventStatus, OutputFormat, ParseError,
    StatusOptions, StepTarget, SystemStatusOptions, VALID_COMMANDS, VerifyProfile,
};

pub(crate) fn parse_args(args: &[std::ffi::OsString]) -> Result<Command, ParseError> {
    let subcommand = args
        .get(1)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "agent-context" | "ai-context" | "status" | "system" | "action" | "verify" | "validate"
        | "explain" | "compile" | "run" | "run-compiled" | "ipc-serve" | "inspect" | "events"
        | "replay" | "trace" | "retry" | "resume" | "bench-run" | "doctor" | "answer" | "graph"
        | "diff" | "incident" | "simulate" | "submit" | "cancel"
            if shared::has_subcommand_help(args) =>
        {
            Ok(Command::Help)
        }
        "agent-context" => other::parse_agent_context(args),
        "ai-context" => other::parse_ai_context(args),
        "status" => status::parse_status(args),
        "system" => status::parse_system(args),
        "action" => action::parse_action(args),
        "verify" => run::parse_verify(args),
        "validate" => run::parse_validate(args),
        "explain" => run::parse_explain(args),
        "compile" => run::parse_compile(args),
        "run" => run::parse_run(args),
        "run-compiled" => run::parse_run_compiled(args),
        "ipc-serve" => run::parse_ipc_serve(args),
        "inspect" => run_ops::parse_inspect(args),
        "events" => run_ops::parse_events(args),
        "replay" => run_ops::parse_replay(args),
        "trace" => trace::parse_trace(args),
        "retry" => run_ops::parse_retry(args),
        "resume" => run_ops::parse_resume(args),
        "bench-run" => run::parse_bench_run(args),
        "doctor" => other::parse_doctor(args),
        "answer" => run_ops::parse_answer(args),
        "graph" => run::parse_graph(args),
        "diff" => other::parse_diff(args),
        "incident" => run_ops::parse_incident(args),
        "simulate" => run::parse_simulate(args),
        "submit" => run::parse_submit(args),
        "cancel" => run_ops::parse_cancel(args),
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

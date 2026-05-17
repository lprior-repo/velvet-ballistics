//! Argument parsing for velvet_ballastics.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

mod action;
mod error;
mod run_db;
mod shared;
mod status;
#[cfg(test)]
mod tests;
mod workflow;

pub(crate) use error::ParseError;

/// Structured output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OutputFormat {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// JSON object output.
    Json,
    /// JSON Lines output (one JSON object per line).
    Jsonl,
}

/// Verification profile controlling depth of static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VerifyProfile {
    /// Fast surface checks only.
    Quick,
    /// Default verification depth.
    #[default]
    Standard,
    /// Exhaustive verification including budget, capability, taint.
    Full,
}

impl VerifyProfile {
    /// Returns the name used on the command line for this profile.
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Version,
    AgentContext,
    AiContext {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Status {
        options: StatusOptions,
        output: OutputFormat,
    },
    ActionList {
        output: OutputFormat,
        registry: ActionRegistryMode,
    },
    ActionInspect {
        action_id: u16,
        output: OutputFormat,
        registry: ActionRegistryMode,
    },
    Verify {
        workflow: PathBuf,
        profile: VerifyProfile,
        output: OutputFormat,
    },
    Validate {
        workflow: PathBuf,
        #[allow(dead_code)]
        output: OutputFormat,
    },
    Compile {
        workflow: PathBuf,
        emit: EmitTarget,
        out: PathBuf,
        output: OutputFormat,
    },
    Run {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
        step: Option<StepTarget>,
        output: OutputFormat,
    },
    RunCompiled {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
        output: OutputFormat,
    },
    IpcServe {
        socket: PathBuf,
        db: PathBuf,
    },
    Inspect {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Events {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Replay {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Trace {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Retry {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Resume {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    BenchRun {
        workflow: PathBuf,
        output: OutputFormat,
    },
    Doctor {
        db: Option<PathBuf>,
        output: OutputFormat,
    },
    Explain {
        workflow: PathBuf,
        #[allow(dead_code)]
        output: OutputFormat,
    },
    Answer {
        run_id: String,
        step: u16,
        value_file: PathBuf,
        db: PathBuf,
        output: OutputFormat,
    },
    Graph {
        workflow: PathBuf,
        output: OutputFormat,
    },
    Diff {
        run_a: String,
        run_b: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Incident {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Simulate {
        workflow: PathBuf,
        output: OutputFormat,
    },
    Submit {
        workflow: PathBuf,
        input_bin: PathBuf,
        db: PathBuf,
        durability: DurabilityMode,
        output: OutputFormat,
    },
    Cancel {
        run_id: String,
        db: PathBuf,
        reason: Option<String>,
        output: OutputFormat,
    },
}

pub(crate) const VALID_COMMANDS: &str = "help, version, agent-context, ai-context, status, action, validate, verify, explain, compile, run, run-compiled, ipc-serve, inspect, events, replay, trace, retry, resume, bench-run, doctor, answer, graph, diff, incident, submit, simulate, cancel";

/// Optional diagnostic status values used when no live runtime handle exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct StatusOptions {
    pub(crate) active_runs: Option<usize>,
    pub(crate) queue_depth: Option<usize>,
    pub(crate) trace_dropped: Option<u64>,
    pub(crate) emit_yaml: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ActionRegistryMode {
    #[default]
    Registered,
    Empty,
    Uninitialized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmitTarget {
    Ir,
    Rust,
    Yaml,
    Postcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DurabilityMode {
    Strict,
    Journaled,
    None,
}

/// Single-step isolation target for `run --step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepTarget {
    pub(crate) step_id: u16,
    pub(crate) step_input: PathBuf,
}

pub(crate) fn parse_args(args: &[OsString]) -> Result<Command, ParseError> {
    let subcommand = args
        .get(1)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "agent-context" => Ok(Command::AgentContext),
        "ai-context" => run_db::parse_ai_context(args),
        "status" => status::parse_status(args),
        "action" => action::parse_action(args),
        "verify" => workflow::parse_verify(args),
        "validate" => workflow::parse_validate(args),
        "explain" => workflow::parse_explain(args),
        "compile" => workflow::parse_compile(args),
        "run" => workflow::parse_run(args),
        "run-compiled" => workflow::parse_run_compiled(args),
        "ipc-serve" => workflow::parse_ipc_serve(args),
        "inspect" => run_db::parse_inspect(args),
        "events" => run_db::parse_events(args),
        "replay" => run_db::parse_replay(args),
        "trace" => run_db::parse_trace(args),
        "retry" => run_db::parse_retry(args),
        "resume" => run_db::parse_resume(args),
        "bench-run" => workflow::parse_bench_run(args),
        "doctor" => run_db::parse_doctor(args),
        "answer" => run_db::parse_answer(args),
        "graph" => workflow::parse_graph(args),
        "diff" => run_db::parse_diff(args),
        "incident" => run_db::parse_incident(args),
        "simulate" => workflow::parse_simulate(args),
        "submit" => workflow::parse_submit(args),
        "cancel" => run_db::parse_cancel(args),
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

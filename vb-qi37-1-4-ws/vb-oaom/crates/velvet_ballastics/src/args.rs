//! Argument parsing for velvet_ballastics.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

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
        db: PathBuf,
        output: OutputFormat,
    },
    Explain {
        workflow: PathBuf,
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
}

pub(crate) const VALID_COMMANDS: &str = "help, version, agent-context, ai-context, status, action, validate, verify, explain, compile, run, run-compiled, ipc-serve, inspect, events, replay, trace, retry, resume, bench-run, doctor, answer, graph, diff, incident, submit, simulate";

/// Optional diagnostic status values used when no live runtime handle exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct StatusOptions {
    pub(crate) active_runs: Option<usize>,
    pub(crate) queue_depth: Option<usize>,
    pub(crate) trace_dropped: Option<u64>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ParseError {
    MissingArgument(&'static str),
    UnknownEmitTarget(String),
    UnknownDurability(String),
    UnknownProfile(String),
    UnknownCommand(String),
    InvalidStatusArgument(String),
    UnknownActionCommand(String),
    UnknownActionRegistry(String),
    MissingActionRegistryValue,
    UnknownActionListFlag(String),
    UnexpectedActionListArgument(String),
    UnknownActionInspectFlag(String),
    UnexpectedActionInspectArgument(String),
    InvalidActionId(String),
    NoCommand,
    InvalidStep(String),
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
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

fn parse_ai_context(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::AiContext {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_status(args: &[OsString]) -> Result<Command, ParseError> {
    let tokens = args.get(2..).ok_or(ParseError::NoCommand)?;
    let options = parse_status_options(tokens, StatusOptions::default())?;
    let output = parse_output_format(args);
    Ok(Command::Status { options, output })
}

fn parse_status_options(
    args: &[OsString],
    options: StatusOptions,
) -> Result<StatusOptions, ParseError> {
    match args.split_first() {
        None => validate_status_options(options),
        Some((flag, rest)) => match flag.to_str() {
            Some("--json" | "--jsonl") => parse_status_options(rest, options),
            Some("--active-runs") => {
                let parsed = parse_status_usize_value(rest, "--active-runs")?;
                parse_status_options(
                    parsed.remaining,
                    StatusOptions {
                        active_runs: Some(parsed.value),
                        ..options
                    },
                )
            }
            Some("--queue-depth") => {
                let parsed = parse_status_usize_value(rest, "--queue-depth")?;
                parse_status_options(
                    parsed.remaining,
                    StatusOptions {
                        queue_depth: Some(parsed.value),
                        ..options
                    },
                )
            }
            Some("--trace-dropped") => {
                let parsed = parse_status_u64_value(rest, "--trace-dropped")?;
                parse_status_options(
                    parsed.remaining,
                    StatusOptions {
                        trace_dropped: Some(parsed.value),
                        ..options
                    },
                )
            }
            Some(other) if other.starts_with('-') => Err(ParseError::InvalidStatusArgument(
                format!("unknown flag {other}"),
            )),
            Some(other) => Err(ParseError::InvalidStatusArgument(format!(
                "unexpected positional argument {other}"
            ))),
            None => Err(ParseError::InvalidStatusArgument(
                "argument is not valid UTF-8".into(),
            )),
        },
    }
}

fn parse_action(args: &[OsString]) -> Result<Command, ParseError> {
    let action_command = args
        .get(2)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::MissingArgument("action subcommand"))?;
    let action_args = match args.get(3..) {
        Some(values) => values,
        None => &[],
    };
    if action_command == "inspect" {
        return parse_action_inspect(action_args);
    }
    if action_command != "list" {
        return Err(ParseError::UnknownActionCommand(action_command.into()));
    }
    let parsed = parse_action_list_args(
        action_args,
        ActionListParseState {
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        },
    )?;
    Ok(Command::ActionList {
        output: parsed.output,
        registry: parsed.registry,
    })
}

fn parse_action_inspect(args: &[OsString]) -> Result<Command, ParseError> {
    let (raw_id, rest) = args
        .split_first()
        .ok_or(ParseError::MissingArgument("action_id"))?;
    let id = raw_id
        .to_str()
        .ok_or_else(|| ParseError::InvalidActionId(format!("{raw_id:?}")))?
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidActionId(raw_id.to_string_lossy().into_owned()))?;
    let parsed = parse_action_inspect_args(
        rest,
        ActionInspectParseState {
            output: OutputFormat::Text,
            registry: ActionRegistryMode::Registered,
        },
    )?;
    Ok(Command::ActionInspect {
        action_id: id,
        output: parsed.output,
        registry: parsed.registry,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionListParseState {
    output: OutputFormat,
    registry: ActionRegistryMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionInspectParseState {
    output: OutputFormat,
    registry: ActionRegistryMode,
}

fn parse_action_inspect_args(
    args: &[OsString],
    state: ActionInspectParseState,
) -> Result<ActionInspectParseState, ParseError> {
    match args.split_first() {
        None => Ok(state),
        Some((raw, rest)) => match raw.to_str() {
            Some("--json") => parse_action_inspect_args(
                rest,
                ActionInspectParseState {
                    output: OutputFormat::Json,
                    ..state
                },
            ),
            Some("--jsonl") => parse_action_inspect_args(
                rest,
                ActionInspectParseState {
                    output: OutputFormat::Jsonl,
                    ..state
                },
            ),
            Some("--registry") => parse_action_inspect_registry_arg(rest, state),
            Some(flag) if flag.starts_with("--") => {
                Err(ParseError::UnknownActionInspectFlag(flag.into()))
            }
            Some(arg) => Err(ParseError::UnexpectedActionInspectArgument(arg.into())),
            None => Err(ParseError::UnexpectedActionInspectArgument(format!(
                "{raw:?}"
            ))),
        },
    }
}

fn parse_action_list_args(
    args: &[OsString],
    state: ActionListParseState,
) -> Result<ActionListParseState, ParseError> {
    match args.split_first() {
        None => Ok(state),
        Some((raw, rest)) => match raw.to_str() {
            Some("--json") => parse_action_list_args(
                rest,
                ActionListParseState {
                    output: OutputFormat::Json,
                    ..state
                },
            ),
            Some("--jsonl") => parse_action_list_args(
                rest,
                ActionListParseState {
                    output: OutputFormat::Jsonl,
                    ..state
                },
            ),
            Some("--registry") => parse_action_registry_arg(rest, state),
            Some(flag) if flag.starts_with("--") => {
                Err(ParseError::UnknownActionListFlag(flag.into()))
            }
            Some(arg) => Err(ParseError::UnexpectedActionListArgument(arg.into())),
            None => Err(ParseError::UnexpectedActionListArgument(format!(
                "{raw:?}"
            ))),
        },
    }
}

struct ParsedStatusValue<'a, T> {
    value: T,
    remaining: &'a [OsString],
}

fn parse_status_usize_value<'a>(
    args: &'a [OsString],
    flag: &'static str,
) -> Result<ParsedStatusValue<'a, usize>, ParseError> {
    parse_status_value(args, flag).and_then(|parsed| {
        parsed
            .value
            .parse::<usize>()
            .map(|value| ParsedStatusValue {
                value,
                remaining: parsed.remaining,
            })
            .map_err(|_| ParseError::InvalidStatusArgument(format!("{flag} must be a usize")))
    })
}

fn parse_status_u64_value<'a>(
    args: &'a [OsString],
    flag: &'static str,
) -> Result<ParsedStatusValue<'a, u64>, ParseError> {
    parse_status_value(args, flag).and_then(|parsed| {
        parsed
            .value
            .parse::<u64>()
            .map(|value| ParsedStatusValue {
                value,
                remaining: parsed.remaining,
            })
            .map_err(|_| ParseError::InvalidStatusArgument(format!("{flag} must be a u64")))
    })
}

fn parse_status_value<'a>(
    args: &'a [OsString],
    flag: &'static str,
) -> Result<ParsedStatusValue<'a, &'a str>, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument(flag)),
            Some(value) => Ok(ParsedStatusValue { value, remaining }),
            None => Err(ParseError::InvalidStatusArgument(format!(
                "{flag} value is not valid UTF-8"
            ))),
        },
        None => Err(ParseError::MissingArgument(flag)),
    }
}

fn validate_status_options(options: StatusOptions) -> Result<StatusOptions, ParseError> {
    let config = vb_runtime::shard::ShardConfig::default();
    validate_status_usize_limit(
        options.queue_depth,
        config.command_queue_capacity,
        "--queue-depth",
    )?;
    validate_status_usize_limit(options.active_runs, config.max_active_runs, "--active-runs")?;
    Ok(options)
}

fn validate_status_usize_limit(
    value: Option<usize>,
    max: usize,
    flag: &'static str,
) -> Result<(), ParseError> {
    match value {
        Some(actual) if actual > max => Err(ParseError::InvalidStatusArgument(format!(
            "{flag} must be <= {max}"
        ))),
        Some(_) | None => Ok(()),
    }
}

fn parse_action_registry_arg(
    args: &[OsString],
    state: ActionListParseState,
) -> Result<ActionListParseState, ParseError> {
    match args.split_first() {
        Some((raw, rest)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingActionRegistryValue),
            Some(value) => parse_action_registry_mode(value).and_then(|registry| {
                parse_action_list_args(rest, ActionListParseState { registry, ..state })
            }),
            None => Err(ParseError::MissingActionRegistryValue),
        },
        None => Err(ParseError::MissingActionRegistryValue),
    }
}

fn parse_action_inspect_registry_arg(
    args: &[OsString],
    state: ActionInspectParseState,
) -> Result<ActionInspectParseState, ParseError> {
    match args.split_first() {
        Some((raw, rest)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingActionRegistryValue),
            Some(value) => parse_action_registry_mode(value).and_then(|registry| {
                parse_action_inspect_args(rest, ActionInspectParseState { registry, ..state })
            }),
            None => Err(ParseError::MissingActionRegistryValue),
        },
        None => Err(ParseError::MissingActionRegistryValue),
    }
}

fn parse_action_registry_mode(value: &str) -> Result<ActionRegistryMode, ParseError> {
    match value {
        "registered" => Ok(ActionRegistryMode::Registered),
        "empty" => Ok(ActionRegistryMode::Empty),
        "uninitialized" => Ok(ActionRegistryMode::Uninitialized),
        other => Err(ParseError::UnknownActionRegistry(other.into())),
    }
}

fn parse_verify(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let profile = match named_flag(args, "--profile") {
        Some(raw) => match raw.as_str() {
            "quick" => VerifyProfile::Quick,
            "standard" => VerifyProfile::Standard,
            "full" => VerifyProfile::Full,
            other => return Err(ParseError::UnknownProfile(other.into())),
        },
        None => VerifyProfile::default(),
    };
    let output = parse_output_format(args);
    Ok(Command::Verify {
        workflow,
        profile,
        output,
    })
}

fn parse_validate(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Validate { workflow, output })
}

fn parse_explain(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Explain { workflow, output })
}

fn parse_compile(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let emit_raw = named_flag(args, "--emit").ok_or(ParseError::MissingArgument("--emit"))?;
    let emit = match emit_raw.as_str() {
        "ir" => EmitTarget::Ir,
        "rust" => EmitTarget::Rust,
        "yaml" => EmitTarget::Yaml,
        "postcard" => EmitTarget::Postcard,
        other => return Err(ParseError::UnknownEmitTarget(other.into())),
    };
    let out = named_flag(args, "--out").ok_or(ParseError::MissingArgument("--out"))?;
    let output = parse_output_format(args);
    Ok(Command::Compile {
        workflow,
        emit,
        out: PathBuf::from(out),
        output,
    })
}

fn parse_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let input_bin =
        named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let durability_raw =
        named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let db = parse_optional_run_db(args, durability)?;
    let step = parse_optional_step(args)?;
    let output = parse_output_format(args);
    Ok(Command::Run {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
        step,
        output,
    })
}

fn parse_optional_step(args: &[OsString]) -> Result<Option<StepTarget>, ParseError> {
    let step_raw = match named_flag(args, "--step") {
        Some(s) => s,
        None => return Ok(None),
    };
    let step_id = step_raw
        .parse::<u16>()
        .map_err(|_| ParseError::MissingArgument("--step"))?;
    let step_input =
        named_flag(args, "--step-input").ok_or(ParseError::MissingArgument("--step-input"))?;
    Ok(Some(StepTarget {
        step_id,
        step_input: PathBuf::from(step_input),
    }))
}

fn parse_run_compiled(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.vbir")?;
    let input_bin =
        named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let durability_raw =
        named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let db = parse_optional_run_db(args, durability)?;
    let output = parse_output_format(args);
    Ok(Command::RunCompiled {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
        output,
    })
}

fn parse_optional_run_db(
    args: &[OsString],
    durability: DurabilityMode,
) -> Result<Option<PathBuf>, ParseError> {
    let db = named_flag(args, "--db").map(PathBuf::from);
    if durability == DurabilityMode::None {
        return Ok(db);
    }
    match db {
        Some(path) => Ok(Some(path)),
        None => Err(ParseError::MissingArgument("--db")),
    }
}

fn parse_ipc_serve(args: &[OsString]) -> Result<Command, ParseError> {
    let socket = named_flag(args, "--socket").ok_or(ParseError::MissingArgument("--socket"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::IpcServe {
        socket: PathBuf::from(socket),
        db: PathBuf::from(db),
    })
}

fn parse_inspect(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Inspect {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

/// Common arguments for commands that operate on a run database entry.
struct RunDbArgs {
    run_id: String,
    db: PathBuf,
    output: OutputFormat,
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

fn parse_events(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Events {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_replay(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Replay {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_trace(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Trace {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_retry(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Retry {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_resume(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Resume {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::BenchRun { workflow, output })
}

fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Doctor {
        db: PathBuf::from(db),
        output,
    })
}

fn parse_answer(args: &[OsString]) -> Result<Command, ParseError> {
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

fn parse_graph(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Graph { workflow, output })
}

fn parse_diff(args: &[OsString]) -> Result<Command, ParseError> {
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

fn parse_incident(args: &[OsString]) -> Result<Command, ParseError> {
    let a = parse_run_db_args(args)?;
    Ok(Command::Incident {
        run_id: a.run_id,
        db: a.db,
        output: a.output,
    })
}

fn parse_simulate(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Simulate { workflow, output })
}

fn parse_submit(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let input_bin =
        named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let durability_raw =
        named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let output = parse_output_format(args);
    Ok(Command::Submit {
        workflow,
        input_bin: PathBuf::from(input_bin),
        db: PathBuf::from(db),
        durability,
        output,
    })
}

fn parse_durability(raw: &str) -> Result<DurabilityMode, ParseError> {
    match raw {
        "strict" => Ok(DurabilityMode::Strict),
        "journaled" => Ok(DurabilityMode::Journaled),
        "none" => Ok(DurabilityMode::None),
        other => Err(ParseError::UnknownDurability(other.into())),
    }
}

/// Parse --json or --jsonl output format flags.
/// Returns OutputFormat::Text by default.
fn parse_output_format(args: &[OsString]) -> OutputFormat {
    if contains_flag(args, "--jsonl") {
        OutputFormat::Jsonl
    } else if contains_flag(args, "--json") {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

/// Check if args contain a specific flag.
fn contains_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn positional(args: &[OsString], index: usize, name: &'static str) -> Result<PathBuf, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(PathBuf::from)
        .ok_or(ParseError::MissingArgument(name))
}

fn positional_str(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<String, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(ParseError::MissingArgument(name))
}

fn named_flag(args: &[OsString], flag: &str) -> Option<String> {
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

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingArgument(name) => write!(formatter, "missing argument: {name}"),
            Self::UnknownEmitTarget(target) => {
                write!(
                    formatter,
                    "unknown emit target: {target} (expected: ir, rust, yaml, postcard)"
                )
            }
            Self::UnknownDurability(mode) => {
                write!(
                    formatter,
                    "unknown durability mode: {mode} (expected: strict, journaled, none)"
                )
            }
            Self::UnknownProfile(profile) => {
                write!(
                    formatter,
                    "unknown verify profile: {profile} (expected: quick, standard, full)"
                )
            }
            Self::UnknownCommand(cmd) => {
                write!(
                    formatter,
                    "unknown command: {cmd} (expected one of: {VALID_COMMANDS})"
                )
            }
            Self::InvalidStatusArgument(reason) => {
                write!(formatter, "invalid status argument: {reason}")
            }
            Self::UnknownActionCommand(cmd) => {
                write!(
                    formatter,
                    "unknown action command: {cmd} (expected: list, inspect)"
                )
            }
            Self::UnknownActionRegistry(registry) => {
                write!(
                    formatter,
                    "unknown action registry: {registry} (expected: registered, empty, uninitialized)"
                )
            }
            Self::MissingActionRegistryValue => write!(
                formatter,
                "missing action-args value for --registry (expected: registered, empty, uninitialized)"
            ),
            Self::UnknownActionListFlag(flag) => {
                write!(formatter, "unknown action list flag: {flag}")
            }
            Self::UnexpectedActionListArgument(argument) => {
                write!(formatter, "unexpected action list argument: {argument}")
            }
            Self::UnknownActionInspectFlag(flag) => {
                write!(formatter, "unknown action inspect flag: {flag}")
            }
            Self::UnexpectedActionInspectArgument(argument) => {
                write!(formatter, "unexpected action inspect argument: {argument}")
            }
            Self::InvalidActionId(action_id) => {
                write!(formatter, "invalid action id: {action_id}")
            }
            Self::NoCommand => write!(formatter, "no command provided"),
            Self::InvalidStep(step) => write!(formatter, "invalid step: {step}"),
        }
    }
}

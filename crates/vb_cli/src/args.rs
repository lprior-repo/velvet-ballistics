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
    SystemStatus {
        options: SystemStatusOptions,
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

pub(crate) const VALID_COMMANDS: &str = "help, version, agent-context, ai-context, status, system, action, validate, verify, explain, compile, run, run-compiled, ipc-serve, inspect, events, replay, trace, retry, resume, bench-run, doctor, answer, graph, diff, incident, submit, simulate, cancel";

/// Optional diagnostic status values used when no live runtime handle exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct StatusOptions {
    pub(crate) active_runs: Option<usize>,
    pub(crate) queue_depth: Option<usize>,
    pub(crate) trace_dropped: Option<u64>,
    pub(crate) emit_yaml: bool,
}

/// System-status probe depth and runtime selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SystemStatusOptions {
    pub(crate) profile: VerifyProfile,
    pub(crate) server: DurabilityMode,
    pub(crate) emit_yaml: bool,
}

impl Default for SystemStatusOptions {
    fn default() -> Self {
        Self {
            profile: VerifyProfile::Standard,
            server: DurabilityMode::None,
            emit_yaml: false,
        }
    }
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

impl DurabilityMode {
    #[must_use]
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Journaled => "journaled",
            Self::None => "none",
        }
    }
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
    UnknownServerMode(String),
    InvalidStatusArgument(String),
    InvalidSystemStatusArgument(String),
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
    ReasonTooLong,
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
        "system" => parse_system(args),
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
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

fn parse_system(args: &[OsString]) -> Result<Command, ParseError> {
    match args.get(2).and_then(|value| value.to_str()) {
        Some("status") => parse_system_status_tokens(args.get(3..).ok_or(ParseError::NoCommand)?),
        Some(other) => Err(ParseError::InvalidSystemStatusArgument(format!(
            "unknown system command {other}"
        ))),
        None => Err(ParseError::MissingArgument("system subcommand")),
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
            Some("--emit") => match rest.split_first() {
                Some((emit, remaining)) => match emit.to_str() {
                    Some("yaml") => parse_status_options(
                        remaining,
                        StatusOptions {
                            emit_yaml: true,
                            ..options
                        },
                    ),
                    Some("text") => parse_status_options(remaining, options),
                    Some("postcard") => Err(ParseError::InvalidStatusArgument(
                        "postcard emit is not supported for status".into(),
                    )),
                    Some(other) => Err(ParseError::InvalidStatusArgument(format!(
                        "unknown emit mode {other}"
                    ))),
                    None => Err(ParseError::InvalidStatusArgument(
                        "emit mode is not valid UTF-8".into(),
                    )),
                },
                None => Err(ParseError::MissingArgument("--emit")),
            },
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

fn parse_system_status_tokens(tokens: &[OsString]) -> Result<Command, ParseError> {
    let options = parse_system_status_options(tokens, SystemStatusOptions::default())?;
    let output = parse_output_format(tokens);
    Ok(Command::SystemStatus { options, output })
}

fn parse_system_status_options(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        None => Ok(options),
        Some((flag, rest)) => match flag.to_str() {
            Some("--json" | "--jsonl") => parse_system_status_options(rest, options),
            Some("--emit") => parse_system_status_emit(rest, options),
            Some("--profile") => parse_system_status_profile(rest, options),
            Some("--server") => parse_system_status_server(rest, options),
            Some(other) if other.starts_with('-') => Err(ParseError::InvalidSystemStatusArgument(
                format!("unknown flag {other}"),
            )),
            Some(other) => Err(ParseError::InvalidSystemStatusArgument(format!(
                "unexpected positional argument {other}"
            ))),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "argument is not valid UTF-8".into(),
            )),
        },
    }
}

fn parse_system_status_emit(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some("yaml") => parse_system_status_options(
                remaining,
                SystemStatusOptions {
                    emit_yaml: true,
                    ..options
                },
            ),
            Some("text") => parse_system_status_options(remaining, options),
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument("--emit")),
            Some(other) => Err(ParseError::InvalidSystemStatusArgument(format!(
                "unknown emit mode {other}"
            ))),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "emit mode is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--emit")),
    }
}

fn parse_system_status_profile(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some("quick") => parse_system_status_options(
                remaining,
                SystemStatusOptions {
                    profile: VerifyProfile::Quick,
                    ..options
                },
            ),
            Some("standard") => parse_system_status_options(remaining, options),
            Some("full") => parse_system_status_options(
                remaining,
                SystemStatusOptions {
                    profile: VerifyProfile::Full,
                    ..options
                },
            ),
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument("--profile")),
            Some(other) => Err(ParseError::UnknownProfile(other.into())),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "profile is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--profile")),
    }
}

fn parse_system_status_server(
    args: &[OsString],
    options: SystemStatusOptions,
) -> Result<SystemStatusOptions, ParseError> {
    match args.split_first() {
        Some((raw, remaining)) => match raw.to_str() {
            Some(value) if value.starts_with("--") => Err(ParseError::MissingArgument("--server")),
            Some(value) => parse_server_mode(value).and_then(|server| {
                parse_system_status_options(remaining, SystemStatusOptions { server, ..options })
            }),
            None => Err(ParseError::InvalidSystemStatusArgument(
                "server mode is not valid UTF-8".into(),
            )),
        },
        None => Err(ParseError::MissingArgument("--server")),
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
            None => Err(ParseError::UnexpectedActionListArgument(format!("{raw:?}"))),
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
    // Find the first positional argument (workflow path) by skipping over
    // named flags and their values. Start at index 2 to skip program name and subcommand.
    // This correctly handles:
    //   vb verify workflow.yaml              (workflow at index 2)
    //   vb verify --profile quick workflow.yaml  (workflow at index 4)
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
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

fn parse_cancel(args: &[OsString]) -> Result<Command, ParseError> {
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

fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::BenchRun { workflow, output })
}

fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    let db = named_flag(args, "--db").map(PathBuf::from);
    let output = parse_output_format(args);
    Ok(Command::Doctor { db, output })
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

fn parse_server_mode(raw: &str) -> Result<DurabilityMode, ParseError> {
    match raw {
        "none" => Ok(DurabilityMode::None),
        other => Err(ParseError::UnknownServerMode(other.into())),
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

/// Find the first positional argument (not starting with `--`) starting at `start_idx`.
/// This correctly skips over named flags and their values to locate the workflow path.
fn find_positional(args: &[OsString], start_idx: usize) -> Option<PathBuf> {
    let mut i = start_idx;
    while i < args.len() {
        let arg = args.get(i)?.to_str()?;
        // Skip named flags (starting with `--`) and their values
        if arg.starts_with("--") {
            i = i.saturating_add(2); // Skip flag name and value
        } else {
            // Found a positional argument
            return Some(PathBuf::from(arg));
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
            Self::UnknownServerMode(mode) => {
                write!(
                    formatter,
                    "unknown server mode: {mode} (expected: none; strict and journaled require a backend probe that is not implemented)"
                )
            }
            Self::InvalidStatusArgument(reason) => {
                write!(formatter, "invalid status argument: {reason}")
            }
            Self::InvalidSystemStatusArgument(reason) => {
                write!(formatter, "invalid system status argument: {reason}")
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
            Self::ReasonTooLong => {
                write!(formatter, "reason exceeds maximum length of 256 characters")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActionRegistryMode, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError,
        StepTarget, VerifyProfile, parse_args,
    };
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(|part| OsString::from(*part)).collect()
    }

    #[test]
    fn parse_run_accepts_db_for_journaled_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "journaled",
            "--db",
            "journal-db",
        ]));

        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            ..
        }) = parsed
        {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(input_bin, PathBuf::from("input.bin"));
            assert_eq!(durability, DurabilityMode::Journaled);
            assert_eq!(db, Some(PathBuf::from("journal-db")));
        }
    }

    #[test]
    fn parse_run_compiled_requires_db_for_strict_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run-compiled",
            "workflow.vbir",
            "--input-bin",
            "input.bin",
            "--durability",
            "strict",
        ]));

        assert!(
            matches!(parsed, Err(ParseError::MissingArgument("--db"))),
            "unexpected parse result: {parsed:?}"
        );
    }

    #[test]
    fn parse_run_none_mode_keeps_db_optional() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
        ]));

        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run { durability, db, .. }) = parsed {
            assert_eq!(durability, DurabilityMode::None);
            assert_eq!(db, None);
        }
    }

    #[test]
    fn parse_run_without_step_flags_produces_none_step() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run { step, .. }) = parsed {
            assert!(step.is_none());
        }
    }

    #[test]
    fn parse_run_step_requires_step_input() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
            "--step",
            "0",
        ]));
        assert!(
            matches!(parsed, Err(ParseError::MissingArgument("--step-input"))),
            "unexpected parse result: {parsed:?}"
        );
    }

    #[test]
    fn step_target_holds_step_id_and_path() {
        let target = StepTarget {
            step_id: 5,
            step_input: PathBuf::from("data.bin"),
        };
        assert_eq!(target.step_id, 5);
        assert_eq!(target.step_input, PathBuf::from("data.bin"));
    }

    #[test]
    fn parse_validate_accepts_json_flag() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "validate",
            "workflow.yaml",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Validate { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Validate { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_explain_accepts_jsonl_flag() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "explain",
            "workflow.yaml",
            "--jsonl",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Explain { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Explain { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Jsonl);
        }
    }

    #[test]
    fn parse_compile_includes_output_format() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "compile",
            "workflow.yaml",
            "--emit",
            "ir",
            "--out",
            "output.vbir",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Compile { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Compile {
            workflow,
            emit,
            out,
            output,
        }) = parsed
        {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(emit, EmitTarget::Ir);
            assert_eq!(out, PathBuf::from("output.vbir"));
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_run_with_step_flags() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "none",
            "--step",
            "3",
            "--step-input",
            "step-data.bin",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Run { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Run {
            step: Some(target), ..
        }) = parsed
        {
            assert_eq!(target.step_id, 3);
            assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
        }
    }

    #[test]
    fn parse_compile_rejects_unknown_emit_target_with_exact_variant() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "compile",
            "workflow.yaml",
            "--emit",
            "wasm",
            "--out",
            "output.vbir",
        ]));

        assert!(
            matches!(parsed, Err(ParseError::UnknownEmitTarget(ref t)) if t == "wasm"),
            "expected UnknownEmitTarget(wasm), got {parsed:?}"
        );
    }

    #[test]
    fn parse_run_rejects_unknown_durability_with_exact_variant() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "run",
            "workflow.yaml",
            "--input-bin",
            "input.bin",
            "--durability",
            "ephemeral",
        ]));

        assert!(
            matches!(parsed, Err(ParseError::UnknownDurability(ref m)) if m == "ephemeral"),
            "expected UnknownDurability(ephemeral), got {parsed:?}"
        );
    }

    #[test]
    fn parse_answer_rejects_invalid_step_with_exact_variant() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "answer",
            "run-1",
            "--step",
            "not-a-step",
            "--value-file",
            "value.bin",
            "--db",
            "test-db",
        ]));

        assert!(
            matches!(parsed, Err(ParseError::InvalidStep(ref s)) if s == "not-a-step"),
            "expected InvalidStep(not-a-step), got {parsed:?}"
        );
    }

    #[test]
    fn parse_inspect_includes_output_format() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "inspect",
            "42",
            "--db",
            "test-db",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Inspect { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Inspect {
            run_id, db, output, ..
        }) = parsed
        {
            assert_eq!(run_id, "42");
            assert_eq!(db, PathBuf::from("test-db"));
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_help_command() {
        let parsed = parse_args(&args(&["velvet-ballastics", "help"]));
        assert!(matches!(parsed, Ok(Command::Help)));
    }

    #[test]
    fn parse_version_command() {
        let parsed = parse_args(&args(&["velvet-ballastics", "--version"]));
        assert!(matches!(parsed, Ok(Command::Version)));
    }

    #[test]
    fn parse_agent_context_command() {
        let parsed = parse_args(&args(&["velvet-ballastics", "agent-context"]));
        assert!(matches!(parsed, Ok(Command::AgentContext)));
    }

    #[test]
    fn parse_status_accepts_no_runtime_defaults() {
        let parsed = parse_args(&args(&["velvet-ballastics", "status", "--json"]));
        assert!(matches!(parsed, Ok(Command::Status { .. })));
        if let Ok(Command::Status { options, output }) = parsed {
            assert_eq!(options.active_runs, None);
            assert_eq!(options.queue_depth, None);
            assert_eq!(options.trace_dropped, None);
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_status_accepts_diagnostic_counters() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "status",
            "--active-runs",
            "5",
            "--queue-depth",
            "3",
            "--trace-dropped",
            "0",
        ]));
        assert!(matches!(parsed, Ok(Command::Status { .. })));
        if let Ok(Command::Status { options, output }) = parsed {
            assert_eq!(options.active_runs, Some(5));
            assert_eq!(options.queue_depth, Some(3));
            assert_eq!(options.trace_dropped, Some(0));
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_status_rejects_invalid_numeric_argument() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "status",
            "--queue-depth",
            "many",
        ]));
        assert!(
            matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be a usize"),
            "expected InvalidStatusArgument(--queue-depth must be a usize), got {parsed:?}"
        );
    }

    #[test]
    fn parse_status_rejects_missing_queue_depth_value() {
        let parsed = parse_args(&args(&["velvet-ballastics", "status", "--queue-depth"]));
        assert!(matches!(
            parsed,
            Err(ParseError::MissingArgument("--queue-depth"))
        ));
    }

    #[test]
    fn parse_status_rejects_missing_active_runs_value() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "status",
            "--active-runs",
            "--json",
        ]));
        assert!(matches!(
            parsed,
            Err(ParseError::MissingArgument("--active-runs"))
        ));
    }

    #[test]
    fn parse_status_rejects_missing_trace_dropped_value() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "status",
            "--trace-dropped",
            "--queue-depth",
            "1",
        ]));
        assert!(matches!(
            parsed,
            Err(ParseError::MissingArgument("--trace-dropped"))
        ));
    }

    #[test]
    fn parse_status_rejects_unknown_flag() {
        let parsed = parse_args(&args(&["velvet-ballastics", "status", "--bogus"]));
        assert!(
            matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unknown flag --bogus"),
            "expected InvalidStatusArgument(unknown flag --bogus), got {parsed:?}"
        );
    }

    #[test]
    fn parse_status_rejects_extra_positional_argument() {
        let parsed = parse_args(&args(&["velvet-ballastics", "status", "extra"]));
        assert!(
            matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unexpected positional argument extra"),
            "expected InvalidStatusArgument(unexpected positional argument extra), got {parsed:?}"
        );
    }

    #[test]
    fn parse_status_rejects_out_of_range_queue_depth() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "status",
            "--queue-depth",
            "1025",
        ]));
        assert!(
            matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be <= 1024"),
            "expected InvalidStatusArgument(--queue-depth must be <= 1024), got {parsed:?}"
        );
    }

    #[test]
    fn parse_status_rejects_out_of_range_active_runs() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "status",
            "--active-runs",
            "1025",
        ]));
        assert!(
            matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--active-runs must be <= 1024"),
            "expected InvalidStatusArgument(--active-runs must be <= 1024), got {parsed:?}"
        );
    }

    #[test]
    fn parse_system_status_defaults_to_standard_none_text() {
        let parsed = parse_args(&args(&["velvet-ballastics", "system", "status"]));
        assert!(matches!(parsed, Ok(Command::SystemStatus { .. })));
        if let Ok(Command::SystemStatus { options, output }) = parsed {
            assert_eq!(options.profile, VerifyProfile::Standard);
            assert_eq!(options.server, DurabilityMode::None);
            assert!(!options.emit_yaml);
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_system_status_accepts_profile_server_emit_yaml_and_jsonl() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "system",
            "status",
            "--profile",
            "full",
            "--server",
            "none",
            "--emit",
            "yaml",
            "--jsonl",
        ]));
        assert!(matches!(parsed, Ok(Command::SystemStatus { .. })));
        if let Ok(Command::SystemStatus { options, output }) = parsed {
            assert_eq!(options.profile, VerifyProfile::Full);
            assert_eq!(options.server, DurabilityMode::None);
            assert!(options.emit_yaml);
            assert_eq!(output, OutputFormat::Jsonl);
        }
    }

    #[test]
    fn parse_system_status_rejects_unknown_profile() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "system",
            "status",
            "--profile",
            "deep",
        ]));
        assert!(matches!(parsed, Err(ParseError::UnknownProfile(ref p)) if p == "deep"));
    }

    #[test]
    fn parse_system_status_rejects_unknown_server_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "system",
            "status",
            "--server",
            "remote",
        ]));
        assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref m)) if m == "remote"));
    }

    #[test]
    fn parse_system_status_rejects_unprobed_server_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "system",
            "status",
            "--server",
            "strict",
        ]));
        assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref m)) if m == "strict"));
    }

    #[test]
    fn parse_no_command_returns_error() {
        let parsed = parse_args(&args(&["velvet-ballastics"]));
        assert!(matches!(parsed, Err(ParseError::NoCommand)));
    }

    #[test]
    fn parse_unknown_command_returns_error() {
        let parsed = parse_args(&args(&["velvet-ballastics", "foobar"]));
        assert!(matches!(parsed, Err(ParseError::UnknownCommand(_))));
    }

    #[test]
    fn unknown_command_error_enumerates_valid_commands() {
        let err = ParseError::UnknownCommand(String::from("foobar"));
        let rendered = err.to_string();

        assert!(rendered.contains("expected one of"));
        assert!(rendered.contains("agent-context"));
    }

    #[test]
    fn parse_verify_defaults_to_standard_profile() {
        let parsed = parse_args(&args(&["velvet-ballastics", "verify", "workflow.yaml"]));
        assert!(
            matches!(parsed, Ok(Command::Verify { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Verify {
            workflow,
            profile,
            output,
        }) = parsed
        {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(profile, VerifyProfile::Standard);
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_verify_accepts_quick_profile() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "verify",
            "workflow.yaml",
            "--profile",
            "quick",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Verify { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Verify { profile, .. }) = parsed {
            assert_eq!(profile, VerifyProfile::Quick);
        }
    }

    #[test]
    fn parse_verify_accepts_full_profile_with_json() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "verify",
            "workflow.yaml",
            "--profile",
            "full",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Verify { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Verify {
            profile, output, ..
        }) = parsed
        {
            assert_eq!(profile, VerifyProfile::Full);
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_verify_rejects_unknown_profile() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "verify",
            "workflow.yaml",
            "--profile",
            "thorough",
        ]));
        assert!(
            matches!(parsed, Err(ParseError::UnknownProfile(_))),
            "unexpected parse result: {parsed:?}"
        );
    }

    #[test]
    fn parse_graph_defaults_to_text_output() {
        let parsed = parse_args(&args(&["velvet-ballastics", "graph", "workflow.yaml"]));
        assert!(
            matches!(parsed, Ok(Command::Graph { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Graph { workflow, output }) = parsed {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_graph_accepts_json_flag() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "graph",
            "workflow.yaml",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Graph { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Graph { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_diff_requires_both_run_ids_and_db() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "diff",
            "1",
            "2",
            "--db",
            "test-db",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Diff { .. })),
            "unexpected: {parsed:?}"
        );
        if let Ok(Command::Diff {
            run_a,
            run_b,
            db,
            output,
        }) = parsed
        {
            assert_eq!(run_a, "1");
            assert_eq!(run_b, "2");
            assert_eq!(db, PathBuf::from("test-db"));
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_diff_accepts_json_flag() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "diff",
            "10",
            "20",
            "--db",
            "test-db",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Diff { .. })),
            "unexpected: {parsed:?}"
        );
        if let Ok(Command::Diff { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_diff_requires_db_flag() {
        let parsed = parse_args(&args(&["velvet-ballastics", "diff", "1", "2"]));
        assert!(
            matches!(parsed, Err(ParseError::MissingArgument("--db"))),
            "unexpected: {parsed:?}"
        );
    }

    #[test]
    fn parse_simulate_defaults_to_text_output() {
        let parsed = parse_args(&args(&["velvet-ballastics", "simulate", "workflow.yaml"]));
        assert!(
            matches!(parsed, Ok(Command::Simulate { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Simulate { workflow, output }) = parsed {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_simulate_accepts_json_flag() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "simulate",
            "workflow.yaml",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Simulate { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Simulate { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_simulate_accepts_jsonl_flag() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "simulate",
            "workflow.yaml",
            "--jsonl",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Simulate { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Simulate { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Jsonl);
        }
    }

    #[test]
    fn parse_doctor_without_db_is_stateless_text_mode() {
        let parsed = parse_args(&args(&["velvet-ballastics", "doctor"]));
        assert!(
            matches!(parsed, Ok(Command::Doctor { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Doctor { db, output }) = parsed {
            assert_eq!(db, None);
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_doctor_accepts_optional_db_and_json_output() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "doctor",
            "--db",
            "journal-db",
            "--json",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Doctor { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Doctor { db, output }) = parsed {
            assert_eq!(db, Some(PathBuf::from("journal-db")));
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_action_list_accepts_jsonl_output() {
        let parsed = parse_args(&args(&["velvet-ballastics", "action", "list", "--jsonl"]));
        assert!(
            matches!(parsed, Ok(Command::ActionList { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::ActionList { output, registry }) = parsed {
            assert_eq!(output, OutputFormat::Jsonl);
            assert_eq!(registry, ActionRegistryMode::Registered);
        }
    }

    // --- Cancel command parsing tests ---

    #[test]
    fn parse_cancel_accepts_run_id_and_db() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "cancel",
            "42",
            "--db",
            "journal-db",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Cancel { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) = parsed
        {
            assert_eq!(run_id, "42");
            assert_eq!(db, PathBuf::from("journal-db"));
            assert_eq!(reason, None);
            assert_eq!(output, OutputFormat::Text);
        }
    }

    #[test]
    fn parse_cancel_accepts_reason() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "cancel",
            "42",
            "--db",
            "journal-db",
            "--reason",
            "user request",
        ]));
        assert!(
            matches!(parsed, Ok(Command::Cancel { .. })),
            "unexpected parse result: {parsed:?}"
        );
        if let Ok(Command::Cancel { reason, .. }) = parsed {
            assert_eq!(reason, Some("user request".to_string()));
        }
    }

    #[test]
    fn parse_cancel_accepts_json_output() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "cancel",
            "42",
            "--db",
            "journal-db",
            "--json",
        ]));
        if let Ok(Command::Cancel { output, .. }) = parsed {
            assert_eq!(output, OutputFormat::Json);
        } else {
            assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
        }
    }

    #[test]
    fn parse_cancel_rejects_missing_db() {
        let parsed = parse_args(&args(&["velvet-ballastics", "cancel", "42"]));
        assert!(
            matches!(parsed, Err(ParseError::MissingArgument("--db"))),
            "unexpected: {parsed:?}"
        );
    }

    #[test]
    fn parse_cancel_rejects_reason_longer_than_256_bytes() {
        let long_reason = "a".repeat(257);
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "cancel",
            "42",
            "--db",
            "journal-db",
            "--reason",
            &long_reason,
        ]));
        assert!(
            matches!(parsed, Err(ParseError::ReasonTooLong)),
            "unexpected: {parsed:?}"
        );
    }

    #[test]
    fn parse_cancel_accepts_reason_exactly_256_bytes() {
        let reason = "a".repeat(256);
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "cancel",
            "42",
            "--db",
            "journal-db",
            "--reason",
            &reason,
        ]));
        assert!(
            matches!(parsed, Ok(Command::Cancel { .. })),
            "unexpected: {parsed:?}"
        );
    }
}

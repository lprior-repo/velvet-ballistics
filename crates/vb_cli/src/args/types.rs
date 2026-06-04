//! Domain types for argument parsing.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::commands_journal::{TraceFilters, TraceStatus};

/// Structured output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OutputFormat {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// YAML structured text output (canonical for v1).
    Yaml,
    /// Postcard binary output (canonical machine format for v1).
    Postcard,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EventStatus {
    #[default]
    Pending,
    Active,
    WaitingAnswer,
    Cancelled,
    Completed,
    Failed,
}

impl EventStatus {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::WaitingAnswer => "waiting_answer",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Version,
    AgentContext {
        deliver: Option<String>,
    },
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
        action_name: String,
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
        status: Option<EventStatus>,
        limit: Option<i64>,
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
        filters: TraceFilters,
    },
    Retry {
        run_id: String,
        step: Option<u16>,
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
        slot: u16,
        value: PathBuf,
        db: PathBuf,
        output: OutputFormat,
    },
    Graph {
        workflow: PathBuf,
        output: OutputFormat,
    },
    Diff {
        diff_mode: DiffMode,
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

/// Discriminated union for diff modes — makes illegal states unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiffMode {
    /// Compare workflow expected actions against an actual run's journal events.
    WorkflowAgainst {
        workflow: PathBuf,
        against: String,
        db: PathBuf,
    },
    /// Compare two runs' journal events.
    RunAgainst {
        run_a: String,
        run_b: String,
        db: PathBuf,
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

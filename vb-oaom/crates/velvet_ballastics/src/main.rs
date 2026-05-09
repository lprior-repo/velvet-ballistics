//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]

mod agent_context;
mod args;
mod commands_ai_context;
mod commands_diff;
mod commands_incident;
mod commands_journal;
mod commands_status;
mod commands_verify;
mod commands_workflow;
mod exit_code;

use std::ffi::OsString;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use args::parse_args;
use args::{
    ActionRegistryMode, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError, StepTarget,
    VALID_COMMANDS, VerifyProfile,
};
use exit_code::CliExitCode;
use vb_runtime::action::ActionRegistry;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const INPUT_MAPPING_DECODE_FAILED_MESSAGE: &str = "INPUT_MAPPING_FAILED: input-bin decode failed";
const INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count";
const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "INPUT_MAPPING_FAILED: input slot index out of range";

macro_rules! outln {
    ($($arg:tt)*) => {{
        write_stdout_line(format_args!($($arg)*));
    }};
}

macro_rules! errln {
    ($($arg:tt)*) => {{
        write_stderr_line(format_args!($($arg)*));
    }};
}

const HELP: &str = "\
velvet-ballastics - compiled workflow runtime

commands:
  validate   <workflow.yaml> [--json|--jsonl]          Validate a workflow definition
  verify     <workflow.yaml> [--profile <quick|standard|full>] [--json|--jsonl]  Verify a workflow
  explain    <workflow.yaml> [--json|--jsonl]          Explain validation errors in detail
  compile    <workflow.yaml> --emit <ir|rust|yaml|postcard> --out <file> [--json|--jsonl]  Compile a workflow
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>] [--json|--jsonl]
             [--step <id> --step-input <file>]                                 Run a single step in isolation
  run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>] [--json|--jsonl]
  ipc-serve  --socket <path> --db <path>               Start IPC server
  inspect    <run_id> --db <path> [--json|--jsonl]     Inspect a run
  events     <run_id> --db <path> [--json|--jsonl]     List run events
  replay     <run_id> --db <path> [--json|--jsonl]     Replay a run from journal
  trace      <run_id> --db <path> [--json|--jsonl]     Show step-by-step execution trace
  retry      <run_id> --db <path> [--json|--jsonl]     Retry a failed run from last successful step
  resume     <run_id> --db <path> [--json|--jsonl]     Resume a suspended run
  bench-run  <workflow.yaml> [--json|--jsonl]          Benchmark a workflow
  doctor     --db <path> [--json|--jsonl]              Run diagnostic checks
  answer     <run_id> --step <N> --value-file <file> --db <path> [--json|--jsonl]  Answer a suspended step
  graph      <workflow.yaml> [--json|--jsonl]          Output control flow graph in DOT format
  diff       <run_a> <run_b> --db <path> [--json|--jsonl]  Compare two runs
  incident   <run_id> --db <path> [--json|--jsonl]     Black-box failure report
  submit     <workflow.yaml> --input-bin <file> --db <path> --durability <mode> [--json|--jsonl]  Submit workflow run
  simulate   <workflow.yaml> [--json|--jsonl]     Dry-run workflow without executing actions
  ai-context <run_id> --db <path> [--json|--jsonl]  Emit compact AI context packet for a run
  help                                                Print this message
  version                                              Print version information
";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let parsed = match parse_args(&args) {
        Ok(cmd) => cmd,
        Err(err) => {
            errln!("{err}");
            return CliExitCode::ValidationFailed.into();
        }
    };

    match parsed {
        Command::Help => {
            outln!("{HELP}");
            ExitCode::SUCCESS
        }
        Command::Version => {
            outln!("velvet-ballastics {}", VERSION);
            ExitCode::SUCCESS
        }
        Command::AgentContext => {
            let registry = ActionRegistry::new();
            let ctx = agent_context::build_agent_context(registry);
            let json = serde_json::to_string_pretty(&ctx).expect("agent context must serialize");
            outln!("{json}");
            ExitCode::SUCCESS
        }
        Command::AiContext {
            run_id,
            db,
            output,
        } => commands_ai_context::handle(&run_id, &db, output),
        Command::Status {
            options,
            output,
        } => {
            if output == OutputFormat::Text {
                outln!("status: running");
            } else {
                outln!("{{\"status\": \"running\"}}");
            }
            ExitCode::SUCCESS
        }
        Command::Inspect { run_id, db, output } => {
            outln!("inspect run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Events { run_id, db, output } => {
            outln!("events for run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Replay { run_id, db, output } => {
            outln!("replay run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Trace { run_id, db, output } => {
            outln!("trace run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Retry { run_id, db, output } => {
            outln!("retry run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Resume { run_id, db, output } => {
            outln!("resume run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Incident { run_id, db, output } => {
            outln!("incident report for run: {}", run_id);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Diff {
            run_a,
            run_b,
            db,
            output,
        } => {
            outln!("diff run {} vs run {}", run_a, run_b);
            outln!("db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::ActionList { output, registry } => {
            outln!("action list (registry mode: {:?})", registry);
            ExitCode::SUCCESS
        }
        Command::ActionInspect {
            action_id,
            output,
            registry,
        } => {
            outln!(
                "inspect action {} (registry mode: {:?})",
                action_id, registry
            );
            ExitCode::SUCCESS
        }
        Command::Verify {
            workflow,
            profile,
            output,
        } => {
            outln!(
                "verify workflow: {} (profile: {:?})",
                workflow.display(),
                profile
            );
            ExitCode::SUCCESS
        }
        Command::Validate { workflow, output } => {
            outln!("validate workflow: {}", workflow.display());
            ExitCode::SUCCESS
        }
        Command::Compile {
            workflow,
            emit,
            out,
            output,
        } => {
            outln!(
                "compile workflow: {} -> {} (emit: {:?})",
                workflow.display(),
                out.display(),
                emit
            );
            ExitCode::SUCCESS
        }
        Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
            output,
        } => {
            outln!(
                "run workflow: {} input: {} durability: {:?} db: {:?} step: {:?}",
                workflow.display(),
                input_bin.display(),
                durability,
                db.map(|p| p.display().to_string()),
                step.map(|s| s.step_id.to_string())
            );
            ExitCode::SUCCESS
        }
        Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
            output,
        } => {
            outln!(
                "run-compiled workflow: {} input: {} durability: {:?} db: {:?}",
                workflow.display(),
                input_bin.display(),
                durability,
                db.map(|p| p.display().to_string())
            );
            ExitCode::SUCCESS
        }
        Command::IpcServe { socket, db } => {
            outln!("ipc-serve socket: {} db: {}", socket.display(), db.display());
            ExitCode::SUCCESS
        }
        Command::BenchRun { workflow, output } => {
            outln!("bench-run workflow: {}", workflow.display());
            ExitCode::SUCCESS
        }
        Command::Doctor { db, output } => {
            outln!("doctor db: {}", db.display());
            ExitCode::SUCCESS
        }
        Command::Answer {
            run_id,
            step,
            value_file,
            db,
            output,
        } => {
            outln!(
                "answer run: {} step: {} value-file: {} db: {}",
                run_id,
                step,
                value_file.display(),
                db.display()
            );
            ExitCode::SUCCESS
        }
        Command::Graph { workflow, output } => {
            outln!("graph workflow: {}", workflow.display());
            ExitCode::SUCCESS
        }
        Command::Simulate { workflow, output } => {
            outln!("simulate workflow: {}", workflow.display());
            ExitCode::SUCCESS
        }
        Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        } => {
            outln!(
                "submit workflow: {} input: {} db: {} durability: {:?}",
                workflow.display(),
                input_bin.display(),
                db.display(),
                durability
            );
            ExitCode::SUCCESS
        }
        Command::Explain { workflow, output } => {
            outln!("explain workflow: {}", workflow.display());
            ExitCode::SUCCESS
        }
    }
}

fn write_stdout_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
        .is_err()
    {}
}

//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
  validate   <workflow.yaml>                          Validate a workflow definition
  compile    <workflow.yaml> --emit <ir|rust> --out <file>  Compile a workflow to IR or Rust
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>]  Execute a workflow
             [--step <id> --step-input <file>]                                 Run a single step in isolation
  run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>]  Execute compiled IR
  ipc-serve  --socket <path> --db <path>               Start IPC server
  inspect    <run_id> --db <path>                       Inspect a run
  events     <run_id> --db <path>                       List run events
  replay     <run_id> --db <path>                       Replay a run from journal
  bench-run  <workflow.yaml>                            Benchmark a workflow
  doctor     --db <path>                                Run diagnostic checks
  help                                                Print this message
  version                                             Print version

architecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let parsed = parse_args(&args);

    match parsed {
        Ok(Command::Help) => exit_from_io(&write_help_stdout(), ExitCode::SUCCESS),
        Ok(Command::Version) => exit_from_io(&write_version_stdout(), ExitCode::SUCCESS),
        Ok(Command::Validate { workflow }) => cmd_validate(&workflow),
        Ok(Command::Compile {
            workflow,
            emit,
            out,
        }) => cmd_compile(&workflow, emit, &out),
        Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
        }) => match step {
            Some(target) => cmd_run_step(&workflow, durability, &target),
            None => cmd_run(&workflow, &input_bin, durability, db.as_deref()),
        },
        Ok(Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
        }) => cmd_run_compiled(&workflow, &input_bin, durability, db.as_deref()),
        Ok(Command::IpcServe { socket, db }) => cmd_ipc_serve(&socket, &db),
        Ok(Command::Inspect { run_id, db }) => cmd_inspect(&run_id, &db),
        Ok(Command::Events { run_id, db }) => cmd_events(&run_id, &db),
        Ok(Command::Replay { run_id, db }) => cmd_replay(&run_id, &db),
        Ok(Command::BenchRun { workflow }) => cmd_bench_run(&workflow),
        Ok(Command::Doctor { db }) => cmd_doctor(&db),
        Err(e) => exit_from_io(&write_error_stderr(&e), ExitCode::FAILURE),
    }
}

#[derive(Debug)]
enum Command {
    Help,
    Version,
    Validate {
        workflow: PathBuf,
    },
    Compile {
        workflow: PathBuf,
        emit: EmitTarget,
        out: PathBuf,
    },
    Run {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
        step: Option<StepTarget>,
    },
    RunCompiled {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
    },
    IpcServe {
        socket: PathBuf,
        db: PathBuf,
    },
    Inspect {
        run_id: String,
        db: PathBuf,
    },
    Events {
        run_id: String,
        db: PathBuf,
    },
    Replay {
        run_id: String,
        db: PathBuf,
    },
    BenchRun {
        workflow: PathBuf,
    },
    Doctor {
        db: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitTarget {
    Ir,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DurabilityMode {
    Strict,
    Journaled,
    None,
}

/// Single-step isolation target for `run --step`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StepTarget {
    step_id: u16,
    step_input: PathBuf,
}

#[derive(Debug)]
enum ParseError {
    MissingArgument(&'static str),
    UnknownEmitTarget(String),
    UnknownDurability(String),
    UnknownCommand(String),
    NoCommand,
}

fn parse_args(args: &[OsString]) -> Result<Command, ParseError> {
    let subcommand = args
        .get(1)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "validate" => parse_validate(args),
        "compile" => parse_compile(args),
        "run" => parse_run(args),
        "run-compiled" => parse_run_compiled(args),
        "ipc-serve" => parse_ipc_serve(args),
        "inspect" => parse_inspect(args),
        "events" => parse_events(args),
        "replay" => parse_replay(args),
        "bench-run" => parse_bench_run(args),
        "doctor" => parse_doctor(args),
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

fn parse_validate(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    Ok(Command::Validate { workflow })
}

fn parse_compile(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let emit_raw = named_flag(args, "--emit").ok_or(ParseError::MissingArgument("--emit"))?;
    let emit = match emit_raw.as_str() {
        "ir" => EmitTarget::Ir,
        "rust" => EmitTarget::Rust,
        other => return Err(ParseError::UnknownEmitTarget(other.into())),
    };
    let out = named_flag(args, "--out").ok_or(ParseError::MissingArgument("--out"))?;
    Ok(Command::Compile {
        workflow,
        emit,
        out: PathBuf::from(out),
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
    Ok(Command::Run {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
        step,
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
    Ok(Command::RunCompiled {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
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
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Inspect {
        run_id,
        db: PathBuf::from(db),
    })
}

fn parse_events(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Events {
        run_id,
        db: PathBuf::from(db),
    })
}

fn parse_replay(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Replay {
        run_id,
        db: PathBuf::from(db),
    })
}

fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    Ok(Command::BenchRun { workflow })
}

fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Doctor {
        db: PathBuf::from(db),
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

// --- Helpers for reading files and printing errors ---

fn read_file(path: &std::path::Path) -> Result<Vec<u8>, ExitCode> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            errln!("error reading {}: {e}", path.display());
            Err(ExitCode::FAILURE)
        }
    }
}

fn parse_run_id(raw: &str) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            errln!("invalid run_id '{raw}': {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

// --- Command implementations ---

fn cmd_validate(workflow: &std::path::Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            errln!("file is not valid UTF-8: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Phase 1: strict YAML profile and AST parse via vb_yaml
    match vb_yaml::parse_workflow_source(text) {
        Ok(_ast) => {}
        Err(e) => {
            errln!("YAML parse error: {e}");
            return ExitCode::FAILURE;
        }
    }

    // Phase 2: full compilation pipeline (schema, references, control flow, type/taint)
    match vb_compile::compile_workflow(&bytes) {
        Ok(_compiled) => {}
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    }

    outln!("valid");
    ExitCode::SUCCESS
}

fn cmd_compile(workflow: &std::path::Path, emit: EmitTarget, out: &std::path::Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    };

    match emit {
        EmitTarget::Ir => {
            // Serialize the compiled workflow parts using postcard.
            // WorkflowParts is Serialize+Deserialize; CompiledWorkflow itself is not.
            let parts = compiled.to_parts();
            let encoded = match postcard::to_allocvec(&parts) {
                Ok(data) => data,
                Err(e) => {
                    errln!("IR serialization error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                errln!("error writing {}: {e}", out.display());
                return ExitCode::FAILURE;
            }
            outln!("compiled IR written to {}", out.display());
        }
        EmitTarget::Rust => {
            let source = match vb_codegen::emit_rust_workflow(&compiled) {
                Ok(s) => s,
                Err(e) => {
                    errln!("codegen error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = std::fs::write(out, &source) {
                errln!("error writing {}: {e}", out.display());
                return ExitCode::FAILURE;
            }
            outln!("generated Rust written to {}", out.display());
        }
    }

    ExitCode::SUCCESS
}

fn cmd_run(
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
) -> ExitCode {
    let input_data = match read_file(input_bin) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            errln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db)
}

/// Executes a single step in isolation using `step_once`.
fn cmd_run_step(
    workflow: &std::path::Path,
    durability: DurabilityMode,
    target: &StepTarget,
) -> ExitCode {
    if durability != DurabilityMode::None {
        errln!("step isolation requires --durability none");
        return setup_exit_code();
    }
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let compiled = match compile_bytes(&bytes) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let step_idx = vb_core::StepIdx::new(target.step_id);
    let node = match compiled.node(step_idx) {
        Some(n) => n,
        None => {
            errln!("step {} not found in workflow", target.step_id);
            return setup_exit_code();
        }
    };
    let input_data = match read_file(&target.step_input) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let inputs = match decode_step_inputs(&input_data) {
        Ok(v) => v,
        Err(code) => return code,
    };
    execute_step_isolated(&compiled, step_idx, node, &inputs)
}

fn setup_exit_code() -> ExitCode {
    ExitCode::from(2)
}

fn compile_bytes(bytes: &[u8]) -> Result<vb_core::CompiledWorkflow, ExitCode> {
    match vb_compile::compile_workflow(bytes) {
        Ok(c) => Ok(c),
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            Err(ExitCode::FAILURE)
        }
    }
}

fn decode_step_inputs(data: &[u8]) -> Result<Box<[vb_core::SlotValue]>, ExitCode> {
    if data.is_empty() {
        return Ok(Box::from([]));
    }
    match postcard::from_bytes::<Box<[vb_core::SlotValue]>>(data) {
        Ok(values) => Ok(values),
        Err(e) => {
            errln!("step-input decode error: {e}");
            Err(setup_exit_code())
        }
    }
}

fn execute_step_isolated(
    compiled: &vb_core::CompiledWorkflow,
    step_idx: vb_core::StepIdx,
    node: &vb_core::workflow::CompiledNode,
    inputs: &[vb_core::SlotValue],
) -> ExitCode {
    let mut frame = match build_step_frame(compiled, step_idx) {
        Ok(f) => f,
        Err(code) => return code,
    };
    write_step_inputs(&mut frame, inputs);
    let mut store = vb_core::ValueStore::new();
    let signal = match vb_core::step_once(compiled, &mut frame, &mut store) {
        Ok(s) => s,
        Err(e) => {
            errln!("step error: {e}");
            return ExitCode::FAILURE;
        }
    };
    print_step_result(step_idx, node, &frame, &signal);
    ExitCode::SUCCESS
}

fn build_step_frame(
    compiled: &vb_core::CompiledWorkflow,
    step_idx: vb_core::StepIdx,
) -> Result<vb_core::RunFrame, ExitCode> {
    let step_count = compiled.node_count();
    let slot_count = compiled.slot_count();
    let run_id = vb_core::RunId::new(0);
    match vb_core::RunFrame::new(run_id, step_idx, step_count, slot_count) {
        Ok(frame) => Ok(frame),
        Err(e) => {
            errln!("frame build error: {e}");
            Err(setup_exit_code())
        }
    }
}

fn write_step_inputs(frame: &mut vb_core::RunFrame, inputs: &[vb_core::SlotValue]) {
    for (i, value) in inputs.iter().enumerate() {
        if let Ok(slot) = u16::try_from(i) {
            let slot_idx = vb_core::SlotIdx::new(slot);
            let _ = frame.write_slot(slot_idx, *value);
        }
    }
}

fn print_step_result(
    step: vb_core::StepIdx,
    node: &vb_core::workflow::CompiledNode,
    frame: &vb_core::RunFrame,
    signal: &vb_core::EngineSignal,
) {
    outln!("step: {}", step.get());
    outln!("kind: {}", node_kind_name(&node.kind));
    print_input_slots(frame);
    if let Some(output_slot) = node.output {
        print_output_slot(frame, output_slot);
    }
    outln!("signal: {}", signal_name(signal));
    if let Some(output_slot) = node.output {
        print_taint(frame, output_slot);
    }
}

fn print_input_slots(frame: &vb_core::RunFrame) {
    let count = frame.slot_count();
    for i in 0..count {
        let slot = vb_core::SlotIdx::new(i);
        if let Ok(value) = frame.read_slot(slot) {
            outln!("  slot[{i}]: {value:?}");
        }
    }
}

fn print_output_slot(frame: &vb_core::RunFrame, slot: vb_core::SlotIdx) {
    if let Ok(value) = frame.read_slot(slot) {
        outln!("output: {value:?}");
    }
}

fn print_taint(frame: &vb_core::RunFrame, slot: vb_core::SlotIdx) {
    if let Ok(taint) = frame.read_taint(slot) {
        outln!("taint: {taint:?}");
    }
}

fn node_kind_name(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
    match kind {
        vb_core::workflow::CompiledNodeKind::Nop => "Nop",
        vb_core::workflow::CompiledNodeKind::SetConst { .. } => "SetConst",
        vb_core::workflow::CompiledNodeKind::Copy { .. } => "Copy",
        vb_core::workflow::CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        vb_core::workflow::CompiledNodeKind::BuildObject { .. } => "BuildObject",
        vb_core::workflow::CompiledNodeKind::BuildList { .. } => "BuildList",
        vb_core::workflow::CompiledNodeKind::Do { .. } => "Do",
        vb_core::workflow::CompiledNodeKind::Choose { .. } => "Choose",
        vb_core::workflow::CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        vb_core::workflow::CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        vb_core::workflow::CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        vb_core::workflow::CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        vb_core::workflow::CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        vb_core::workflow::CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        vb_core::workflow::CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        vb_core::workflow::CompiledNodeKind::CollectStart { .. } => "CollectStart",
        vb_core::workflow::CompiledNodeKind::CollectPage { .. } => "CollectPage",
        vb_core::workflow::CompiledNodeKind::CollectNext { .. } => "CollectNext",
        vb_core::workflow::CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        vb_core::workflow::CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        vb_core::workflow::CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        vb_core::workflow::CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        vb_core::workflow::CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        vb_core::workflow::CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        vb_core::workflow::CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        vb_core::workflow::CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        vb_core::workflow::CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        vb_core::workflow::CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        vb_core::workflow::CompiledNodeKind::Ask { .. } => "Ask",
        vb_core::workflow::CompiledNodeKind::AskResume { .. } => "AskResume",
        vb_core::workflow::CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
    }
}

fn signal_name(signal: &vb_core::EngineSignal) -> &'static str {
    match signal {
        vb_core::EngineSignal::Continue => "Continue",
        vb_core::EngineSignal::Finished(_, _) => "Finished",
        vb_core::EngineSignal::StepBudgetExhausted => "StepBudgetExhausted",
        vb_core::EngineSignal::AwaitingAction => "AwaitingAction",
        vb_core::EngineSignal::AwaitingWait => "AwaitingWait",
        vb_core::EngineSignal::AwaitingAsk => "AwaitingAsk",
    }
}

fn cmd_run_compiled(
    vbir_path: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
) -> ExitCode {
    let input_data = match read_file(input_bin) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let ir_bytes = match read_file(vbir_path) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled: vb_core::CompiledWorkflow =
        match postcard::from_bytes::<vb_core::WorkflowParts>(&ir_bytes) {
            Ok(parts) => match vb_core::CompiledWorkflow::try_from_parts(parts) {
                Ok(c) => c,
                Err(e) => {
                    errln!("compiled IR validation error: {e}");
                    return ExitCode::FAILURE;
                }
            },
            Err(e) => {
                errln!("error deserializing compiled IR: {e}");
                return ExitCode::FAILURE;
            }
        };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            errln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMappingError {
    DecodeFailed,
    SlotCountExceeded,
    SlotIndexOutOfRange,
}

impl std::fmt::Display for InputMappingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DecodeFailed => INPUT_MAPPING_DECODE_FAILED_MESSAGE,
            Self::SlotCountExceeded => INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
            Self::SlotIndexOutOfRange => INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
        })
    }
}

fn map_runtime_inputs(
    compiled: &vb_core::CompiledWorkflow,
    input_data: &[u8],
) -> Result<Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>, InputMappingError> {
    if input_data.is_empty() {
        return Ok(Box::from([]));
    }
    let values = postcard::from_bytes::<Box<[vb_core::SlotValue]>>(input_data)
        .map_err(|_| InputMappingError::DecodeFailed)?;
    if values.len() > usize::from(compiled.slot_count()) {
        return Err(InputMappingError::SlotCountExceeded);
    }
    values
        .iter()
        .copied()
        .enumerate()
        .map(|(index, value)| {
            let slot = u16::try_from(index).map_err(|_| InputMappingError::SlotIndexOutOfRange)?;
            Ok((vb_core::SlotIdx::new(slot), value))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn runtime_journal_for_mode(
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, ExitCode> {
    match durability {
        DurabilityMode::None => Ok(vb_runtime::journal::NoopRuntimeJournal::shared()),
        DurabilityMode::Journaled => open_storage_runtime_journal(db, false),
        DurabilityMode::Strict => open_storage_runtime_journal(db, true),
    }
}

fn open_storage_runtime_journal(
    db: Option<&std::path::Path>,
    strict: bool,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, ExitCode> {
    let Some(path) = db else {
        errln!("--db is required when --durability is strict or journaled");
        return Err(ExitCode::FAILURE);
    };
    let journal = match vb_storage::FjallJournal::open(path, None) {
        Ok(journal) => Arc::new(journal),
        Err(e) => {
            errln!("error opening journal at {}: {e}", path.display());
            return Err(ExitCode::FAILURE);
        }
    };
    if strict {
        return Ok(vb_runtime::journal::StorageRuntimeJournal::shared_strict(
            journal,
        ));
    }
    Ok(vb_runtime::journal::StorageRuntimeJournal::shared_journaled(journal))
}

fn run_compiled_workflow(
    compiled: &vb_core::CompiledWorkflow,
    inputs: Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
) -> ExitCode {
    let run_id = vb_core::RunId::new(1);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        errln!("runtime configuration error: shard count must be non-zero");
        return ExitCode::FAILURE;
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let journal = match runtime_journal_for_mode(durability, db) {
        Ok(journal) => journal,
        Err(code) => return code,
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(shard_count, config, journal);

    if let Err(e) = runtime.submit_compiled_with_inputs(run_id, compiled.clone(), inputs) {
        errln!("runtime submit error: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = runtime.tick_all() {
        errln!("runtime tick error: {e}");
        return ExitCode::FAILURE;
    }

    let counters = runtime.counters_snapshot();
    let traces = runtime.drain_trace();
    outln!(
        "run {}: submitted={} completed={} failed={} steps={}",
        run_id.as_u64(),
        counters.runs_submitted,
        counters.runs_completed,
        counters.runs_failed,
        counters.steps_executed
    );
    for trace in &traces {
        print_trace_event(trace);
    }

    if counters.runs_failed != 0 {
        errln!("run failed");
        return ExitCode::FAILURE;
    }
    if counters.runs_completed != 0 {
        outln!("run completed");
    } else {
        outln!("run accepted but not terminal after one runtime tick");
    }

    ExitCode::SUCCESS
}

fn print_trace_event(event: &vb_runtime::trace::TraceEvent) {
    match event {
        vb_runtime::trace::TraceEvent::StepStarted { step, .. } => {
            outln!("  trace: StepStarted step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::StepEnded { step, .. } => {
            outln!("  trace: StepEnded step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::SlotWritten { slot, .. } => {
            outln!("  trace: SlotWritten slot={}", slot.get());
        }
        vb_runtime::trace::TraceEvent::ActionScheduled { step, .. } => {
            outln!("  trace: ActionScheduled step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::ActionCompleted { step, .. } => {
            outln!("  trace: ActionCompleted step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::ActionFailed { step, .. } => {
            outln!("  trace: ActionFailed step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::AskAnswered { step, slot, .. } => {
            outln!(
                "  trace: AskAnswered step={} slot={}",
                step.get(),
                slot.get()
            );
        }
        vb_runtime::trace::TraceEvent::RunSubmitted { .. } => {
            outln!("  trace: RunSubmitted");
        }
        vb_runtime::trace::TraceEvent::RunFinished { .. } => {
            outln!("  trace: RunFinished");
        }
        vb_runtime::trace::TraceEvent::RunFailed { .. } => {
            outln!("  trace: RunFailed");
        }
        vb_runtime::trace::TraceEvent::RunCancelled { .. } => {
            outln!("  trace: RunCancelled");
        }
    }
}

fn cmd_ipc_serve(socket: &std::path::Path, db: &std::path::Path) -> ExitCode {
    // Open the storage journal to validate the path
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };
    let journal = Arc::new(journal);
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&journal),
    };
    let queue =
        match vb_storage::JournalWriterQueue::new(1024, 64, vb_storage::StorageLimits::DEFAULT) {
            Ok(q) => Arc::new(q),
            Err(e) => {
                errln!("error creating journal queue: {e}");
                return ExitCode::FAILURE;
            }
        };
    let runtime_journal =
        vb_runtime::journal::RuntimeJournalConfig::new(vb_storage::DurabilityProfile::Journaled)
            .shared_journal(journal, queue);

    // Create runtime
    let shard_count = match NonZeroUsize::new(1) {
        Some(count) => count,
        None => NonZeroUsize::MIN,
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let mut runtime =
        vb_runtime::runtime::Runtime::new_with_journal(shard_count, config, runtime_journal);

    // Bind the IPC server
    let mut server = match vb_ipc::server::IpcServer::bind(socket) {
        Ok(s) => s,
        Err(e) => {
            errln!("error binding IPC socket at {}: {e}", socket.display());
            return ExitCode::FAILURE;
        }
    };

    outln!("ipc server listening on {}", socket.display());

    // Event loop
    loop {
        match server.poll_once_with_resolver(
            &mut runtime,
            Some(std::time::Duration::from_millis(100)),
            Some(&mut resolver),
        ) {
            Ok(true) => {}
            Ok(false) => {
                outln!("shutdown requested");
                break;
            }
            Err(e) => {
                errln!("ipc server error: {e}");
                return ExitCode::FAILURE;
            }
        }

        // Process pending commands
        match runtime.tick_all() {
            Ok(true) => {}
            Ok(false) => {
                outln!("runtime shut down");
                break;
            }
            Err(e) => {
                errln!("runtime tick error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

struct StorageWorkflowResolver {
    journal: Arc<vb_storage::FjallJournal>,
}

impl vb_ipc::server::WorkflowResolver for StorageWorkflowResolver {
    fn resolve_workflow(
        &mut self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<vb_core::CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
        let record = match self.journal.compiled_ir(digest) {
            Ok(Some(record)) => record,
            Ok(None) => return Err(vb_ipc::server::WorkflowResolutionError::NotFound),
            Err(_) => return Err(vb_ipc::server::WorkflowResolutionError::InvalidArtifact),
        };
        if record.digest != digest {
            return Err(vb_ipc::server::WorkflowResolutionError::InvalidArtifact);
        }
        let parts = postcard::from_bytes::<vb_core::WorkflowParts>(&record.ir)
            .map_err(|_| vb_ipc::server::WorkflowResolutionError::InvalidArtifact)?;
        vb_core::CompiledWorkflow::try_from_parts(parts)
            .map_err(|_| vb_ipc::server::WorkflowResolutionError::InvalidArtifact)
    }
}

fn cmd_inspect(run_id: &str, db: &std::path::Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                outln!("run {run_id}: no events found");
            } else {
                let terminal = events.last();
                let status = match terminal {
                    Some(vb_storage::JournalEvent::RunFinished { .. }) => "finished",
                    Some(vb_storage::JournalEvent::RunFailedEvent { .. }) => "failed",
                    Some(vb_storage::JournalEvent::RunCancelled { .. }) => "cancelled",
                    _ => "running",
                };
                outln!("run {run_id}: status={status}, events={}", events.len());
            }
        }
        Err(e) => {
            errln!("error reading run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn cmd_events(run_id: &str, db: &std::path::Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                outln!("no events found for run {run_id}");
            } else {
                for event in &events {
                    print_event(event);
                }
                outln!("{} event(s) total", events.len());
            }
        }
        Err(e) => {
            errln!("error reading events for run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn print_event(event: &vb_storage::JournalEvent) {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, .. } => {
            outln!("  seq={}: RunAccepted", seq.get());
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            outln!("  seq={}: StepStarted step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            outln!(
                "  seq={}: StepSucceeded step={} output={}",
                seq.get(),
                step.get(),
                output.get()
            );
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionScheduled step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionCompleted step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            outln!(
                "  seq={}: ActionFailed step={} action={}",
                seq.get(),
                step.get(),
                action.get()
            );
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            outln!("  seq={}: SlotWritten slot={}", seq.get(), slot.get());
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: WaitScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: AskScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            outln!("  seq={}: AskAnswered step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            outln!("  seq={}: RetryScheduled step={}", seq.get(), step.get());
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            outln!("  seq={}: RunCancelled", seq.get());
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            outln!("  seq={}: RunFinished result={}", seq.get(), result.get());
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            outln!("  seq={}: RunFailed", seq.get());
        }
    }
}

fn cmd_replay(run_id: &str, db: &std::path::Path) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("error opening journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker) {
        Ok(events) => {
            outln!("recovered {} event(s) for run {run_id}", events.len());
            for event in &events {
                print_event(event);
            }
            match vb_storage::recovery::extract_terminal(&events) {
                Some(terminal) => {
                    outln!("terminal: {}", event_name(terminal));
                }
                None => {
                    outln!("terminal: none");
                }
            }
        }
        Err(e) => {
            errln!("error replaying run {run_id}: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn cmd_bench_run(workflow: &std::path::Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compile_start = Instant::now();
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    };
    let compile_elapsed = compile_start.elapsed();

    let run_start = Instant::now();
    let run_id = vb_core::RunId::new(1);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        errln!("runtime configuration error: shard count must be non-zero");
        return ExitCode::FAILURE;
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let mut runtime = vb_runtime::runtime::Runtime::new(shard_count, config);
    if let Err(e) = runtime.submit_compiled(run_id, compiled) {
        errln!("runtime submit error: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = runtime.tick_all() {
        errln!("runtime tick error: {e}");
        return ExitCode::FAILURE;
    }
    let run_elapsed = run_start.elapsed();
    let counters = runtime.counters_snapshot();

    outln!("compile: {}us", compile_elapsed.as_micros());
    outln!("execute: {}us", run_elapsed.as_micros());
    outln!(
        "total:   {}us",
        compile_elapsed
            .as_micros()
            .saturating_add(run_elapsed.as_micros())
    );
    outln!(
        "runtime: submitted={} completed={} failed={} steps={}",
        counters.runs_submitted,
        counters.runs_completed,
        counters.runs_failed,
        counters.steps_executed
    );

    if counters.runs_failed != 0 {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn cmd_doctor(db: &std::path::Path) -> ExitCode {
    // Check 1: can we open the journal?
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            errln!("FAIL: cannot open journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };
    outln!("OK: journal opened at {}", db.display());

    // Check 2: can we persist?
    match journal.persist_strict() {
        Ok(()) => outln!("OK: strict persist succeeded"),
        Err(e) => {
            errln!("FAIL: strict persist failed: {e}");
            return ExitCode::FAILURE;
        }
    }

    // Check 3: can we write and read back an event?
    let test_run = vb_core::RunId::new(unique_doctor_run_id());
    let test_event = vb_storage::JournalEvent::RunAccepted {
        run: test_run,
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
    };

    if let Err(e) = journal.append_journaled(&test_event) {
        errln!("FAIL: cannot append test event: {e}");
        return ExitCode::FAILURE;
    }
    outln!("OK: journal append succeeded");

    match journal.events_for_run(test_run) {
        Ok(events) => {
            if events.is_empty() {
                errln!("FAIL: test event not found after append");
                return ExitCode::FAILURE;
            }
            outln!("OK: journal read-back returned {} event(s)", events.len());
        }
        Err(e) => {
            errln!("FAIL: cannot read test run events: {e}");
            return ExitCode::FAILURE;
        }
    }

    outln!("doctor: all checks passed");
    ExitCode::SUCCESS
}

fn event_name(event: &vb_storage::JournalEvent) -> &'static str {
    match event {
        vb_storage::JournalEvent::RunAccepted { .. } => "RunAccepted",
        vb_storage::JournalEvent::StepStarted { .. } => "StepStarted",
        vb_storage::JournalEvent::StepSucceeded { .. } => "StepSucceeded",
        vb_storage::JournalEvent::ActionScheduled { .. } => "ActionScheduled",
        vb_storage::JournalEvent::ActionCompletedEvent { .. } => "ActionCompleted",
        vb_storage::JournalEvent::ActionFailedEvent { .. } => "ActionFailed",
        vb_storage::JournalEvent::SlotWrittenEvent { .. } => "SlotWritten",
        vb_storage::JournalEvent::WaitScheduledEvent { .. } => "WaitScheduled",
        vb_storage::JournalEvent::AskScheduledEvent { .. } => "AskScheduled",
        vb_storage::JournalEvent::AskAnsweredEvent { .. } => "AskAnswered",
        vb_storage::JournalEvent::RetryScheduledEvent { .. } => "RetryScheduled",
        vb_storage::JournalEvent::RunCancelled { .. } => "RunCancelled",
        vb_storage::JournalEvent::RunFinished { .. } => "RunFinished",
        vb_storage::JournalEvent::RunFailedEvent { .. } => "RunFailed",
    }
}

fn unique_doctor_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
}

// --- Helpers ---

fn exit_from_io(result: &io::Result<()>, success_code: ExitCode) -> ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => ExitCode::FAILURE,
    }
}

fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballastics {VERSION}")
}

fn write_error_stderr(error: &ParseError) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match error {
        ParseError::MissingArgument(name) => {
            writeln!(handle, "missing argument: {name}\n\n{HELP}")
        }
        ParseError::UnknownEmitTarget(target) => {
            writeln!(
                handle,
                "unknown emit target: {target} (expected: ir, rust)\n\n{HELP}"
            )
        }
        ParseError::UnknownDurability(mode) => {
            writeln!(
                handle,
                "unknown durability mode: {mode} (expected: strict, journaled, none)\n\n{HELP}"
            )
        }
        ParseError::UnknownCommand(cmd) => {
            writeln!(handle, "unknown command: {cmd}\n\n{HELP}")
        }
        ParseError::NoCommand => {
            writeln!(handle, "{HELP}")
        }
    }
}

fn write_stdout_line(args: std::fmt::Arguments<'_>) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match handle.write_fmt(args) {
        Ok(()) | Err(_) => {}
    }
    match handle.write_all(b"\n") {
        Ok(()) | Err(_) => {}
    }
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match handle.write_fmt(args) {
        Ok(()) | Err(_) => {}
    }
    match handle.write_all(b"\n") {
        Ok(()) | Err(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Command, DurabilityMode, INPUT_MAPPING_DECODE_FAILED_MESSAGE,
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE, ParseError, StepTarget, StorageWorkflowResolver,
        build_step_frame, compile_bytes, decode_step_inputs, execute_step_isolated,
        map_runtime_inputs, node_kind_name, parse_args, run_compiled_workflow, signal_name,
        write_step_inputs,
    };
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::Arc;
    use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::ConstValue;
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };
    use vb_storage::{CompiledIrRecord, EventSeq, JournalEvent};

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(|part| OsString::from(*part)).collect()
    }

    fn finish_workflow() -> Option<CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finish"),
            digest: WorkflowDigest::from_bytes([9; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::default(),
        };
        CompiledWorkflow::try_from_parts(parts).ok()
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
    fn input_mapping_failure_message_uses_stable_code() {
        assert_eq!(
            INPUT_MAPPING_DECODE_FAILED_MESSAGE,
            "INPUT_MAPPING_FAILED: input-bin decode failed"
        );
        assert_eq!(
            INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
            "INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count"
        );
    }

    #[test]
    fn map_runtime_inputs_decodes_slot_values() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let values: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::Bool(true)]);
            let payload = postcard::to_allocvec(&values);
            assert!(payload.is_ok(), "test payload should encode: {payload:?}");
            let Ok(payload) = payload else {
                return;
            };
            let mapped = map_runtime_inputs(&compiled, &payload);
            assert!(mapped.is_ok(), "input mapping should decode: {mapped:?}");
            if let Ok(mapped) = mapped {
                assert_eq!(
                    mapped.as_ref(),
                    &[(vb_core::SlotIdx::ZERO, vb_core::SlotValue::Bool(true))]
                );
            }
        }
    }

    #[test]
    fn map_runtime_inputs_rejects_malformed_input_bin() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let mapped = map_runtime_inputs(&compiled, b"not-postcard");
            assert_eq!(
                mapped.map(|_| ()),
                Err(super::InputMappingError::DecodeFailed)
            );
        }
    }

    #[test]
    fn journaled_run_writes_storage_events() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        let dir = tempfile::tempdir();
        assert!(dir.is_ok(), "test directory should be available: {dir:?}");

        if let (Some(compiled), Ok(dir)) = (compiled, dir) {
            let code = run_compiled_workflow(
                &compiled,
                Box::from([]),
                DurabilityMode::Journaled,
                Some(dir.path()),
            );
            assert_eq!(code, std::process::ExitCode::SUCCESS);

            let journal = vb_storage::FjallJournal::open(dir.path(), None);
            assert!(journal.is_ok(), "journal should reopen");
            if let Ok(journal) = journal {
                let run = vb_core::RunId::new(1);
                let events = journal.events_for_run(run);
                assert!(events.is_ok(), "events should be readable: {events:?}");

                if let Ok(events) = events {
                    assert!(events.contains(&JournalEvent::RunAccepted {
                        run,
                        seq: EventSeq::new(0),
                        workflow: WorkflowDigest::from_bytes([9; 32]),
                    }));
                    assert!(events.iter().any(|e| matches!(
                        e,
                        JournalEvent::RunFinished {
                            run: r,
                            result: SlotIdx::ZERO,
                            ..
                        } if *r == run
                    )));
                }
            }
        }
    }

    #[test]
    fn ipc_storage_resolver_loads_compiled_ir_from_journal() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        let dir = tempfile::tempdir();
        assert!(dir.is_ok(), "test directory should be available: {dir:?}");

        if let (Some(compiled), Ok(dir)) = (compiled, dir) {
            let journal = vb_storage::FjallJournal::open(dir.path(), None);
            assert!(journal.is_ok(), "journal should open");
            let Ok(journal) = journal else {
                return;
            };
            let parts = compiled.to_parts();
            let ir = postcard::to_allocvec(&parts);
            assert!(ir.is_ok(), "workflow parts should encode: {ir:?}");
            let Ok(ir) = ir else {
                return;
            };
            let record = CompiledIrRecord {
                digest: compiled.digest(),
                ir,
            };
            assert!(
                journal.put_compiled_ir(&record).is_ok(),
                "put_compiled_ir must succeed"
            );
            let mut resolver = StorageWorkflowResolver {
                journal: Arc::new(journal),
            };

            let resolved = vb_ipc::server::WorkflowResolver::resolve_workflow(
                &mut resolver,
                compiled.digest(),
            );

            assert!(resolved.is_ok(), "resolver should load compiled IR");
            let Ok(resolved) = resolved else {
                return;
            };
            assert_eq!(resolved.digest(), compiled.digest());
        }
    }

    #[test]
    fn ipc_storage_resolver_returns_not_found_for_missing_digest() {
        let dir = tempfile::tempdir();
        assert!(dir.is_ok(), "test directory should be available: {dir:?}");
        if let Ok(dir) = dir {
            let journal = vb_storage::FjallJournal::open(dir.path(), None);
            assert!(journal.is_ok(), "journal should open");
            let Ok(journal) = journal else {
                return;
            };
            let mut resolver = StorageWorkflowResolver {
                journal: Arc::new(journal),
            };

            let result = vb_ipc::server::WorkflowResolver::resolve_workflow(
                &mut resolver,
                WorkflowDigest::from_bytes([5; 32]),
            );

            assert!(
                matches!(
                    result,
                    Err(vb_ipc::server::WorkflowResolutionError::NotFound)
                ),
                "missing digest should return NotFound"
            );
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
    fn node_kind_name_returns_correct_labels() {
        assert_eq!(node_kind_name(&CompiledNodeKind::Nop), "Nop");
        assert_eq!(
            node_kind_name(&CompiledNodeKind::SetConst {
                value: ConstIdx::new(0)
            }),
            "SetConst"
        );
        assert_eq!(
            node_kind_name(&CompiledNodeKind::Finish {
                result: SlotIdx::ZERO
            }),
            "Finish"
        );
    }

    #[test]
    fn signal_name_returns_correct_labels() {
        assert_eq!(signal_name(&vb_core::EngineSignal::Continue), "Continue");
        assert_eq!(
            signal_name(&vb_core::EngineSignal::Finished(
                vb_core::SlotValue::Bool(true),
                vb_core::Taint::Clean
            )),
            "Finished"
        );
    }

    #[test]
    fn decode_step_inputs_empty_data_returns_empty() {
        let result = decode_step_inputs(b"");
        assert!(result.is_ok());
        let values = result.expect("ok");
        assert!(values.is_empty());
    }

    #[test]
    fn decode_step_inputs_invalid_data_returns_error() {
        let result = decode_step_inputs(b"garbage");
        assert!(result.is_err());
    }

    #[test]
    fn write_step_inputs_populates_frame_slots() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let mut frame = build_step_frame(&compiled, StepIdx::ZERO).expect("frame should build");
            let inputs: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::I64(42)]);
            write_step_inputs(&mut frame, &inputs);
            assert_eq!(
                frame.read_slot(SlotIdx::ZERO),
                Ok(&vb_core::SlotValue::I64(42))
            );
        }
    }

    #[test]
    fn execute_step_isolated_set_const_step_succeeds() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let node = compiled.node(StepIdx::ZERO).expect("step 0 must exist");
            let inputs: Box<[vb_core::SlotValue]> = Box::from([]);
            let code = execute_step_isolated(&compiled, StepIdx::ZERO, node, &inputs);
            assert_eq!(code, std::process::ExitCode::SUCCESS);
        }
    }

    #[test]
    fn build_step_frame_out_of_range_returns_error() {
        let compiled = finish_workflow();
        assert!(compiled.is_some(), "test workflow should compile");
        if let Some(compiled) = compiled {
            let result = build_step_frame(&compiled, StepIdx::new(99));
            assert!(result.is_err());
        }
    }
}

//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]

mod args;
mod exit_code;

use std::ffi::OsString;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use args::{Command, DurabilityMode, EmitTarget, OutputFormat, ParseError, StepTarget, VerifyProfile};
use args::parse_args;
use exit_code::CliExitCode;

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
  help                                                Print this message
  version                                             Print version

options:
  --json      Output structured JSON
  --jsonl     Output structured JSON Lines (one object per line)

architecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let parsed = parse_args(&args);

    match parsed {
        Ok(Command::Help) => exit_from_io(&write_help_stdout(), ExitCode::SUCCESS),
        Ok(Command::Version) => exit_from_io(&write_version_stdout(), ExitCode::SUCCESS),
        Ok(Command::Verify {
            workflow,
            profile,
            output,
        }) => cmd_verify(&workflow, profile, output),
        Ok(Command::Validate {
            workflow,
            output: _,
        }) => cmd_validate(&workflow),
        Ok(Command::Explain {
            workflow,
            output: _,
        }) => cmd_explain(&workflow),
        Ok(Command::Compile {
            workflow,
            emit,
            out,
            output,
        }) => cmd_compile(&workflow, emit, &out, output),
        Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
            output,
        }) => match step {
            Some(target) => cmd_run_step(&workflow, durability, &target),
            None => cmd_run(&workflow, &input_bin, durability, db.as_deref(), output),
        },
        Ok(Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
            output,
        }) => cmd_run_compiled(&workflow, &input_bin, durability, db.as_deref(), output),
        Ok(Command::IpcServe { socket, db }) => cmd_ipc_serve(&socket, &db),
        Ok(Command::Inspect { run_id, db, output }) => cmd_inspect(&run_id, &db, output),
        Ok(Command::Events { run_id, db, output }) => cmd_events(&run_id, &db, output),
        Ok(Command::Replay { run_id, db, output }) => cmd_replay(&run_id, &db, output),
        Ok(Command::Trace { run_id, db, output }) => cmd_trace(&run_id, &db, output),
        Ok(Command::Retry { run_id, db, output }) => cmd_retry(&run_id, &db, output),
        Ok(Command::Resume { run_id, db, output }) => cmd_resume(&run_id, &db, output),
        Ok(Command::BenchRun { workflow, output }) => cmd_bench_run(&workflow, output),
        Ok(Command::Doctor { db, output }) => cmd_doctor(&db, output),
        Ok(Command::Answer {
            run_id,
            step,
            value_file,
            db,
            output,
        }) => cmd_answer(&run_id, step, &value_file, &db, output),
        Err(e) => exit_from_io(&write_error_stderr(&e), CliExitCode::ValidationFailed.into()),
    }
}

// --- Helpers for reading files and printing errors ---

fn read_file(path: &std::path::Path) -> Result<Vec<u8>, ExitCode> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            errln!("error reading {}: {e}", path.display());
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

fn parse_run_id(raw: &str) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            errln!("invalid run_id '{raw}': {e}");
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

// --- Command implementations ---

fn cmd_verify(
    workflow: &std::path::Path,
    profile: VerifyProfile,
    output: OutputFormat,
) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            errln!("file is not valid UTF-8: {e}");
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Phase 1: strict YAML parse
    match vb_yaml::parse_workflow_source(text) {
        Ok(_) => {}
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "profile": profile.as_str(),
                        "error": format!("YAML parse error: {e}")
                    }),
                    output,
                );
            } else {
                errln!("YAML parse error: {e}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    }

    // Phase 2: compilation pipeline (schema, references, control flow, type/taint)
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            let error_msgs: Vec<String> = errors.0.iter().map(|err| err.to_string()).collect();
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "profile": profile.as_str(),
                        "error": "compilation failed",
                        "errors": error_msgs
                    }),
                    output,
                );
            } else {
                for err in &errors.0 {
                    errln!("compile error: {err}");
                }
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    let digest = compiled.digest();
    let digest_hex: String = digest.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    let mut checks: Vec<&'static str> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check: YAML parse
    checks.push("yaml_parse");

    // Check: compilation
    checks.push("compilation");

    // Check: IR validation gates
    let parts = compiled.to_parts();
    match vb_validate::shared::validate(&parts) {
        Ok(()) => {
            checks.push("ir_validation");
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "profile": profile.as_str(),
                        "digest": digest_hex,
                        "error": format!("IR validation failed: {e}")
                    }),
                    output,
                );
            } else {
                errln!("IR validation failed: {e}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    }

    // Profile-dependent checks: budget and boundedness (standard and full only)
    if profile == VerifyProfile::Standard || profile == VerifyProfile::Full {
        let entry = compiled.entry();
        let nodes: Vec<vb_core::CompiledNode> = {
            let mut result = Vec::new();
            for i in 0..compiled.node_count() {
                let step = vb_core::StepIdx::new(i);
                if let Some(node) = compiled.node(step) {
                    result.push(node.clone());
                }
            }
            result
        };
        let contract = compiled.resource_contract();
        match vb_core::budget::WholeWorkflowBudget::compute(&nodes, entry, &contract) {
            Ok(_budget) => {
                checks.push("budget_computation");
                let policy = vb_core::budget::BoundednessPolicy::DEFAULT;
                match policy.validate(&_budget) {
                    Ok(()) => {
                        checks.push("boundedness_policy");
                    }
                    Err(e) => {
                        if profile == VerifyProfile::Full {
                            if output != OutputFormat::Text {
                                json_error(
                                    &serde_json::json!({
                                        "success": false,
                                        "profile": profile.as_str(),
                                        "digest": digest_hex,
                                        "error": format!("budget policy violation: {e}")
                                    }),
                                    output,
                                );
                            } else {
                                errln!("budget policy violation: {e}");
                            }
                            return CliExitCode::ValidationFailed.into();
                        }
                        warnings.push(format!("budget policy warning: {e}"));
                        checks.push("boundedness_policy_check");
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("budget computation note: {e}"));
            }
        }
    }

    // Emit certificate
    if output != OutputFormat::Text {
        let warning_strs: Vec<&str> = warnings.iter().map(String::as_str).collect();
        json_out(
            &serde_json::json!({
                "success": true,
                "profile": profile.as_str(),
                "digest": digest_hex,
                "checks": checks,
                "warnings": warning_strs
            }),
            output,
        );
    } else {
        outln!("verification certificate");
        outln!("  digest:  {digest_hex}");
        outln!("  profile: {}", profile.as_str());
        outln!("  checks:  {}", checks.len());
        for check in &checks {
            outln!("    - {check}");
        }
        if !warnings.is_empty() {
            outln!("  warnings: {}", warnings.len());
            for warning in &warnings {
                outln!("    - {warning}");
            }
        }
        outln!("verified");
    }

    ExitCode::SUCCESS
}

fn cmd_validate(workflow: &std::path::Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            errln!("file is not valid UTF-8: {e}");
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Phase 1: strict YAML profile and AST parse via vb_yaml
    match vb_yaml::parse_workflow_source(text) {
        Ok(_ast) => {}
        Err(e) => {
            errln!("YAML parse error: {e}");
            return CliExitCode::ValidationFailed.into();
        }
    }

    // Phase 2: full compilation pipeline (schema, references, control flow, type/taint)
    match vb_compile::compile_workflow(&bytes) {
        Ok(_compiled) => {}
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    }

    outln!("valid");
    ExitCode::SUCCESS
}

fn cmd_compile(
    workflow: &std::path::Path,
    emit: EmitTarget,
    out: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            let error_msgs: Vec<String> = errors.0.iter().map(|err| err.to_string()).collect();
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": "compilation failed",
                        "errors": error_msgs
                    }),
                    output,
                );
            } else {
                for err in &errors.0 {
                    errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
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
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "error": format!("IR serialization error: {e}")
                            }),
                            output,
                        );
                    } else {
                        errln!("IR serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("error writing {}: {e}", out.display())
                        }),
                        output,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                json_out(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "ir"
                    }),
                    output,
                );
            } else {
                outln!("compiled IR written to {}", out.display());
            }
        }
        EmitTarget::Rust => {
            let source = match vb_codegen::emit_rust_workflow(&compiled) {
                Ok(s) => s,
                Err(e) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "error": format!("codegen error: {e}")
                            }),
                            output,
                        );
                    } else {
                        errln!("codegen error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &source) {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("error writing {}: {e}", out.display())
                        }),
                        output,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                json_out(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "rust"
                    }),
                    output,
                );
            } else {
                outln!("generated Rust written to {}", out.display());
            }
        }
        EmitTarget::Yaml => {
            let parts = compiled.to_parts();
            let yaml_str = match serde_yaml::to_string(&parts) {
                Ok(s) => s,
                Err(e) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "error": format!("YAML serialization error: {e}")
                            }),
                            output,
                        );
                    } else {
                        errln!("YAML serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, yaml_str.as_bytes()) {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("error writing {}: {e}", out.display())
                        }),
                        output,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                json_out(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "yaml"
                    }),
                    output,
                );
            } else {
                outln!("compiled YAML written to {}", out.display());
            }
        }
        EmitTarget::Postcard => {
            let parts = compiled.to_parts();
            let encoded = match postcard::to_allocvec(&parts) {
                Ok(data) => data,
                Err(e) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "error": format!("postcard serialization error: {e}")
                            }),
                            output,
                        );
                    } else {
                        errln!("postcard serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("error writing {}: {e}", out.display())
                        }),
                        output,
                    );
                } else {
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                json_out(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "postcard"
                    }),
                    output,
                );
            } else {
                outln!("compiled postcard written to {}", out.display());
            }
        }
    }

    ExitCode::SUCCESS
}

fn cmd_run(
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
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
            let error_msgs: Vec<String> = errors.0.iter().map(|err| err.to_string()).collect();
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": "compilation failed",
                        "errors": error_msgs
                    }),
                    output,
                );
            } else {
                for err in &errors.0 {
                    errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": error.to_string()
                    }),
                    output,
                );
            } else {
                errln!("{error}");
            }
            return CliExitCode::RuntimeFailed.into();
        }
    };

    // run_compiled_workflow outputs directly; for structured output we output a result wrapper
    if output != OutputFormat::Text {
        outln!("{{\"status\": \"running\", \"run_id\": 1}}");
    }
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
    CliExitCode::VerificationFailed.into()
}

fn compile_bytes(bytes: &[u8]) -> Result<vb_core::CompiledWorkflow, ExitCode> {
    match vb_compile::compile_workflow(bytes) {
        Ok(c) => Ok(c),
        Err(errors) => {
            for err in &errors.0 {
                errln!("compile error: {err}");
            }
            Err(CliExitCode::CompileFailed.into())
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
            return CliExitCode::RuntimeFailed.into();
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
            drop(frame.write_slot(slot_idx, *value));
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
    output: OutputFormat,
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
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "error": format!("compiled IR validation error: {e}")
                            }),
                            output,
                        );
                    } else {
                        errln!("compiled IR validation error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            },
            Err(e) => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("error deserializing compiled IR: {e}")
                        }),
                        output,
                    );
                } else {
                    errln!("error deserializing compiled IR: {e}");
                }
                return CliExitCode::CompileFailed.into();
            }
        };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": error.to_string()
                    }),
                    output,
                );
            } else {
                errln!("{error}");
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    if output != OutputFormat::Text {
        outln!("{{\"status\": \"running\", \"run_id\": 1}}");
    }
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
        return Err(CliExitCode::StorageError.into());
    };
    let journal = match vb_storage::FjallJournal::open(path, None) {
        Ok(journal) => Arc::new(journal),
        Err(e) => {
            errln!("error opening journal at {}: {e}", path.display());
            return Err(CliExitCode::StorageError.into());
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
        return CliExitCode::RuntimeFailed.into();
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let journal = match runtime_journal_for_mode(durability, db) {
        Ok(journal) => journal,
        Err(code) => return code,
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(shard_count, config, journal);

    if let Err(e) = runtime.submit_compiled_with_inputs(run_id, compiled.clone(), inputs) {
        errln!("runtime submit error: {e}");
        return CliExitCode::RuntimeFailed.into();
    }
    if let Err(e) = runtime.tick_all() {
        errln!("runtime tick error: {e}");
        return CliExitCode::RuntimeFailed.into();
    }

    let counters = runtime.counters_snapshot();
    let traces = runtime.drain_trace();
    outln!(
        "run {}: submitted={} completed={} failed={} steps={}",
        run_id.get(),
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
        return CliExitCode::RuntimeFailed.into();
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
            return CliExitCode::IpcError.into();
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
                return CliExitCode::IpcError.into();
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
            return CliExitCode::IpcError.into();
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
                return CliExitCode::IpcError.into();
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
                return CliExitCode::IpcError.into();
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

fn cmd_inspect(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_out(
                        &serde_json::json!({
                            "run_id": run_id,
                            "status": "not_found",
                            "events": 0
                        }),
                        output,
                    );
                } else {
                    outln!("run {run_id}: no events found");
                }
            } else {
                let terminal = events.last();
                let status = match terminal {
                    Some(vb_storage::JournalEvent::RunFinished { .. }) => "finished",
                    Some(vb_storage::JournalEvent::RunFailedEvent { .. }) => "failed",
                    Some(vb_storage::JournalEvent::RunCancelled { .. }) => "cancelled",
                    _ => "running",
                };
                if output != OutputFormat::Text {
                    json_out(
                        &serde_json::json!({
                            "run_id": run_id,
                            "status": status,
                            "events": events.len()
                        }),
                        output,
                    );
                } else {
                    outln!("run {run_id}: status={status}, events={}", events.len());
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

fn cmd_events(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_out(
                        &serde_json::json!({
                            "run_id": run_id,
                            "events": [],
                            "total": 0
                        }),
                        output,
                    );
                } else {
                    outln!("no events found for run {run_id}");
                }
            } else {
                match output {
                    OutputFormat::Json => {
                        let event_list: Vec<serde_json::Value> =
                            events.iter().map(event_to_json).collect();
                        json_out(
                            &serde_json::json!({
                                "run_id": run_id,
                                "events": event_list,
                                "total": events.len()
                            }),
                            output,
                        );
                    }
                    OutputFormat::Jsonl => {
                        for event in &events {
                            let json_val = event_to_json(event);
                            outln!("{}", serde_json::to_string(&json_val).unwrap_or_default());
                        }
                        outln!("{{\"total\": {}}}", events.len());
                    }
                    OutputFormat::Text => {
                        for event in &events {
                            print_event(event);
                        }
                        outln!("{} event(s) total", events.len());
                    }
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading events for run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading events for run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
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

/// Convert a journal event to a JSON value for structured output.
fn event_to_json(event: &vb_storage::JournalEvent) -> serde_json::Value {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, run, workflow } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunAccepted",
                "run": run.get(),
                "workflow": format!("{:?}", workflow)
            })
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepStarted",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepSucceeded",
                "step": step.get(),
                "output": output.get()
            })
        }
        vb_storage::JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionScheduled",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionCompleted",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionFailed",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "SlotWritten",
                "slot": slot.get()
            })
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "WaitScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskAnswered",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RetryScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunCancelled"
            })
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunFinished",
                "result": result.get()
            })
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunFailed"
            })
        }
    }
}

fn cmd_replay(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker) {
        Ok(events) => {
            let terminal_name =
                vb_storage::recovery::extract_terminal(&events).map(|e| event_name(e).to_string());

            match output {
                OutputFormat::Json => {
                    let event_list: Vec<serde_json::Value> =
                        events.iter().map(event_to_json).collect();
                    json_out(
                        &serde_json::json!({
                            "run_id": run_id,
                            "recovered": events.len(),
                            "events": event_list,
                            "terminal": terminal_name
                        }),
                        output,
                    );
                }
                OutputFormat::Jsonl => {
                    for event in &events {
                        let json_val = event_to_json(event);
                        outln!("{}", serde_json::to_string(&json_val).unwrap_or_default());
                    }
                    if let Some(term) = terminal_name {
                        outln!("{{\"terminal\": \"{}\"}}", term);
                    } else {
                        outln!("{{\"terminal\": null}}");
                    }
                }
                OutputFormat::Text => {
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
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error replaying run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error replaying run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}


fn cmd_trace(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_out(
                        &serde_json::json!({
                            "run_id": run_id,
                            "trace": [],
                            "total": 0
                        }),
                        output,
                    );
                } else {
                    outln!("no events found for run {run_id}");
                }
            } else {
                match output {
                    OutputFormat::Json => {
                        let trace_entries: Vec<serde_json::Value> =
                            events.iter().map(trace_entry_to_json).collect();
                        json_out(
                            &serde_json::json!({
                                "run_id": run_id,
                                "trace": trace_entries,
                                "total": events.len()
                            }),
                            output,
                        );
                    }
                    OutputFormat::Jsonl => {
                        for event in &events {
                            let json_val = trace_entry_to_json(event);
                            outln!("{}", serde_json::to_string(&json_val).unwrap_or_default());
                        }
                        outln!("{{\"total\": {}}}", events.len());
                    }
                    OutputFormat::Text => {
                        outln!("execution trace for run {run_id}");
                        for (idx, event) in events.iter().enumerate() {
                            print_trace_entry(idx, event);
                        }
                        outln!("{} event(s) total", events.len());
                    }
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading trace for run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading trace for run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    CliExitCode::Success.into()
}

fn print_trace_entry(idx: usize, event: &vb_storage::JournalEvent) {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, .. } => {
            outln!("  [{}] RunAccepted at seq {}", idx, seq.get());
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            outln!("  [{}] StepStarted at step {} (seq {})", idx, step.get(), seq.get());
        }
        vb_storage::JournalEvent::StepSucceeded { seq, step, output, .. } => {
            outln!(
                "  [{}] StepSucceeded at step {} output={} (seq {})",
                idx,
                step.get(),
                output.get(),
                seq.get()
            );
        }
        vb_storage::JournalEvent::ActionScheduled { seq, step, action, .. } => {
            outln!(
                "  [{}] ActionScheduled at step {} action={} (seq {})",
                idx,
                step.get(),
                action.get(),
                seq.get()
            );
        }
        vb_storage::JournalEvent::ActionCompletedEvent { seq, step, action, .. } => {
            outln!(
                "  [{}] ActionCompleted at step {} action={} (seq {})",
                idx,
                step.get(),
                action.get(),
                seq.get()
            );
        }
        vb_storage::JournalEvent::ActionFailedEvent { seq, step, action, .. } => {
            outln!(
                "  [{}] ActionFailed at step {} action={} (seq {})",
                idx,
                step.get(),
                action.get(),
                seq.get()
            );
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            outln!("  [{}] SlotWritten slot={} (seq {})", idx, slot.get(), seq.get());
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            outln!("  [{}] WaitScheduled at step {} (seq {})", idx, step.get(), seq.get());
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            outln!("  [{}] AskScheduled at step {} (seq {})", idx, step.get(), seq.get());
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            outln!("  [{}] AskAnswered at step {} (seq {})", idx, step.get(), seq.get());
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            outln!("  [{}] RetryScheduled at step {} (seq {})", idx, step.get(), seq.get());
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            outln!("  [{}] RunCancelled (seq {})", idx, seq.get());
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            outln!("  [{}] RunFinished result={} (seq {})", idx, result.get(), seq.get());
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            outln!("  [{}] RunFailed (seq {})", idx, seq.get());
        }
    }
}

/// Convert a journal event to a structured trace JSON entry.
fn trace_entry_to_json(event: &vb_storage::JournalEvent) -> serde_json::Value {
    match event {
        vb_storage::JournalEvent::RunAccepted { seq, run, workflow } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunAccepted",
                "run": run.get(),
                "workflow": format!("{:?}", workflow)
            })
        }
        vb_storage::JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepStarted",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::StepSucceeded { seq, step, output, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "StepSucceeded",
                "step": step.get(),
                "output": output.get()
            })
        }
        vb_storage::JournalEvent::ActionScheduled { seq, step, action, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionScheduled",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionCompletedEvent { seq, step, action, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionCompleted",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::ActionFailedEvent { seq, step, action, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "ActionFailed",
                "step": step.get(),
                "action": action.get()
            })
        }
        vb_storage::JournalEvent::SlotWrittenEvent { seq, slot, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "SlotWritten",
                "slot": slot.get()
            })
        }
        vb_storage::JournalEvent::WaitScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "WaitScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::AskScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::AskAnsweredEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "AskAnswered",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RetryScheduledEvent { seq, step, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RetryScheduled",
                "step": step.get()
            })
        }
        vb_storage::JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunCancelled"
            })
        }
        vb_storage::JournalEvent::RunFinished { seq, result, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunFinished",
                "result": result.get()
            })
        }
        vb_storage::JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunFailed"
            })
        }
    }
}

fn cmd_retry(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("run {run_id} not found")
                        }),
                        output,
                    );
                } else {
                    errln!("run {run_id}: no events found");
                }
                return CliExitCode::StorageError.into();
            }

            // Find the last StepSucceeded step and the failure point
            let mut last_successful_step: Option<u16> = None;
            let mut failed_step: Option<u16> = None;
            for event in &events {
                match event {
                    vb_storage::JournalEvent::StepSucceeded { step, .. } => {
                        last_successful_step = Some(step.get());
                    }
                    vb_storage::JournalEvent::ActionFailedEvent { step, .. } => {
                        failed_step = Some(step.get());
                    }
                    vb_storage::JournalEvent::RunFailedEvent { .. } => {
                        // Terminal failure marker
                    }
                    _ => {}
                }
            }

            // Determine terminal status
            let terminal = events.last();
            let is_failed = matches!(
                terminal,
                Some(vb_storage::JournalEvent::RunFailedEvent { .. })
                    | Some(vb_storage::JournalEvent::RunCancelled { .. })
            );

            if !is_failed {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("run {run_id} did not fail (no retry needed)")
                        }),
                        output,
                    );
                } else {
                    errln!("run {run_id} did not fail; retry is not applicable");
                }
                return CliExitCode::ValidationFailed.into();
            }

            let failure_step = failed_step.or(last_successful_step.map(|s| s.saturating_add(1)));
            let resume_step = last_successful_step.map(|s| s.saturating_add(1));

            if output != OutputFormat::Text {
                json_out(
                    &serde_json::json!({
                        "run_id": run_id,
                        "failed_at_step": failure_step,
                        "last_successful_step": last_successful_step,
                        "resume_from_step": resume_step,
                        "events": events.len()
                    }),
                    output,
                );
            } else {
                match (failure_step, last_successful_step) {
                    (Some(fail), Some(last)) => {
                        outln!("Run {run_id} failed at step {fail}. Last successful: step {last}.");
                    }
                    (Some(fail), None) => {
                        outln!("Run {run_id} failed at step {fail}. No successful steps recorded.");
                    }
                    (None, Some(last)) => {
                        outln!("Run {run_id} failed. Last successful: step {last}.");
                    }
                    (None, None) => {
                        outln!("Run {run_id} failed. No step progress recorded.");
                    }
                }
                match resume_step {
                    Some(step) => {
                        outln!("Retry would resume from step {step} with recovered state.");
                    }
                    None => {
                        outln!("Retry would resume from the beginning.");
                    }
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

fn cmd_resume(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("run {run_id} not found")
                        }),
                        output,
                    );
                } else {
                    errln!("run {run_id}: no events found");
                }
                return CliExitCode::StorageError.into();
            }

            // Scan for suspension indicators: WaitScheduled or AskScheduled
            // These indicate the run was suspended at that step.
            let mut suspended_at_step: Option<u16> = None;
            for event in &events {
                match event {
                    vb_storage::JournalEvent::WaitScheduledEvent { step, .. } => {
                        suspended_at_step = Some(step.get());
                    }
                    vb_storage::JournalEvent::AskScheduledEvent { step, .. } => {
                        suspended_at_step = Some(step.get());
                    }
                    _ => {}
                }
            }

            // Check terminal status - the run must not be finished/failed/cancelled
            let terminal = events.last();
            let is_terminal = matches!(
                terminal,
                Some(vb_storage::JournalEvent::RunFinished { .. })
                    | Some(vb_storage::JournalEvent::RunFailedEvent { .. })
                    | Some(vb_storage::JournalEvent::RunCancelled { .. })
            );

            if is_terminal {
                let status = match terminal {
                    Some(vb_storage::JournalEvent::RunFinished { .. }) => "finished",
                    Some(vb_storage::JournalEvent::RunFailedEvent { .. }) => "failed",
                    Some(vb_storage::JournalEvent::RunCancelled { .. }) => "cancelled",
                    _ => "unknown",
                };
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("run {run_id} is {status}, not suspended")
                        }),
                        output,
                    );
                } else {
                    errln!("run {run_id} is {status}, not suspended; resume is not applicable");
                }
                return CliExitCode::ValidationFailed.into();
            }

            let resume_step = suspended_at_step;

            if output != OutputFormat::Text {
                json_out(
                    &serde_json::json!({
                        "run_id": run_id,
                        "suspended_at_step": suspended_at_step,
                        "status": "suspended",
                        "resume_from_step": resume_step,
                        "events": events.len()
                    }),
                    output,
                );
            } else {
                match resume_step {
                    Some(step) => {
                        outln!(
                            "Run {run_id} suspended at step {step}. Resume would continue from step {step} with recovered state."
                        );
                    }
                    None => {
                        outln!("Run {run_id} is active but no explicit suspension point found. Resume would continue from current state.");
                    }
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error reading run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

fn cmd_answer(
    _run_id: &str,
    _step: u16,
    _value_file: &std::path::Path,
    _db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": "answer command not yet implemented"
            }),
            output,
        );
    } else {
        errln!("answer command not yet implemented");
    }
    CliExitCode::RuntimeFailed.into()
}

fn cmd_explain(workflow: &std::path::Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            errln!("error: file is not valid UTF-8: {e}");
            return CliExitCode::ValidationFailed.into();
        }
    };

    if let Err(e) = vb_yaml::parse_workflow_source(text) {
        outln!("YAML Parse Error:");
        outln!("  {e}");
        outln!("");
        outln!("The workflow file contains invalid YAML syntax.");
        return CliExitCode::ValidationFailed.into();
    }

    match vb_compile::compile_workflow(&bytes) {
        Ok(_) => {
            outln!("Workflow is valid. No errors to explain.");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            outln!("Workflow has {} validation error(s):", errors.0.len());
            outln!("");
            for (i, err) in errors.0.iter().enumerate() {
                if i > 0 {
                    outln!("---");
                }
                explain_error(err);
            }
            CliExitCode::ValidationFailed.into()
        }
    }
}

fn explain_error(err: &vb_compile::CompileError) {
    use vb_compile::CompileError;
    match err {
        CompileError::SourceTooLarge { actual, limit } => {
            outln!("Source Too Large");
            outln!("  The workflow YAML source is {actual} bytes, exceeds limit of {limit}.");
        }
        CompileError::EmptySource => {
            outln!("Empty Source");
            outln!("  The workflow file contains no YAML document.");
        }
        CompileError::Parse(e) => {
            outln!("YAML Parse Error");
            outln!("  The YAML parser rejected the document: {e}");
        }
        CompileError::DocumentCount { count } => {
            outln!("Multiple YAML Documents");
            outln!("  Expected exactly one YAML document, but found {count}.");
        }
        CompileError::TopLevelNotMapping => {
            outln!("Invalid Top-Level Structure");
            outln!("  The top-level YAML document must be a mapping.");
        }
        CompileError::NonStringKey { mark } => {
            outln!("Non-String Key");
            outln!("  A mapping key at position {mark:?} is not a string.");
        }
        CompileError::DuplicateKey { key, mark } => {
            outln!("Duplicate Key");
            outln!("  The YAML mapping contains duplicate key '{key}' at {mark:?}.");
        }
        CompileError::AliasForbidden { mark } => {
            outln!("YAML Alias Forbidden");
            outln!("  YAML aliases are not allowed at {mark:?}.");
        }
        CompileError::AnchorForbidden { mark } => {
            outln!("YAML Anchor Forbidden");
            outln!("  YAML anchors are not allowed at {mark:?}.");
        }
        CompileError::MergeKeyForbidden { mark } => {
            outln!("YAML Merge Key Forbidden");
            outln!("  YAML merge keys are not allowed at {mark:?}.");
        }
        CompileError::TagForbidden { mark } => {
            outln!("YAML Tag Forbidden");
            outln!("  YAML tags are not allowed at {mark:?}.");
        }
        CompileError::BadValue => {
            outln!("Invalid YAML Scalar");
            outln!("  A YAML scalar value is malformed.");
        }
        CompileError::FloatForbidden => {
            outln!("Floating-Point Numbers Forbidden");
            outln!("  Floating-point YAML scalars are not allowed.");
        }
        CompileError::DepthLimit { depth, limit } => {
            outln!("Nesting Depth Exceeded");
            outln!("  YAML nesting depth of {depth} exceeds limit of {limit}.");
        }
        CompileError::NodeLimit { limit } => {
            outln!("YAML Node Limit Exceeded");
            outln!("  The workflow exceeds node limit of {limit}.");
        }
        CompileError::SequenceLimit { actual, limit } => {
            outln!("Sequence Too Long");
            outln!("  A sequence has {actual} items, exceeding limit of {limit}.");
        }
        CompileError::MappingLimit { actual, limit } => {
            outln!("Mapping Too Large");
            outln!("  A mapping has {actual} entries, exceeding limit of {limit}.");
        }
        CompileError::ScalarLimit { actual, limit } => {
            outln!("Scalar Too Long");
            outln!("  A scalar is {actual} chars, exceeding limit of {limit}.");
        }
        CompileError::MissingField { field } => {
            outln!("Missing Required Field");
            outln!("  Required workflow field '{field}' is missing.");
        }
        CompileError::UnknownTopLevelField { field } => {
            outln!("Unknown Workflow Field");
            outln!("  '{field}' is not a recognized Velvet workflow field.");
        }
        CompileError::InvalidVersion { actual } => {
            outln!("Invalid Workflow Version");
            outln!("  Found version '{actual}', but Velvet v1 requires 'velvet-ballastics/v1'.");
        }
        CompileError::InvalidTriggerCount { count } => {
            outln!("Invalid Trigger Count");
            outln!("  Workflow must declare exactly one trigger, but found {count}.");
        }
        CompileError::UnknownTriggerKind { trigger } => {
            outln!("Unknown Trigger Kind");
            outln!("  Trigger kind '{trigger}' is not recognized.");
        }
        CompileError::TriggerShape { trigger, expected: _ } => {
            outln!("Invalid Trigger Shape");
            outln!("  Trigger '{trigger}' has the wrong structure.");
        }
        CompileError::UnknownTriggerField { trigger, field } => {
            outln!("Unknown Trigger Field");
            outln!("  Trigger '{trigger}' has unknown field '{field}'.");
        }
        CompileError::MissingTriggerField { trigger, field } => {
            outln!("Missing Trigger Field");
            outln!("  Trigger '{trigger}' is missing required field '{field}'.");
        }
        CompileError::InvalidTriggerField { trigger, field, expected: _ } => {
            outln!("Invalid Trigger Field");
            outln!("  Trigger '{trigger}' field '{field}' is invalid.");
        }
        CompileError::FieldShape { field, expected: _ } => {
            outln!("Invalid Field Shape");
            outln!("  Field '{field}' has the wrong structure.");
        }
        CompileError::UnknownInputSchemaField { field } => {
            outln!("Unknown Input Schema Field");
            outln!("  '{field}' is not a recognized input schema field.");
        }
        CompileError::InvalidInputSchema { field, expected: _ } => {
            outln!("Invalid Input Schema");
            outln!("  Input schema field '{field}' is invalid.");
        }
        CompileError::UnsupportedTopLevelResult => {
            outln!("Unsupported Top-Level Result");
            outln!("  Non-empty top-level result mapping is not supported.");
        }
        CompileError::EmptySteps => {
            outln!("Empty Steps");
            outln!("  Workflow must contain at least one executable step.");
        }
        CompileError::InvalidName { field, value } => {
            outln!("Invalid Name");
            outln!("  '{value}' is not a valid Velvet v1 name for {field}.");
        }
        CompileError::MissingStepId { step } => {
            outln!("Missing Step ID");
            outln!("  Step at index {step} is missing its required 'id' field.");
        }
        CompileError::DuplicateStepId { id } => {
            outln!("Duplicate Step ID");
            outln!("  Step ID '{id}' appears more than once in the workflow.");
        }
        CompileError::StepShape { step } => {
            outln!("Invalid Step Shape");
            outln!("  Step at index {step} must be a YAML mapping.");
        }
        CompileError::UnknownStepField { step, field } => {
            outln!("Unknown Step Field");
            outln!("  Step {step} has unknown field '{field}'.");
        }
        CompileError::UnknownStepPrimitiveField { step, primitive, field } => {
            outln!("Unknown Primitive Field");
            outln!("  Step {step} primitive '{primitive}' has unknown field '{field}'.");
        }
        CompileError::MissingStepPrimitive { step } => {
            outln!("Missing Step Primitive");
            outln!("  Step {step} is missing a primitive action.");
        }
        CompileError::MultipleStepPrimitives { step } => {
            outln!("Multiple Step Primitives");
            outln!("  Step {step} contains multiple primitive fields.");
        }
        CompileError::UnsupportedStepPrimitive { step, primitive } => {
            outln!("Unsupported Step Primitive");
            outln!("  Step {step} primitive '{primitive}' is not supported.");
        }
        CompileError::UnsupportedStepControlField { step, field } => {
            outln!("Unsupported Step Control Field");
            outln!("  Step {step} control field '{field}' is not supported.");
        }
        CompileError::MissingStepField { step, field } => {
            outln!("Missing Step Field");
            outln!("  Step {step} is missing required field '{field}'.");
        }
        CompileError::StepFieldShape { step, field, expected: _ } => {
            outln!("Invalid Step Field Shape");
            outln!("  Step {step} field '{field}' has wrong structure.");
        }
        CompileError::StepIndexOutOfRange { value } => {
            outln!("Step Index Out of Range");
            outln!("  Step index {value} exceeds the u16 representation limit.");
        }
        CompileError::SlotIndexOutOfRange { value } => {
            outln!("Slot Index Out of Range");
            outln!("  Slot index {value} is outside the valid u16 range.");
        }
        CompileError::BranchTargetOutOfRange { value } => {
            outln!("Branch Target Out of Range");
            outln!("  Branch target {value} is outside the valid u16 range.");
        }
        CompileError::BackwardBranchTarget { step, target } => {
            outln!("Backward Branch Target");
            outln!("  Step {step} branches to {target}, but forward branches are required.");
        }
        CompileError::PrimitiveLoweringLimitExceeded { primitive, field, value, limit } => {
            outln!("Primitive Limit Exceeded");
            outln!("  Primitive '{primitive}' field '{field}' value {value} exceeds limit {limit}.");
        }
        CompileError::LastStepMustFinish => {
            outln!("Last Step Must Finish");
            outln!("  The final step in a linear workflow must be a 'finish' step.");
        }
        CompileError::UnsupportedConstantValue { step } => {
            outln!("Unsupported Constant Value");
            outln!("  Step {step} constant value must be a scalar YAML value.");
        }
        CompileError::UnknownReferenceRoot { reference, root } => {
            outln!("Unknown Reference Root");
            outln!("  Reference '{reference}' uses unknown root '{root}'.");
        }
        CompileError::IllegalReference { reference } => {
            outln!("Illegal Reference");
            outln!("  Reference '{reference}' is not allowed in deterministic workflows.");
        }
        CompileError::UnknownReferenceName { kind, reference, name } => {
            outln!("Unknown Reference");
            outln!("  Reference '{reference}' refers to unknown {kind} '{name}'.");
        }
        CompileError::UnsupportedAccessorReference { reference, root, path } => {
            outln!("Unsupported Accessor Reference");
            outln!("  Accessor reference '{reference}' (root: {root}, path: {path}) is not supported.");
        }
        CompileError::UnknownStepTarget { step, target } => {
            outln!("Unknown Step Target");
            outln!("  Step {step} branches to undeclared step index {target}.");
        }
        CompileError::UnreachableStep { step } => {
            outln!("Unreachable Step");
            outln!("  Step {step} cannot be reached from the workflow entry point.");
        }
        CompileError::TypeMismatch { field, expected, found } => {
            outln!("Type Mismatch");
            outln!("  Field '{field}': expected {expected}, but found {found}.");
        }
        CompileError::Workflow(e) => {
            outln!("Workflow IR Validation Error");
            outln!("  {e}");
        }
        CompileError::Validation(e) => {
            explain_validation_error(e);
        }
        _ => {
            outln!("Compilation Error");
            outln!("  {err}");
        }
    }
}

fn explain_validation_error(err: &vb_validate::ValidationError) {
    use vb_validate::ValidationError;
    match err {
        ValidationError::DuplicateKey => {
            outln!("Duplicate Key");
            outln!("  A YAML mapping contains duplicate keys, which is not allowed.");
        }
        ValidationError::ForbiddenYamlFeature => {
            outln!("Forbidden YAML Feature");
            outln!("  The workflow uses a YAML feature that is not allowed in Velvet.");
        }
        ValidationError::UnknownTopLevelField => {
            outln!("Unknown Top-Level Field");
            outln!("  The workflow contains an unrecognized top-level field.");
        }
        ValidationError::UnknownStepField => {
            outln!("Unknown Step Field");
            outln!("  A step contains an unrecognized field.");
        }
        ValidationError::MissingRequiredField { field } => {
            outln!("Missing Required Field");
            outln!("  Required field '{field}' is missing from the workflow.");
        }
        ValidationError::InvalidVersion { version } => {
            outln!("Invalid Version");
            outln!("  Found version '{version}', but Velvet v1 requires 'velvet-ballastics/v1'.");
        }
        ValidationError::InvalidId { id } => {
            outln!("Invalid Identifier");
            outln!("  '{id}' is not a valid Velvet identifier.");
        }
        ValidationError::ReservedId { id } => {
            outln!("Reserved Identifier");
            outln!("  '{id}' is a reserved identifier and cannot be used.");
        }
        ValidationError::DuplicateId { id } => {
            outln!("Duplicate Identifier");
            outln!("  The identifier '{id}' appears more than once.");
        }
        ValidationError::MultipleStepPrimitives => {
            outln!("Multiple Step Primitives");
            outln!("  A step contains multiple primitive actions.");
        }
        ValidationError::MissingStepPrimitive => {
            outln!("Missing Step Primitive");
            outln!("  A step is missing its primitive action.");
        }
        ValidationError::UnknownReference { reference } => {
            outln!("Unknown Reference");
            outln!("  Reference '{reference}' is not declared in the workflow.");
        }
        ValidationError::FutureReference { reference } => {
            outln!("Future Reference");
            outln!("  Reference '{reference}' refers to a step that hasn't been defined yet.");
        }
        ValidationError::SecretNotDeclared { secret } => {
            outln!("Undeclared Secret");
            outln!("  Secret '{secret}' is referenced but not declared in the workflow secrets.");
        }
        ValidationError::DirectRuntimeReference => {
            outln!("Direct Runtime Reference");
            outln!("  References to runtime state are not allowed in this context.");
        }
        ValidationError::InvalidThenTarget => {
            outln!("Invalid Branch Target");
            outln!("  A 'then' branch targets an invalid step.");
        }
        ValidationError::ControlFlowCycle => {
            outln!("Control Flow Cycle");
            outln!("  The workflow contains a cycle in its control flow graph.");
        }
        ValidationError::UnreachableStep { step } => {
            outln!("Unreachable Step");
            outln!("  Step '{step}' cannot be reached from the workflow entry.");
        }
        ValidationError::InvalidChoose => {
            outln!("Invalid Choose");
            outln!("  The 'choose' (conditional) construct is invalid.");
        }
        ValidationError::InvalidForEach => {
            outln!("Invalid ForEach");
            outln!("  The 'for_each' loop construct is invalid.");
        }
        ValidationError::InvalidTogether => {
            outln!("Invalid Together");
            outln!("  The 'together' (parallel) construct is invalid.");
        }
        ValidationError::InvalidCollect => {
            outln!("Invalid Collect");
            outln!("  The 'collect' pagination construct is invalid.");
        }
        ValidationError::InvalidReduce => {
            outln!("Invalid Reduce");
            outln!("  The 'reduce' fold construct is invalid.");
        }
        ValidationError::InvalidRepeat => {
            outln!("Invalid Repeat");
            outln!("  The 'repeat' loop construct is invalid.");
        }
        ValidationError::InvalidWait => {
            outln!("Invalid Wait");
            outln!("  The 'wait' step is invalid.");
        }
        ValidationError::InvalidAsk => {
            outln!("Invalid Ask");
            outln!("  The 'ask' (interaction) step is invalid.");
        }
        ValidationError::InvalidFinish => {
            outln!("Invalid Finish");
            outln!("  The 'finish' step is invalid.");
        }
        ValidationError::InvalidRetry => {
            outln!("Invalid Retry");
            outln!("  The 'retry' construct is invalid.");
        }
        ValidationError::InvalidOnError => {
            outln!("Invalid OnError");
            outln!("  The 'on_error' error handler is invalid.");
        }
        ValidationError::SecretResultLeak => {
            outln!("Secret Result Leak");
            outln!("  A secret value may be exposed in the workflow result.");
        }
        ValidationError::TypeMismatch { expected, found } => {
            outln!("Type Mismatch");
            outln!("  Expected type: {expected}");
            outln!("  Found type: {found}");
        }
        ValidationError::PayloadTooLarge => {
            outln!("Payload Too Large");
            outln!("  The workflow payload exceeds size limits.");
        }
        ValidationError::LimitRequired { resource } => {
            outln!("Limit Required");
            outln!("  Resource '{resource}' requires an explicit limit.");
        }
        ValidationError::LimitExceeded { resource } => {
            outln!("Limit Exceeded");
            outln!("  Resource '{resource}' has exceeded its configured limit.");
        }
        ValidationError::UnsupportedTrigger { trigger } => {
            outln!("Unsupported Trigger");
            outln!("  Trigger type '{trigger}' is not supported.");
        }
        ValidationError::HttpTriggerOutOfCore => {
            outln!("HTTP Trigger Out of Core");
            outln!("  HTTP triggers are not available in the core runtime.");
        }
        ValidationError::ExpressionStackExceeded { declared, limit } => {
            outln!("Expression Stack Exceeded");
            outln!("  Expression stack depth {declared} exceeds limit {limit}.");
        }
        ValidationError::ExpressionStackMismatch { expr_index, declared, computed } => {
            outln!("Expression Stack Mismatch");
            outln!("  Expression {expr_index}: declared {declared} stack slots, computed {computed}.");
        }
        ValidationError::AccessorSlotOutOfRange { accessor_index, slot, slot_count } => {
            outln!("Accessor Slot Out of Range");
            outln!("  Accessor {accessor_index} references slot {slot}, but slot_count is {slot_count}.");
        }
        ValidationError::AccessorPathInvalid { accessor_index, segment_index } => {
            outln!("Accessor Path Invalid");
            outln!("  Accessor {accessor_index} has invalid segment at index {segment_index}.");
        }
        ValidationError::SlotReferenceOutOfRange { slot, slot_count, context } => {
            outln!("Slot Reference Out of Range");
            outln!("  Slot {slot} is out of range (slot_count={slot_count}) in context: {context}.");
        }
        ValidationError::LoopBodyStepOutOfRange { step, node_count, source_node, label: _ } => {
            outln!("Loop Body Step Out of Range");
            outln!("  Step {step}: loop body step out of range (node_count={node_count}, source_node={source_node}).");
        }
        ValidationError::SlotDependencyCycle { slot, chain } => {
            outln!("Slot Dependency Cycle");
            outln!("  Slot {slot} has a dependency cycle: {chain}.");
        }
        ValidationError::NodeKindConstraintViolation { node_index, detail } => {
            outln!("Node Kind Constraint Violation");
            outln!("  Node {node_index}: {detail}.");
        }
        ValidationError::ActionContractMissing { action_id, node_index } => {
            outln!("Action Contract Missing");
            outln!("  Do node {node_index} references action_id {action_id}, which has no contract.");
        }
        ValidationError::ActionContractOrphan { action_id } => {
            outln!("Action Contract Orphan");
            outln!("  Action contract {action_id} has no corresponding Do node.");
        }
        ValidationError::SlotTypeInconsistency { slot } => {
            outln!("Slot Type Inconsistency");
            outln!("  Slot {slot} has writers with incompatible type kinds.");
        }
        ValidationError::NonDeterministicPath { from_node, to_node } => {
            outln!("Non-Deterministic Path");
            outln!("  Path from node {from_node} to {to_node} contains no suspension point.");
        }
    }
}


fn cmd_bench_run(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compile_start = Instant::now();
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            let error_msgs: Vec<String> = errors.0.iter().map(|err| err.to_string()).collect();
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": "compilation failed",
                        "errors": error_msgs
                    }),
                    output,
                );
            } else {
                for err in &errors.0 {
                    errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
        }
    };
    let compile_elapsed = compile_start.elapsed();

    let run_start = Instant::now();
    let run_id = vb_core::RunId::new(1);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": "runtime configuration error: shard count must be non-zero"
                }),
                output,
            );
        } else {
            errln!("runtime configuration error: shard count must be non-zero");
        }
        return CliExitCode::RuntimeFailed.into();
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let mut runtime = vb_runtime::runtime::Runtime::new(shard_count, config);
    if let Err(e) = runtime.submit_compiled(run_id, compiled) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("runtime submit error: {e}")
                }),
                output,
            );
        } else {
            errln!("runtime submit error: {e}");
        }
        return CliExitCode::RuntimeFailed.into();
    }
    if let Err(e) = runtime.tick_all() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("runtime tick error: {e}")
                }),
                output,
            );
        } else {
            errln!("runtime tick error: {e}");
        }
        return CliExitCode::RuntimeFailed.into();
    }
    let run_elapsed = run_start.elapsed();
    let counters = runtime.counters_snapshot();

    let total_us = compile_elapsed
        .as_micros()
        .saturating_add(run_elapsed.as_micros());

    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "success": counters.runs_failed == 0,
                "compile_us": compile_elapsed.as_micros(),
                "execute_us": run_elapsed.as_micros(),
                "total_us": total_us,
                "runtime": {
                    "submitted": counters.runs_submitted,
                    "completed": counters.runs_completed,
                    "failed": counters.runs_failed,
                    "steps": counters.steps_executed
                }
            }),
            output,
        );
    } else {
        outln!("compile: {}us", compile_elapsed.as_micros());
        outln!("execute: {}us", run_elapsed.as_micros());
        outln!("total:   {}us", total_us);
        outln!(
            "runtime: submitted={} completed={} failed={} steps={}",
            counters.runs_submitted,
            counters.runs_completed,
            counters.runs_failed,
            counters.steps_executed
        );
    }

    if counters.runs_failed != 0 {
        return CliExitCode::RuntimeFailed.into();
    }

    ExitCode::SUCCESS
}

fn cmd_doctor(db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let mut checks = Vec::new();
    let _success = true;

    // Check 1: can we open the journal?
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => {
            checks.push(serde_json::json!({
                "check": "open_journal",
                "status": "pass",
                "message": format!("journal opened at {}", db.display())
            }));
            j
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "open_journal",
                "status": "fail",
                "message": format!("cannot open journal at {}: {e}", db.display())
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: cannot open journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    // Check 2: can we persist?
    match journal.persist_strict() {
        Ok(()) => {
            checks.push(serde_json::json!({
                "check": "strict_persist",
                "status": "pass",
                "message": "strict persist succeeded"
            }));
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "strict_persist",
                "status": "fail",
                "message": format!("strict persist failed: {e}")
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: strict persist failed: {e}");
            }
            return CliExitCode::StorageError.into();
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
        checks.push(serde_json::json!({
            "check": "append_event",
            "status": "fail",
            "message": format!("cannot append test event: {e}")
        }));
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "checks": checks
                }),
                output,
            );
        } else {
            errln!("FAIL: cannot append test event: {e}");
        }
        return CliExitCode::StorageError.into();
    }
    checks.push(serde_json::json!({
        "check": "append_event",
        "status": "pass",
        "message": "journal append succeeded"
    }));

    match journal.events_for_run(test_run) {
        Ok(events) => {
            if events.is_empty() {
                checks.push(serde_json::json!({
                    "check": "read_back_event",
                    "status": "fail",
                    "message": "test event not found after append"
                }));
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "checks": checks
                        }),
                        output,
                    );
                } else {
                    errln!("FAIL: test event not found after append");
                }
                return CliExitCode::StorageError.into();
            }
            checks.push(serde_json::json!({
                "check": "read_back_event",
                "status": "pass",
                "message": format!("journal read-back returned {} event(s)", events.len())
            }));
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "read_back_event",
                "status": "fail",
                "message": format!("cannot read test run events: {e}")
            }));
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "checks": checks
                    }),
                    output,
                );
            } else {
                errln!("FAIL: cannot read test run events: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    checks.push(serde_json::json!({
        "check": "all",
        "status": "pass",
        "message": "all checks passed"
    }));

    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "success": true,
                "checks": checks
            }),
            output,
        );
    } else {
        outln!("doctor: all checks passed");
    }
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
        Err(_) => CliExitCode::ValidationFailed.into(),
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
        ParseError::InvalidSlot(slot) => {
            writeln!(handle, "invalid slot: {slot}\n\n{HELP}")
        }
        ParseError::MissingArgument(name) => {
            writeln!(handle, "missing argument: {name}\n\n{HELP}")
        }
        ParseError::UnknownEmitTarget(target) => {
            writeln!(
                handle,
                "unknown emit target: {target} (expected: ir, rust, yaml, postcard)\n\n{HELP}"
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
        ParseError::UnknownProfile(profile) => {
            writeln!(
                handle,
                "unknown verify profile: {profile} (expected: quick, standard, full)\n\n{HELP}"
            )
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

/// Output a JSON value to stdout in the specified format.
fn json_out(value: &serde_json::Value, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json_str = serde_json::to_string_pretty(value).unwrap_or_default();
            outln!("{json_str}");
        }
        OutputFormat::Jsonl => {
            let json_str = serde_json::to_string(value).unwrap_or_default();
            outln!("{json_str}");
        }
        OutputFormat::Text => {
            // Should not be called in text mode, but fallback to pretty JSON
            let json_str = serde_json::to_string_pretty(value).unwrap_or_default();
            outln!("{json_str}");
        }
    }
}

/// Output a JSON error value to stderr in the specified format.
fn json_error(value: &serde_json::Value, format: OutputFormat) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            let json_str = serde_json::to_string(value).unwrap_or_default();
            errln!("{json_str}");
        }
        OutputFormat::Text => {
            // Should not be called in text mode, but fallback to errln
            let json_str = serde_json::to_string_pretty(value).unwrap_or_default();
            errln!("{json_str}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Command, DurabilityMode, INPUT_MAPPING_DECODE_FAILED_MESSAGE,
        INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE, ParseError, StepTarget, StorageWorkflowResolver,
        build_step_frame, decode_step_inputs, execute_step_isolated,
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

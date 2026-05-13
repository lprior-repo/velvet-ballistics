//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]

mod agent_context;
mod args;
mod cli_envelope;
mod cli_postcard;
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
#[cfg(test)]
pub(crate) use commands_ai_context::{RunStatus, redacted_slot_value, suggested_ai_commands};
use exit_code::CliExitCode;
use vb_ipc::client::IpcClient;
use vb_ipc::{IpcCommand, IpcPayload};
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
  cancel     <run_id> --db <path> [--reason <text>] [--json|--jsonl]  Cancel a run
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
  version                                             Print version
  agent-context                                      Emit versioned AI-agent CLI schema
  status     [--active-runs <N>] [--queue-depth <N>] [--trace-dropped <N>] [--json|--jsonl]  Report runtime shard status
  action list [--json|--jsonl]                       List registered action contracts
  action inspect <action_id> [--json|--jsonl]         Show one registered action contract

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
        Ok(Command::AgentContext) => cmd_agent_context(),
        Ok(Command::AiContext { run_id, db, output }) => {
            commands_ai_context::handle(&run_id, &db, output)
        }
        Ok(Command::Status { options, output }) => cmd_status(options, output),
        Ok(Command::ActionList { output, registry }) => cmd_action_list(output, registry),
        Ok(Command::ActionInspect {
            action_id,
            output,
            registry,
        }) => cmd_action_inspect(action_id, output, registry),
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
        Ok(Command::Graph { workflow, output }) => cmd_graph(&workflow, output),
        Ok(Command::Diff {
            run_a,
            run_b,
            db,
            output,
        }) => cmd_diff(&run_a, &run_b, &db, output),
        Ok(Command::Incident { run_id, db, output }) => cmd_incident(&run_id, &db, output),
        Ok(Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        }) => cmd_submit(&workflow, &input_bin, &db, durability, output),
        Ok(Command::Simulate { workflow, output }) => cmd_simulate(&workflow, output),
        Ok(Command::Cancel {
            run_id,
            db,
            reason,
            output,
        }) => cmd_cancel(&run_id, &db, reason, output),
        Err(e) => exit_from_io(
            &write_error_stderr(&e),
            CliExitCode::ValidationFailed.into(),
        ),
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

fn report_storage_open_error(
    e: &vb_storage::JournalError,
    db: &std::path::Path,
    output: OutputFormat,
) {
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
}

fn read_journal_events(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<Vec<vb_storage::JournalEvent>, ExitCode> {
    let rid = parse_run_id(run_id)?;
    let journal = vb_storage::FjallJournal::open(db, None).map_err(|e| -> ExitCode {
        report_storage_open_error(&e, db, output);
        CliExitCode::StorageError.into()
    })?;
    journal.events_for_run(rid).map_err(|e| {
        if output != OutputFormat::Text {
            json_error(&serde_json::json!({ "success": false, "error": format!("error reading run {run_id}: {e}") }), output);
        } else { errln!("error reading run {run_id}: {e}"); }
        CliExitCode::StorageError.into()
    })
}

// --- Command implementations ---

fn cmd_agent_context() -> ExitCode {
    let context = agent_context::build(VERSION);
    json_out(&context, OutputFormat::Json);
    ExitCode::SUCCESS
}

fn cmd_status(options: args::StatusOptions, output: OutputFormat) -> ExitCode {
    let status = commands_status::build_status(options);
    if options.emit_yaml {
        commands_status::print_status_yaml(&status);
    } else {
        commands_status::print_status(&status, output);
    }
    ExitCode::SUCCESS
}

fn cmd_action_list(output: OutputFormat, registry_mode: ActionRegistryMode) -> ExitCode {
    match registry_mode {
        ActionRegistryMode::Registered => match registered_cli_actions() {
            Ok(registry) => write_action_registry(&registry, output),
            Err(error) => write_action_registry_error(&error, output),
        },
        ActionRegistryMode::Empty => {
            let registry = ActionRegistry::new();
            write_action_registry(&registry, output)
        }
        ActionRegistryMode::Uninitialized => {
            write_action_registry_uninitialized(output);
            CliExitCode::ValidationFailed.into()
        }
    }
}

fn cmd_action_inspect(
    action_id: u16,
    output: OutputFormat,
    registry_mode: ActionRegistryMode,
) -> ExitCode {
    match registry_mode {
        ActionRegistryMode::Registered => match registered_cli_actions() {
            Ok(registry) => write_action_inspect(&registry, action_id, output),
            Err(error) => write_action_registry_error(&error, output),
        },
        ActionRegistryMode::Empty => {
            let registry = ActionRegistry::new();
            write_action_inspect(&registry, action_id, output)
        }
        ActionRegistryMode::Uninitialized => {
            write_action_registry_uninitialized(output);
            CliExitCode::ValidationFailed.into()
        }
    }
}

fn write_action_registry_error(
    error: &vb_core::action::ActionError,
    output: OutputFormat,
) -> ExitCode {
    let message = format!("failed to register CLI action contracts: {error}");
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            output,
        );
    }
    CliExitCode::ValidationFailed.into()
}

fn write_action_registry_uninitialized(output: OutputFormat) {
    let message = "action registry is not initialized";
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            output,
        );
    }
}

fn write_action_registry(registry: &ActionRegistry, output: OutputFormat) -> ExitCode {
    let rows = action_table_rows(registry);
    if rows.is_empty() {
        write_no_registered_actions(output);
        return ExitCode::SUCCESS;
    }

    if output == OutputFormat::Text {
        write_action_table_rows(&rows);
        return ExitCode::SUCCESS;
    }

    let actions: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "idempotency": row.idempotency,
                "retry_safety": row.retry_safety,
                "side_effect": row.side_effect,
                "input_slot_count": row.input_slot_count,
                "output_slot_count": row.output_slot_count,
                "timeout_ms": row.timeout_ms,
            })
        })
        .collect();
    json_out(
        &serde_json::json!({
            "success": true,
            "actions": actions,
        }),
        output,
    );
    ExitCode::SUCCESS
}

fn write_action_inspect(
    registry: &ActionRegistry,
    action_id: u16,
    output: OutputFormat,
) -> ExitCode {
    match registry.resolve_compile_time(vb_core::ActionId::new(action_id)) {
        Ok(contract) => write_action_contract(contract, output),
        Err(error) => write_action_inspect_error(action_id, &error, output),
    }
}

fn write_action_inspect_error(
    action_id: u16,
    error: &vb_core::action::ActionError,
    output: OutputFormat,
) -> ExitCode {
    let message = format!("action {action_id} is not registered: {error}");
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "action_id": action_id,
                "error": message,
            }),
            output,
        );
    }
    CliExitCode::ValidationFailed.into()
}

fn write_action_contract(
    contract: &vb_core::action::ActionContract,
    output: OutputFormat,
) -> ExitCode {
    let detail = action_contract_detail(contract);
    if output == OutputFormat::Text {
        write_action_contract_text(&detail);
    } else {
        json_out(&detail.to_json(), output);
    }
    ExitCode::SUCCESS
}

fn write_action_contract_text(detail: &ActionContractDetail) {
    outln!("action {}", detail.id);
    outln!("  input_slot_count: {}", detail.input_slot_count);
    outln!("  output_slot_count: {}", detail.output_slot_count);
    outln!("  max_input_bytes: {}", detail.max_input_bytes);
    outln!("  max_output_bytes: {}", detail.max_output_bytes);
    outln!("  timeout_ms: {}", detail.timeout_ms);
    outln!("  idempotency: {}", detail.idempotency);
    outln!("  retry_safety: {}", detail.retry_safety);
    outln!("  side_effect: {}", detail.side_effect);
    outln!("  idempotency_rule: {}", detail.idempotency_rule);
    outln!(
        "  required_capabilities: {}",
        detail.required_capabilities.join(",")
    );
    outln!("  failure_codes: {}", detail.failure_codes.join(","));
    outln!("  example_input_schema: {}", detail.example_input_schema);
    outln!("  example_output_schema: {}", detail.example_output_schema);
}

#[derive(Debug, Clone)]
struct ActionContractDetail {
    id: u16,
    input_slot_count: u16,
    output_slot_count: u16,
    max_input_bytes: u32,
    max_output_bytes: u32,
    timeout_ms: u64,
    idempotency: &'static str,
    retry_safety: &'static str,
    side_effect: &'static str,
    required_capabilities: Vec<String>,
    failure_codes: Vec<&'static str>,
    idempotency_rule: &'static str,
    example_input_schema: &'static str,
    example_output_schema: &'static str,
}

impl ActionContractDetail {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": true,
            "action": {
                "id": self.id,
                "input_slot_count": self.input_slot_count,
                "output_slot_count": self.output_slot_count,
                "max_input_bytes": self.max_input_bytes,
                "max_output_bytes": self.max_output_bytes,
                "timeout_ms": self.timeout_ms,
                "idempotency": self.idempotency,
                "retry_safety": self.retry_safety,
                "side_effect": self.side_effect,
                "required_capabilities": self.required_capabilities,
                "failure_codes": self.failure_codes,
                "idempotency_rule": self.idempotency_rule,
                "example_input_schema": self.example_input_schema,
                "example_output_schema": self.example_output_schema,
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionTableRow {
    id: u16,
    idempotency: &'static str,
    retry_safety: &'static str,
    side_effect: &'static str,
    input_slot_count: u16,
    output_slot_count: u16,
    timeout_ms: u64,
}

fn action_table_rows(registry: &ActionRegistry) -> Vec<ActionTableRow> {
    registry
        .registered_contracts()
        .iter()
        .map(|contract| ActionTableRow {
            id: contract.id.get(),
            idempotency: action_idempotency_name(contract.idempotency),
            retry_safety: action_retry_safety_name(contract.retry_safety),
            side_effect: action_side_effect_name(contract.side_effect),
            input_slot_count: contract.input_slot_count,
            output_slot_count: contract.output_slot_count,
            timeout_ms: contract.timeout_ms,
        })
        .collect()
}

fn action_contract_detail(contract: &vb_core::action::ActionContract) -> ActionContractDetail {
    ActionContractDetail {
        id: contract.id.get(),
        input_slot_count: contract.input_slot_count,
        output_slot_count: contract.output_slot_count,
        max_input_bytes: contract.max_input_bytes,
        max_output_bytes: contract.max_output_bytes,
        timeout_ms: contract.timeout_ms,
        idempotency: action_idempotency_name(contract.idempotency),
        retry_safety: action_retry_safety_name(contract.retry_safety),
        side_effect: action_side_effect_name(contract.side_effect),
        required_capabilities: contract
            .required_capabilities
            .iter()
            .map(|capability| format!("{}:{}", capability.name(), capability.action_id().get()))
            .collect(),
        failure_codes: action_failure_code_names().to_vec(),
        idempotency_rule: action_idempotency_rule(contract.idempotency, contract.retry_safety),
        example_input_schema: "postcard(ActionInput { run, step, action, input, ticket })",
        example_output_schema: "postcard(ActionOutcome::Ready|Suspended|Failed)",
    }
}

fn write_action_table_rows(rows: &[ActionTableRow]) {
    outln!("id\tidempotency\tretry_safety\tside_effect\tinput_slots\toutput_slots\ttimeout_ms");
    rows.iter().for_each(|row| {
        outln!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.id,
            row.idempotency,
            row.retry_safety,
            row.side_effect,
            row.input_slot_count,
            row.output_slot_count,
            row.timeout_ms
        );
    });
}

fn write_no_registered_actions(output: OutputFormat) {
    let message = "no registered actions";
    if output == OutputFormat::Text {
        outln!("{message}");
    } else {
        json_out(
            &serde_json::json!({
                "success": true,
                "actions": [],
                "message": message,
            }),
            output,
        );
    }
}

fn registered_cli_actions() -> vb_core::action::ActionResult<ActionRegistry> {
    cli_action_specs()
        .iter()
        .try_fold(ActionRegistry::new(), |mut registry, spec| {
            registry.register(action_contract(*spec))?;
            Ok(registry)
        })
}

#[derive(Debug, Clone, Copy)]
struct CliActionSpec {
    id: u16,
    idempotency: vb_core::action::Idempotency,
    retry_safety: vb_core::action::RetrySafety,
    side_effect: vb_core::action::SideEffect,
    input_slot_count: u16,
    output_slot_count: u16,
    timeout_ms: u64,
}

fn cli_action_specs() -> &'static [CliActionSpec] {
    &[
        CliActionSpec {
            id: 1,
            idempotency: vb_core::action::Idempotency::DeterministicPure,
            retry_safety: vb_core::action::RetrySafety::Safe,
            side_effect: vb_core::action::SideEffect::None,
            input_slot_count: 1,
            output_slot_count: 1,
            timeout_ms: 1_000,
        },
        CliActionSpec {
            id: 2,
            idempotency: vb_core::action::Idempotency::IdempotentExternal,
            retry_safety: vb_core::action::RetrySafety::KeyRequired,
            side_effect: vb_core::action::SideEffect::Writes,
            input_slot_count: 2,
            output_slot_count: 1,
            timeout_ms: 5_000,
        },
        CliActionSpec {
            id: 3,
            idempotency: vb_core::action::Idempotency::AtLeastOnceExternal,
            retry_safety: vb_core::action::RetrySafety::Unsafe,
            side_effect: vb_core::action::SideEffect::Sends,
            input_slot_count: 1,
            output_slot_count: 0,
            timeout_ms: 10_000,
        },
    ]
}

fn action_contract(spec: CliActionSpec) -> vb_core::action::ActionContract {
    vb_core::action::ActionContract {
        id: vb_core::ActionId::new(spec.id),
        input_slot_count: spec.input_slot_count,
        output_slot_count: spec.output_slot_count,
        max_input_bytes: 65_536,
        max_output_bytes: 65_536,
        timeout_ms: spec.timeout_ms,
        idempotency: spec.idempotency,
        side_effect: spec.side_effect,
        retry_safety: spec.retry_safety,
        required_capabilities: Box::new([]),
    }
}

fn action_idempotency_name(value: vb_core::action::Idempotency) -> &'static str {
    match value {
        vb_core::action::Idempotency::DeterministicPure => "deterministic_pure",
        vb_core::action::Idempotency::IdempotentExternal => "idempotent_external",
        vb_core::action::Idempotency::AtLeastOnceExternal => "at_least_once_external",
    }
}

fn action_retry_safety_name(value: vb_core::action::RetrySafety) -> &'static str {
    match value {
        vb_core::action::RetrySafety::Safe => "safe",
        vb_core::action::RetrySafety::KeyRequired => "key_required",
        vb_core::action::RetrySafety::Unsafe => "unsafe",
    }
}

fn action_side_effect_name(value: vb_core::action::SideEffect) -> &'static str {
    match value {
        vb_core::action::SideEffect::None => "none",
        vb_core::action::SideEffect::Writes => "writes",
        vb_core::action::SideEffect::Sends => "sends",
        vb_core::action::SideEffect::Creates => "creates",
        vb_core::action::SideEffect::Destroys => "destroys",
    }
}

fn action_failure_code_names() -> &'static [&'static str] {
    &[
        "rejected",
        "timeout",
        "rate_limited",
        "resource_exhausted",
        "external_unavailable",
        "invalid_input",
        "permission_denied",
        "conflict",
        "unknown",
    ]
}

fn action_idempotency_rule(
    idempotency: vb_core::action::Idempotency,
    retry_safety: vb_core::action::RetrySafety,
) -> &'static str {
    match (idempotency, retry_safety) {
        (vb_core::action::Idempotency::DeterministicPure, _) => {
            "pure deterministic actions may replay without an external key"
        }
        (_, vb_core::action::RetrySafety::KeyRequired) => {
            "external retries require a stable idempotency key"
        }
        (_, vb_core::action::RetrySafety::Unsafe) => {
            "unsafe actions must not be retried automatically"
        }
        _ => "retry behavior follows the action contract",
    }
}

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

    match commands_verify::run_verification(text, &bytes, profile) {
        Ok(result) => {
            if output != OutputFormat::Text {
                let warning_strs: Vec<&str> = result.warnings.iter().map(String::as_str).collect();
                json_out(
                    &serde_json::json!({
                        "success": true,
                        "profile": profile.as_str(),
                        "digest": result.digest_hex,
                        "checks": result.checks,
                        "warnings": warning_strs
                    }),
                    output,
                );
            } else {
                outln!("verification certificate");
                outln!("  digest:  {}", result.digest_hex);
                outln!("  profile: {}", profile.as_str());
                outln!("  checks:  {}", result.checks.len());
                for check in &result.checks {
                    outln!("    - {check}");
                }
                if !result.warnings.is_empty() {
                    outln!("  warnings: {}", result.warnings.len());
                    for warning in &result.warnings {
                        outln!("    - {warning}");
                    }
                }
                outln!("verified");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = commands_verify::exit_code_for_error(&err);
            match &err {
                commands_verify::VerifyError::YamlParse(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::Compile(errors) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": "compilation failed",
                                "errors": errors
                            }),
                            output,
                        );
                    } else {
                        for e in errors {
                            errln!("compile error: {e}");
                        }
                    }
                }
                commands_verify::VerifyError::IrValidation(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::BudgetPolicy(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::StorageError(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::ReplayDivergence(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
            }
            code.into()
        }
    }
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
            let yaml_str = match serde_saphyr::to_string(&parts) {
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

    match durability {
        DurabilityMode::None => {}
        _ => {
            if let Err(code) = store_workflow_artifacts(&compiled, &bytes, db, output) {
                return code;
            }
        }
    }

    // run_compiled_workflow outputs directly; for structured output we output a result wrapper
    if output != OutputFormat::Text {
        outln!("{{\"status\": \"running\", \"run_id\": 1}}");
    }
    run_compiled_workflow(&compiled, inputs, durability, db)
}

fn store_workflow_artifacts(
    compiled: &vb_core::CompiledWorkflow,
    source: &[u8],
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    let Some(db) = db else {
        return Ok(());
    };
    let parts = compiled.to_parts();
    let ir = match postcard::to_allocvec(&parts) {
        Ok(ir) => ir,
        Err(e) => {
            report_compiled_ir_store_error(format_args!("compiled IR encode error: {e}"), output);
            return Err(CliExitCode::StorageError.into());
        }
    };
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(e) => {
            report_compiled_ir_store_error(
                format_args!("error opening journal at {}: {e}", db.display()),
                output,
            );
            return Err(CliExitCode::StorageError.into());
        }
    };
    let source_record = vb_storage::WorkflowSourceRecord {
        digest: compiled.digest(),
        source: source.to_vec(),
    };
    if let Err(e) = journal.put_workflow_source(&source_record) {
        report_compiled_ir_store_error(format_args!("workflow source write error: {e}"), output);
        return Err(CliExitCode::StorageError.into());
    }
    let record = vb_storage::CompiledIrRecord {
        digest: compiled.digest(),
        ir,
    };
    journal.put_compiled_ir(&record).map_err(|e| {
        report_compiled_ir_store_error(format_args!("compiled IR write error: {e}"), output);
        CliExitCode::StorageError.into()
    })
}

fn report_compiled_ir_store_error(args: std::fmt::Arguments<'_>, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({"success": false, "error": args.to_string()}),
            output,
        );
    } else {
        errln!("{args}");
    }
}

fn cmd_submit(
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    db: &std::path::Path,
    durability: DurabilityMode,
    output: OutputFormat,
) -> ExitCode {
    let _input_data = match read_file(input_bin) {
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

    let digest = compiled.digest();
    let digest_hex: String = digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let step_count = compiled.node_count();

    // Generate run_id from timestamp
    let run_id_num = generate_submit_run_id();
    let run_id = vb_core::RunId::new(run_id_num);

    // Open storage journal and record workflow source + run header
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

    // Store the workflow source
    let source_record = vb_storage::WorkflowSourceRecord {
        digest,
        source: bytes,
    };
    if let Err(e) = vb_storage::put_workflow_source(&journal, &source_record) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("workflow source write error: {e}")
                }),
                output,
            );
        } else {
            errln!("workflow source write error: {e}");
        }
        return CliExitCode::StorageError.into();
    }

    // Record the run header
    let accepted_at_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis().try_into().unwrap_or(0),
        Err(_) => 0,
    };
    let header = vb_storage::RunHeaderRecord {
        run: run_id,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms,
    };
    if let Err(e) = vb_storage::put_run_header(&journal, &header) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("run header write error: {e}")
                }),
                output,
            );
        } else {
            errln!("run header write error: {e}");
        }
        return CliExitCode::StorageError.into();
    }
    // Also record submission for durability-aware runbooks before releasing the metadata journal.
    if durability != DurabilityMode::None {
        let event = vb_storage::JournalEvent::RunAccepted {
            run: run_id,
            seq: vb_storage::EventSeq::new(0),
            workflow: digest,
        };
        if let Err(e) = journal.append_strict_batch(&[event]) {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("journal append error: {e}")
                    }),
                    output,
                );
            } else {
                errln!("journal append error: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }
    drop(journal);

    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "run_id": run_id.get(),
                "digest": digest_hex,
                "status": "submitted",
                "step_count": step_count
            }),
            output,
        );
    } else {
        outln!("submitted run {}", run_id.get());
        outln!("  digest:     {digest_hex}");
        outln!("  steps:      {step_count}");
        outln!("  durability: {}", durability_as_str(durability));
        outln!("  status:     submitted");
    }

    CliExitCode::Success.into()
}

fn durability_as_str(mode: DurabilityMode) -> &'static str {
    match mode {
        DurabilityMode::Strict => "strict",
        DurabilityMode::Journaled => "journaled",
        DurabilityMode::None => "none",
    }
}

fn generate_submit_run_id() -> u64 {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return u64::MAX;
    };
    match u64::try_from(now.as_nanos()) {
        Ok(value) => value,
        Err(_) => now.as_secs(),
    }
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

fn compile_bytes_json(
    bytes: &[u8],
    output: OutputFormat,
) -> Result<vb_core::CompiledWorkflow, ExitCode> {
    match vb_compile::compile_workflow(bytes) {
        Ok(c) => Ok(c),
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
        vb_storage::JournalEvent::RunAdmission { seq, policy, .. } => {
            outln!("  seq={}: RunAdmission policy={policy:?}", seq.get());
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
        vb_storage::JournalEvent::RunResumed { run, .. } => {
            outln!("  RunResumed run={}", run.get());
        }
        vb_storage::JournalEvent::RunRetried { run, .. } => {
            outln!("  RunRetried run={}", run.get());
        }
        vb_storage::JournalEvent::RunAnswered { run, slot_idx, .. } => {
            outln!("  RunAnswered run={} slot={}", run.get(), slot_idx.get());
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
        vb_storage::JournalEvent::RunAdmission {
            seq,
            run,
            artifact_digest,
            granted_capabilities,
            policy,
        } => {
            serde_json::json!({
                "seq": seq.get(),
                "type": "RunAdmission",
                "run": run.get(),
                "artifact_digest": format!("{artifact_digest:?}"),
                "granted_capabilities": format!("{granted_capabilities:?}"),
                "policy": format!("{policy:?}")
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
        vb_storage::JournalEvent::RunResumed { run, timestamp } => {
            serde_json::json!({
                "type": "RunResumed",
                "run": run.get(),
                "timestamp": timestamp.to_rfc3339()
            })
        }
        vb_storage::JournalEvent::RunRetried { run, timestamp } => {
            serde_json::json!({
                "type": "RunRetried",
                "run": run.get(),
                "timestamp": timestamp.to_rfc3339()
            })
        }
        vb_storage::JournalEvent::RunAnswered {
            run,
            slot_idx,
            answer,
            timestamp,
        } => {
            serde_json::json!({
                "type": "RunAnswered",
                "run": run.get(),
                "slot_idx": slot_idx.get(),
                "answer": format!("{:?}", answer),
                "timestamp": timestamp.to_rfc3339()
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
            let terminal_name = vb_storage::recovery::extract_terminal(&events)
                .map(|e| commands_diff::event_name(e).to_string());

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
                            outln!("terminal: {}", commands_diff::event_name(terminal));
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
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    let trace = commands_journal::build_trace(&events);
    if trace.is_empty() {
        if output != OutputFormat::Text {
            json_out(
                &serde_json::json!({ "run_id": run_id, "trace": [], "total": 0 }),
                output,
            );
        } else {
            outln!("no events found for run {run_id}");
        }
        return CliExitCode::Success.into();
    }
    match output {
        OutputFormat::Json => {
            let entries: Vec<serde_json::Value> = trace.iter().map(trace_entry_to_json).collect();
            json_out(
                &serde_json::json!({ "run_id": run_id, "trace": entries, "total": trace.len() }),
                output,
            );
        }
        OutputFormat::Jsonl => {
            for entry in &trace {
                outln!(
                    "{}",
                    serde_json::to_string(&trace_entry_to_json(entry)).unwrap_or_default()
                );
            }
            outln!("{{\"total\": {}}}", trace.len());
        }
        OutputFormat::Text => {
            outln!("execution trace for run {run_id}");
            for e in &trace {
                let step_str = e.step.map(|s| format!(" step {s}")).unwrap_or_default();
                outln!("  [{}] {}{step_str} (seq {})", e.index, e.event_type, e.seq);
            }
            outln!("{} event(s) total", trace.len());
        }
    }
    CliExitCode::Success.into()
}

/// Convert a structured trace entry to its JSON representation.
fn trace_entry_to_json(entry: &commands_journal::TraceEntry) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("seq".into(), serde_json::Value::from(entry.seq));
    map.insert("type".into(), serde_json::Value::from(entry.event_type));
    if let Some(step) = entry.step {
        map.insert("step".into(), serde_json::Value::from(step));
    }
    for (k, v) in &entry.extra_json {
        map.insert((*k).into(), v.clone());
    }
    serde_json::Value::Object(map)
}

fn cmd_retry(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} not found") }),
                output,
            );
        } else {
            errln!("run {run_id}: no events found");
        }
        return CliExitCode::StorageError.into();
    }
    let analysis = commands_journal::analyze_retry(&events);
    if !analysis.can_retry {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                output,
            );
        } else {
            errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::ValidationFailed.into();
    }
    let resume_step = analysis.last_successful_step.map(|s| s.saturating_add(1));
    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "run_id": run_id, "failed_at_step": analysis.failed_at_step,
                "last_successful_step": analysis.last_successful_step,
                "resume_from_step": resume_step, "events": events.len()
            }),
            output,
        );
    } else {
        match (analysis.failed_at_step, analysis.last_successful_step) {
            (Some(fail), Some(last)) => {
                outln!("Run {run_id} failed at step {fail}. Last successful: step {last}.")
            }
            (Some(fail), None) => {
                outln!("Run {run_id} failed at step {fail}. No successful steps recorded.")
            }
            (None, Some(last)) => outln!("Run {run_id} failed. Last successful: step {last}."),
            (None, None) => outln!("Run {run_id} failed. No step progress recorded."),
        }
        match resume_step {
            Some(step) => outln!("Retry would resume from step {step} with recovered state."),
            None => outln!("Retry would resume from the beginning."),
        }
    }
    ExitCode::SUCCESS
}

fn cmd_resume(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} not found") }),
                output,
            );
        } else {
            errln!("run {run_id}: no events found");
        }
        return CliExitCode::StorageError.into();
    }
    let analysis = commands_journal::analyze_resume(&events);
    if !analysis.can_resume {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                output,
            );
        } else {
            errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::ValidationFailed.into();
    }
    let resume_step = analysis.suspended_at_step;
    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "run_id": run_id, "suspended_at_step": analysis.suspended_at_step,
                "status": "suspended", "resume_from_step": resume_step, "events": events.len()
            }),
            output,
        );
    } else {
        match resume_step {
            Some(step) => outln!(
                "Run {run_id} suspended at step {step}. Resume would continue from step {step} with recovered state."
            ),
            None => outln!(
                "Run {run_id} is active but no explicit suspension point found. Resume would continue from current state."
            ),
        }
    }
    ExitCode::SUCCESS
}

fn cmd_answer(
    run_id: &str,
    step: u16,
    value_file: &std::path::Path,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    // Parse run_id
    let rid = match run_id.parse::<u64>() {
        Ok(id) => vb_core::RunId::new(id),
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("invalid run_id '{run_id}': {e}")
                    }),
                    output,
                );
            } else {
                errln!("invalid run_id '{run_id}': {e}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Read value_file as bytes (expected to be postcard-encoded SlotValue)
    let answer_bytes = match std::fs::read(value_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading value file {}: {e}", value_file.display())
                    }),
                    output,
                );
            } else {
                errln!("error reading value file {}: {e}", value_file.display());
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Derive IPC socket path from db path: <db_parent>/<db_stem>.sock
    // e.g., /var/lib/vb/run.db -> /var/lib/vb/run.sock
    let socket_path = db.with_extension("sock");

    // Connect to the IPC server
    let mut client = match IpcClient::connect(&socket_path) {
        Ok(c) => c,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error connecting to IPC server at {}: {e}", socket_path.display())
                    }),
                    output,
                );
            } else {
                errln!(
                    "error connecting to IPC server at {}: {e}",
                    socket_path.display()
                );
            }
            return CliExitCode::IpcError.into();
        }
    };

    // Construct the IPC payload
    let payload = IpcPayload::AnswerAsk {
        run_id: rid,
        ticket: step.into(),
        answer: answer_bytes,
        taint: None,
    };

    // Send the command
    if let Err(e) = client.send_command(IpcCommand::AnswerAsk, 0, &payload) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("error sending answer: {e}")
                }),
                output,
            );
        } else {
            errln!("error sending answer: {e}");
        }
        return CliExitCode::IpcError.into();
    }

    // Receive and process the response
    match client.recv_response(vb_ipc::MaxPayloadBytes::DEFAULT) {
        Ok((_header, response)) => match response {
            vb_ipc::server::IpcResponse::AcceptedRun { run_id: _ } => {
                if output != OutputFormat::Text {
                    json_out(
                        &serde_json::json!({
                            "success": true,
                            "run_id": rid.get()
                        }),
                        output,
                    );
                } else {
                    outln!("answer accepted for run {}", rid.get());
                }
                ExitCode::SUCCESS
            }
            vb_ipc::server::IpcResponse::RuntimeError { message } => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": message
                        }),
                        output,
                    );
                } else {
                    errln!("runtime error: {message}");
                }
                CliExitCode::RuntimeFailed.into()
            }
            vb_ipc::server::IpcResponse::PayloadError {
                diagnostic: _,
                message,
            } => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": message
                        }),
                        output,
                    );
                } else {
                    errln!("payload error: {message}");
                }
                CliExitCode::ValidationFailed.into()
            }
            other => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("unexpected response: {other:?}")
                        }),
                        output,
                    );
                } else {
                    errln!("unexpected response: {other:?}");
                }
                CliExitCode::RuntimeFailed.into()
            }
        },
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error receiving response: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error receiving response: {e}");
            }
            CliExitCode::IpcError.into()
        }
    }
}

fn run_is_terminal(events: &[vb_storage::JournalEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            vb_storage::JournalEvent::RunFinished { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
                | vb_storage::JournalEvent::RunCancelled { .. }
        )
    })
}

fn format_cancel_output(run_id: &str, reason: Option<&str>, note: &str, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "success": true,
                "run_id": run_id,
                "status": "cancelled",
                "reason": reason,
                "note": note,
            }),
            output,
        );
    } else {
        let detail = match reason {
            Some(r) => format!(" (reason: {r})"),
            None => String::new(),
        };
        outln!("Run {run_id} cancelled{detail} ({note})");
    }
}

fn write_cancel_event(
    journal: &vb_storage::FjallJournal,
    rid: vb_core::RunId,
    reason: Option<String>,
    events: &[vb_storage::JournalEvent],
) -> Result<(), vb_storage::JournalError> {
    let next_seq = events
        .last()
        .map(|e| e.seq().get().saturating_add(1))
        .unwrap_or(0);
    let event = vb_storage::JournalEvent::RunCancelled {
        run: rid,
        seq: vb_storage::EventSeq::new(next_seq),
        attempt: 1,
        reason,
    };
    journal.append_journaled(&event)
}

fn cmd_cancel(
    run_id: &str,
    db: &std::path::Path,
    reason: Option<String>,
    output: OutputFormat,
) -> ExitCode {
    let rid = match parse_run_id(run_id) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    let events = match journal.events_for_run(rid) {
        Ok(ev) => ev,
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
    };

    // Idempotent: no events means run never existed.
    if events.is_empty() {
        format_cancel_output(
            run_id,
            reason.as_deref(),
            "run not found, idempotent",
            output,
        );
        return ExitCode::SUCCESS;
    }

    // Idempotent: already terminal.
    if run_is_terminal(&events) {
        format_cancel_output(
            run_id,
            reason.as_deref(),
            "already terminal, idempotent",
            output,
        );
        return ExitCode::SUCCESS;
    }

    if let Err(e) = write_cancel_event(&journal, rid, reason.clone(), &events) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("error writing cancel event: {e}")
                }),
                output,
            );
        } else {
            errln!("error writing cancel event: {e}");
        }
        return CliExitCode::StorageError.into();
    }

    format_cancel_output(run_id, reason.as_deref(), "cancelled", output);
    ExitCode::SUCCESS
}

fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
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

    let events = match journal.events_for_run(rid) {
        Ok(evts) => evts,
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
    };

    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("no events found for run {run_id}")
                }),
                output,
            );
        } else {
            errln!("no events found for run {run_id}");
        }
        return CliExitCode::StorageError.into();
    }

    let report = commands_incident::build_incident_report(run_id, &events);
    let failed_step_val = report
        .failed_at_step
        .map(|s| serde_json::Value::Number(serde_json::Number::from(s)))
        .unwrap_or(serde_json::Value::Null);

    let json_report = serde_json::json!({
        "run_id": report.run_id,
        "failure_code": report.failure_code,
        "failed_at_step": failed_step_val,
        "side_effects": report.side_effects,
        "repair_hints": report.repair_hints,
    });

    match output {
        OutputFormat::Json => {
            let json_str = serde_json::to_string_pretty(&json_report).unwrap_or_default();
            outln!("{json_str}");
        }
        OutputFormat::Jsonl => {
            let json_str = serde_json::to_string(&json_report).unwrap_or_default();
            outln!("{json_str}");
        }
        OutputFormat::Text => {
            outln!("incident report for run {run_id}");
            outln!("  failure_code:  {}", report.failure_code);
            match report.failed_at_step {
                Some(step) => outln!("  failed_at_step: {step}"),
                None => outln!("  failed_at_step: unknown"),
            }
            outln!("  side_effects:");
            if report.side_effects.is_empty() {
                outln!("    (none)");
            } else {
                for se in &report.side_effects {
                    let step = &se["step"];
                    let action = &se["action"];
                    let certainty = se["certainty"].as_str().unwrap_or("unknown");
                    outln!("    step={step} action={action} certainty={certainty}");
                }
            }
            outln!("  repair_hints:");
            for hint in &report.repair_hints {
                let hint_str = hint.as_str().unwrap_or("unknown");
                outln!("    - {hint_str}");
            }
        }
    }

    if report.failure_found {
        CliExitCode::Success.into()
    } else {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("run {run_id} has no failure event; not an incident")
                }),
                output,
            );
        } else {
            errln!("run {run_id} has no failure event; not an incident");
        }
        CliExitCode::StorageError.into()
    }
}

fn cmd_diff(run_a: &str, run_b: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid_a = match parse_run_id(run_a) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let rid_b = match parse_run_id(run_b) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error opening journal at {}: {e}", db.display())}),
                    output,
                );
            } else {
                errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events_a = match journal.events_for_run(rid_a) {
        Ok(events) => events,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error reading run {run_a}: {e}")}),
                    output,
                );
            } else {
                errln!("error reading run {run_a}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events_b = match journal.events_for_run(rid_b) {
        Ok(events) => events,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error reading run {run_b}: {e}")}),
                    output,
                );
            } else {
                errln!("error reading run {run_b}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    let result = commands_diff::compute_diff(&events_a, &events_b);

    match output {
        OutputFormat::Json => {
            json_out(
                &serde_json::json!({
                    "run_a": run_a,
                    "run_b": run_b,
                    "events_a": result.events_a,
                    "events_b": result.events_b,
                    "diffs": result.diffs,
                    "total_differences": result.diffs.len()
                }),
                output,
            );
        }
        OutputFormat::Jsonl => {
            for diff in &result.diffs {
                outln!("{}", serde_json::to_string(diff).unwrap_or_default());
            }
            outln!(
                "{}",
                format!("{{\"total_differences\": {}}}", result.diffs.len())
            );
        }
        OutputFormat::Text => {
            outln!("diff: run {run_a} vs run {run_b}");
            outln!("  events: {} vs {}", result.events_a, result.events_b);
            if result.diffs.is_empty() {
                outln!("  no differences found");
            } else {
                for diff in &result.diffs {
                    print_diff_entry(diff);
                }
                outln!("  {} difference(s) total", result.diffs.len());
            }
        }
    }
    CliExitCode::Success.into()
}

fn print_diff_entry(diff: &serde_json::Value) {
    let kind = diff
        .get("kind")
        .and_then(|k| k.as_str())
        .unwrap_or("unknown");
    match kind {
        "only_in_a" => {
            let idx = diff.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  [{idx}] - only in run A");
        }
        "only_in_b" => {
            let idx = diff.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  [{idx}] + only in run B");
        }
        "changed" => {
            let idx = diff.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  [{idx}] ~ changed");
        }
        "step_missing_in_b" => {
            let s = diff.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  step {s}: - present in run A only");
        }
        "step_missing_in_a" => {
            let s = diff.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  step {s}: + present in run B only");
        }
        "step_outcome_differs" => {
            let s = diff.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
            let oa = diff
                .get("outcome_a")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let ob = diff
                .get("outcome_b")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            outln!("  step {s}: ~ {oa} vs {ob}");
        }
        "slot_missing_in_b" => {
            let s = diff.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  slot {s}: - present in run A only");
        }
        "slot_missing_in_a" => {
            let s = diff.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            outln!("  slot {s}: + present in run B only");
        }
        "slot_value_differs" => {
            let s = diff.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            let va = diff.get("value_a").and_then(|v| v.as_str()).unwrap_or("?");
            let vb = diff.get("value_b").and_then(|v| v.as_str()).unwrap_or("?");
            outln!("  slot {s}: ~ {va} vs {vb}");
        }
        _ => {
            outln!("  unknown diff kind: {kind}");
        }
    }
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
        CompileError::TriggerShape {
            trigger,
            expected: _,
        } => {
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
        CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: _,
        } => {
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
        CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field,
        } => {
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
        CompileError::StepFieldShape {
            step,
            field,
            expected: _,
        } => {
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
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        } => {
            outln!("Primitive Limit Exceeded");
            outln!(
                "  Primitive '{primitive}' field '{field}' value {value} exceeds limit {limit}."
            );
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
        CompileError::UnknownReferenceName {
            kind,
            reference,
            name,
        } => {
            outln!("Unknown Reference");
            outln!("  Reference '{reference}' refers to unknown {kind} '{name}'.");
        }
        CompileError::UnsupportedAccessorReference {
            reference,
            root,
            path,
        } => {
            outln!("Unsupported Accessor Reference");
            outln!(
                "  Accessor reference '{reference}' (root: {root}, path: {path}) is not supported."
            );
        }
        CompileError::UnknownStepTarget { step, target } => {
            outln!("Unknown Step Target");
            outln!("  Step {step} branches to undeclared step index {target}.");
        }
        CompileError::UnreachableStep { step } => {
            outln!("Unreachable Step");
            outln!("  Step {step} cannot be reached from the workflow entry point.");
        }
        CompileError::TypeMismatch {
            field,
            expected,
            found,
        } => {
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
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        } => {
            outln!("Expression Stack Mismatch");
            outln!(
                "  Expression {expr_index}: declared {declared} stack slots, computed {computed}."
            );
        }
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
        } => {
            outln!("Accessor Slot Out of Range");
            outln!(
                "  Accessor {accessor_index} references slot {slot}, but slot_count is {slot_count}."
            );
        }
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
        } => {
            outln!("Accessor Path Invalid");
            outln!("  Accessor {accessor_index} has invalid segment at index {segment_index}.");
        }
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        } => {
            outln!("Slot Reference Out of Range");
            outln!(
                "  Slot {slot} is out of range (slot_count={slot_count}) in context: {context}."
            );
        }
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label: _,
        } => {
            outln!("Loop Body Step Out of Range");
            outln!(
                "  Step {step}: loop body step out of range (node_count={node_count}, source_node={source_node})."
            );
        }
        ValidationError::SlotDependencyCycle { slot, chain } => {
            outln!("Slot Dependency Cycle");
            outln!("  Slot {slot} has a dependency cycle: {chain}.");
        }
        ValidationError::NodeKindConstraintViolation { node_index, detail } => {
            outln!("Node Kind Constraint Violation");
            outln!("  Node {node_index}: {detail}.");
        }
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
        } => {
            outln!("Action Contract Missing");
            outln!(
                "  Do node {node_index} references action_id {action_id}, which has no contract."
            );
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
        ValidationError::AccessorPathTooDeep {
            accessor_index,
            depth,
            max,
        } => {
            outln!("Accessor Path Too Deep");
            outln!(
                "  Accessor {accessor_index} has depth {depth}, which exceeds the maximum {max}."
            );
        }
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index,
            segment_index,
            symbol,
            symbols_count,
        } => {
            outln!("Accessor Symbol Out of Bounds");
            outln!(
                "  Accessor {accessor_index} segment {segment_index}: symbol {symbol} is out of bounds (symbols_count={symbols_count})."
            );
        }
        ValidationError::CapabilityNameEmpty {
            action_id,
            capability_index,
        } => {
            outln!("Capability Name Empty");
            outln!("  Action {action_id}: capability {capability_index} has an empty name.");
        }
        ValidationError::CapabilityNameTooLong {
            action_id,
            capability_index,
            len,
            max,
        } => {
            outln!("Capability Name Too Long");
            outln!(
                "  Action {action_id}: capability {capability_index} name length {len} exceeds max {max}."
            );
        }
        ValidationError::CapabilityNameInvalid {
            action_id,
            capability_index,
            name,
        } => {
            outln!("Capability Name Invalid");
            outln!("  Action {action_id}: capability {capability_index} name '{name}' is invalid.");
        }
        ValidationError::CapabilityActionMismatch {
            contract_action_id,
            capability_action_id,
            capability_index,
        } => {
            outln!("Capability Action Mismatch");
            outln!(
                "  Contract action {contract_action_id} != capability action {capability_action_id} at index {capability_index}."
            );
        }
        ValidationError::CapabilityDuplicate {
            action_id,
            first_index,
            duplicate_index,
            name,
        } => {
            outln!("Capability Duplicate");
            outln!(
                "  Action {action_id}: capability '{name}' first at {first_index}, duplicate at {duplicate_index}."
            );
        }
    }
}

fn cmd_graph(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let graph = commands_workflow::generate_dot(&compiled);

    if output != OutputFormat::Text {
        json_out(
            &serde_json::json!({
                "format": "dot",
                "nodes": graph.node_count,
                "edges": graph.edge_count,
                "dot": graph.dot
            }),
            output,
        );
    } else {
        outln!("{}", graph.dot);
    }

    CliExitCode::Success.into()
}

fn cmd_simulate(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let result = commands_workflow::simulate_workflow(&compiled);

    if output != OutputFormat::Text {
        let trace: Vec<serde_json::Value> = result
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "step": s.index,
                    "kind": s.kind_label,
                    "description": s.description,
                })
            })
            .collect();
        json_out(
            &serde_json::json!({
                "schema_version": "velvet-ballastics/v1",
                "kind": "simulate",
                "success": true,
                "total_steps": result.total_steps,
                "total_actions": result.action_count,
                "total_branches": result.branch_count,
                "trace": trace
            }),
            output,
        );
    } else {
        for step in &result.steps {
            outln!("Step {}: {}", step.index, step.description);
        }
        outln!("");
        outln!("simulation summary");
        outln!("  steps:    {}", result.total_steps);
        outln!("  actions:  {}", result.action_count);
        outln!("  branches: {}", result.branch_count);
        outln!("dry-run complete");
    }

    CliExitCode::Success.into()
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

    // Check 4: trim eligibility diagnostic (non-destructive)
    match journal.trim_eligibility_diagnostic(vb_storage::TrimPolicy::default()) {
        Ok(diag) => {
            let mut runs = Vec::new();
            for run in &diag.runs {
                match run {
                    vb_storage::TrimEligibility::Eligible {
                        run: r,
                        safe_point,
                        events_trimmable,
                    } => {
                        runs.push(serde_json::json!({
                            "run": r.get(),
                            "status": "eligible",
                            "safe_point": safe_point.get(),
                            "events_trimmable": events_trimmable
                        }));
                    }
                    vb_storage::TrimEligibility::Blocked { run: r, blocker } => {
                        let blocker_name = match blocker {
                            vb_storage::TrimBlocker::NoDurableSnapshot => "no_durable_snapshot",
                            vb_storage::TrimBlocker::RetentionPolicy { .. } => "retention_policy",
                        };
                        runs.push(serde_json::json!({
                            "run": r.get(),
                            "status": "blocked",
                            "blocker": blocker_name
                        }));
                    }
                }
            }
            checks.push(serde_json::json!({
                "check": "trim_eligibility",
                "status": "pass",
                "message": format!(
                    "trim eligibility: {} total, {} eligible, {} blocked, {} events trimmable",
                    diag.total_runs, diag.eligible_runs, diag.blocked_runs, diag.total_events_trimmable
                ),
                "total_runs": diag.total_runs,
                "eligible_runs": diag.eligible_runs,
                "blocked_runs": diag.blocked_runs,
                "total_events_trimmable": diag.total_events_trimmable,
                "runs": runs
            }));
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "check": "trim_eligibility",
                "status": "fail",
                "message": format!("trim eligibility diagnostic failed: {e}")
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
                errln!("FAIL: trim eligibility diagnostic failed: {e}");
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
        // Print trim eligibility summary in text mode
        if let Ok(diag) = journal.trim_eligibility_diagnostic(vb_storage::TrimPolicy::default()) {
            outln!(
                "doctor: trim eligibility — {} total, {} eligible, {} blocked, {} events trimmable",
                diag.total_runs,
                diag.eligible_runs,
                diag.blocked_runs,
                diag.total_events_trimmable
            );
            for run in &diag.runs {
                match run {
                    vb_storage::TrimEligibility::Eligible {
                        run: r,
                        safe_point,
                        events_trimmable,
                    } => {
                        outln!(
                            "doctor:   run {} eligible — safe_point={} events_trimmable={}",
                            r.get(),
                            safe_point.get(),
                            events_trimmable
                        );
                    }
                    vb_storage::TrimEligibility::Blocked { run: r, blocker } => {
                        let blocker_name = match blocker {
                            vb_storage::TrimBlocker::NoDurableSnapshot => "no_durable_snapshot",
                            vb_storage::TrimBlocker::RetentionPolicy { .. } => "retention_policy",
                        };
                        outln!(
                            "doctor:   run {} blocked — blocker={}",
                            r.get(),
                            blocker_name
                        );
                    }
                }
            }
        }
        outln!("doctor: all checks passed");
    }
    ExitCode::SUCCESS
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
        ParseError::InvalidStep(step) => {
            writeln!(handle, "invalid step: {step}\n\n{HELP}")
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
            writeln!(
                handle,
                "unknown command: {cmd} (expected one of: {VALID_COMMANDS})\n\n{HELP}"
            )
        }
        ParseError::InvalidStatusArgument(reason) => {
            writeln!(handle, "invalid status argument: {reason}\n\n{HELP}")
        }
        ParseError::UnknownActionCommand(cmd) => {
            writeln!(
                handle,
                "unknown action command: {cmd} (expected: list, inspect)\n\n{HELP}"
            )
        }
        ParseError::UnknownActionRegistry(registry) => {
            writeln!(
                handle,
                "unknown action registry: {registry} (expected: registered, empty, uninitialized)\n\n{HELP}"
            )
        }
        ParseError::MissingActionRegistryValue => writeln!(
            handle,
            "missing action-args value for --registry (expected: registered, empty, uninitialized)\n\n{HELP}"
        ),
        ParseError::UnknownActionListFlag(flag) => {
            writeln!(handle, "unknown action list flag: {flag}\n\n{HELP}")
        }
        ParseError::UnexpectedActionListArgument(argument) => writeln!(
            handle,
            "unexpected action list argument: {argument}\n\n{HELP}"
        ),
        ParseError::UnknownActionInspectFlag(flag) => {
            writeln!(handle, "unknown action inspect flag: {flag}\n\n{HELP}")
        }
        ParseError::UnexpectedActionInspectArgument(argument) => writeln!(
            handle,
            "unexpected action inspect argument: {argument}\n\n{HELP}"
        ),
        ParseError::InvalidActionId(action_id) => {
            writeln!(handle, "invalid action id: {action_id}\n\n{HELP}")
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
        ParseError::ReasonTooLong => {
            writeln!(
                handle,
                "reason exceeds maximum length of 256 characters\n\n{HELP}"
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
mod mode_error;

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mode_activation_tests.rs"]
mod mode_activation_tests;

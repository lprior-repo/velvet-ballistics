#![forbid(unsafe_code)]
//! Runtime execution helpers for compiled workflows.

use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget,
};
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_contract_error_json, write_failure_message,
    write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use crate::run_compiled::InputMappingError;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub(crate) fn map_runtime_inputs(
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

pub(crate) fn runtime_journal_for_mode(
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, ExitCode> {
    match durability {
        DurabilityMode::None => Ok(vb_runtime::journal::NoopRuntimeJournal::shared()),
        DurabilityMode::Journaled => open_storage_runtime_journal(db, false, output),
        DurabilityMode::Strict => open_storage_runtime_journal(db, true, output),
    }
}

pub(crate) fn runtime_config_for_durability(
    durability: DurabilityMode,
) -> vb_runtime::shard::ShardConfig {
    let mut config = vb_runtime::shard::ShardConfig::default();
    if durability == DurabilityMode::None {
        config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    }
    config
}

pub(crate) fn open_storage_runtime_journal(
    db: Option<&std::path::Path>,
    strict: bool,
    output: OutputFormat,
) -> Result<vb_runtime::journal::SharedRuntimeJournal, ExitCode> {
    let Some(path) = db else {
        report_runtime_error(
            format_args!("--db is required when --durability is strict or journaled"),
            output,
        );
        return Err(CliExitCode::StorageError.into());
    };
    let journal = match vb_storage::FjallJournal::open(path, None) {
        Ok(journal) => Arc::new(journal),
        Err(e) => {
            report_runtime_error(
                format_args!("error opening journal at {}: {e}", path.display()),
                output,
            );
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

pub(crate) fn run_compiled_workflow(
    compiled: &vb_core::CompiledWorkflow,
    inputs: Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> ExitCode {
    let admitted_workflow = match workflow_for_durable_admission(compiled, durability, output) {
        Ok(workflow) => workflow,
        Err(code) => return code,
    };
    let run_id = vb_core::RunId::new(1);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        report_runtime_error(
            format_args!("runtime configuration error: shard count must be non-zero"),
            output,
        );
        return CliExitCode::RuntimeFailed.into();
    };
    let config = runtime_config_for_durability(durability);
    if durability != DurabilityMode::None
        && let Some(db_path) = db
        && let Err(code) = store_compiled_artifact(&admitted_workflow, db_path, durability, output)
    {
        return code;
    }
    let journal = match runtime_journal_for_mode(durability, db, output) {
        Ok(journal) => journal,
        Err(code) => return code,
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(shard_count, config, journal);

    if let Err(e) = runtime.submit_compiled_with_inputs(run_id, admitted_workflow, inputs) {
        report_runtime_error(format_args!("runtime submit error: {e}"), output);
        return CliExitCode::RuntimeFailed.into();
    }
    if let Err(e) = runtime.tick_all() {
        report_runtime_error(format_args!("runtime tick error: {e}"), output);
        return CliExitCode::RuntimeFailed.into();
    }

    let counters = runtime.counters_snapshot();
    let traces = runtime.drain_trace();
    let status = if counters.runs_failed != 0 {
        "failed"
    } else if counters.runs_completed != 0 {
        "completed"
    } else {
        "accepted"
    };
    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
            &serde_json::json!({
                "success": counters.runs_failed == 0,
                "run_id": run_id.get(),
                "status": status,
                "runtime": {
                    "submitted": counters.runs_submitted,
                    "completed": counters.runs_completed,
                    "failed": counters.runs_failed,
                    "steps": counters.steps_executed
                },
                "trace_count": traces.len()
            }),
            output,
        );
    } else {
        crate::outln!(
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
            crate::errln!("run failed");
        } else if counters.runs_completed != 0 {
            crate::outln!("run completed");
        } else {
            crate::outln!("run accepted but not terminal after one runtime tick");
        }
    }

    if counters.runs_failed != 0 {
        return CliExitCode::RuntimeFailed.into();
    }

    ExitCode::SUCCESS
}

fn workflow_for_durable_admission(
    compiled: &vb_core::CompiledWorkflow,
    durability: DurabilityMode,
    output: OutputFormat,
) -> Result<vb_core::CompiledWorkflow, ExitCode> {
    if durability == DurabilityMode::None {
        return Ok(compiled.clone());
    }
    let mut parts = compiled.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let ir_bytes = postcard::to_allocvec(&parts).map_err(|error| {
        report_runtime_error(format_args!("compiled IR encode error: {error}"), output);
        ExitCode::from(CliExitCode::CompileFailed)
    })?;
    parts.digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&ir_bytes).into());
    vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|error| {
        report_runtime_error(
            format_args!("compiled IR validation error after digest normalization: {error}"),
            output,
        );
        CliExitCode::CompileFailed.into()
    })
}

pub(crate) fn store_compiled_artifact(
    compiled: &vb_core::CompiledWorkflow,
    db: &std::path::Path,
    durability: DurabilityMode,
    output: OutputFormat,
) -> Result<(), ExitCode> {
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
    let policy = match durability {
        DurabilityMode::Strict | DurabilityMode::Journaled => {
            vb_core::policy::RuntimePolicy::Strict
        }
        DurabilityMode::None => vb_core::policy::RuntimePolicy::Relaxed,
    };
    vb_storage::admission::submit_artifact(&journal, compiled, policy).map_err(|e| {
        report_compiled_ir_store_error(format_args!("compiled IR write error: {e}"), output);
        ExitCode::from(CliExitCode::StorageError)
    })?;
    Ok(())
}

pub(crate) fn report_runtime_error(args: std::fmt::Arguments<'_>, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({"success": false, "error": args.to_string()}),
            output,
        );
    } else {
        crate::errln!("{args}");
    }
}

pub(crate) fn print_trace_event(event: &vb_runtime::trace::TraceEvent) {
    match event {
        vb_runtime::trace::TraceEvent::StepStarted { step, .. } => {
            crate::outln!("  trace: StepStarted step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::StepEnded { step, .. } => {
            crate::outln!("  trace: StepEnded step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::SlotWritten { slot, .. } => {
            crate::outln!("  trace: SlotWritten slot={}", slot.get());
        }
        vb_runtime::trace::TraceEvent::ActionScheduled { step, .. } => {
            crate::outln!("  trace: ActionScheduled step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::ActionCompleted { step, .. } => {
            crate::outln!("  trace: ActionCompleted step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::ActionFailed { step, .. } => {
            crate::outln!("  trace: ActionFailed step={}", step.get());
        }
        vb_runtime::trace::TraceEvent::AskAnswered { step, slot, .. } => {
            crate::outln!(
                "  trace: AskAnswered step={} slot={}",
                step.get(),
                slot.get()
            );
        }
        vb_runtime::trace::TraceEvent::RunSubmitted { .. } => {
            crate::outln!("  trace: RunSubmitted");
        }
        vb_runtime::trace::TraceEvent::RunFinished { .. } => {
            crate::outln!("  trace: RunFinished");
        }
        vb_runtime::trace::TraceEvent::RunFailed { .. } => {
            crate::outln!("  trace: RunFailed");
        }
        vb_runtime::trace::TraceEvent::RunCancelled { .. } => {
            crate::outln!("  trace: RunCancelled");
        }
        _ => {
            crate::outln!("  trace: Unknown");
        }
    }
}

fn report_compiled_ir_store_error(msg: std::fmt::Arguments<'_>, output: OutputFormat) {
    if output != OutputFormat::Text {
        crate::errln!("{}", msg);
    } else {
        crate::errln!("compiled IR store error: {}", msg);
    }
}

//! Module: run_compiled_runtime

use crate::app_impl::prelude::*;

pub(crate) mod completion_output;
#[cfg(test)]
mod completion_output_tests;
pub(crate) mod secret_presence;
mod workflow;

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
    workflow::run(compiled, inputs, durability, db, output)
}

pub(super) fn last_started_step(
    traces: &[vb_runtime::trace::TraceEvent],
) -> Option<vb_core::StepIdx> {
    traces.iter().rev().find_map(|trace| match trace {
        vb_runtime::trace::TraceEvent::StepStarted { step, .. } => Some(*step),
        _ => None,
    })
}

/// Selects the run id for `run` / `run-compiled`.
///
/// Ephemeral (`none`) executions always use run 1: nothing is persisted, so
/// the id cannot collide. Durable executions also use run 1 on a fresh
/// database, but fall back to a submit-style time-based id when the database
/// already holds records for run 1 — otherwise the second run into the same
/// database fails at admission persistence (the re-emitted run-1 events
/// collide with the stored ones).
fn fresh_durable_run_id(
    db: Option<&std::path::Path>,
    durability: DurabilityMode,
) -> vb_core::RunId {
    let base = vb_core::RunId::new(1);
    if durability == DurabilityMode::None {
        return base;
    }
    let Some(path) = db else {
        return base;
    };
    if run_records_exist(path, base) {
        vb_core::RunId::new(crate::submit::generate_submit_run_id())
    } else {
        base
    }
}

/// Mirrors the `commit_run_submission` occupancy check: the direct `run`
/// execution path persists events but no header record, so events alone must
/// count as "taken". Any probe failure returns false so the later stages
/// report the real error instead of this heuristic masking it.
fn run_records_exist(db: &std::path::Path, run: vb_core::RunId) -> bool {
    let Ok(journal) = vb_storage::FjallJournal::open(db, None) else {
        return false;
    };
    match journal.run_header(run) {
        Ok(header) if header.is_some() => true,
        Ok(_) => match journal.events_for_run(run) {
            Ok(events) => !events.is_empty(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

pub(crate) fn store_compiled_artifact(
    compiled: &vb_core::CompiledWorkflow,
    db: &std::path::Path,
    output: OutputFormat,
    policy: vb_core::RuntimePolicy,
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
    crate::run::submit_cli_compiled_artifact(&journal, compiled, policy)
        .map_err(|e| {
            report_compiled_ir_store_error(
                format_args!("compiled artifact admission error: {e}"),
                output,
            );
            CliExitCode::StorageError.into()
        })
        .map(|_| ())
}

pub(crate) fn report_runtime_error(args: std::fmt::Arguments<'_>, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({"success": false, "error": args.to_string()}),
            output,
        );
    } else {
        errln!("{args}");
    }
}

pub(crate) fn print_trace_event(event: &vb_runtime::trace::TraceEvent) {
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
        _ => {
            outln!("  trace: Unknown");
        }
    }
}

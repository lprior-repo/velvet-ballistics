//! Module: run_compiled_runtime

use crate::app_impl::prelude::*;

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
        && let Err(code) = store_compiled_artifact(compiled, db_path, output)
    {
        return code;
    }
    let journal = match runtime_journal_for_mode(durability, db, output) {
        Ok(journal) => journal,
        Err(code) => return code,
    };
    let mut runtime = vb_runtime::runtime::Runtime::new_with_journal(shard_count, config, journal);

    if let Err(e) = runtime.submit_compiled_with_inputs(run_id, compiled.clone(), inputs) {
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
        emit_json_or_return!(
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
        } else if counters.runs_completed != 0 {
            outln!("run completed");
        } else {
            outln!("run accepted but not terminal after one runtime tick");
        }
    }

    if counters.runs_failed != 0 {
        return CliExitCode::RuntimeFailed.into();
    }

    ExitCode::SUCCESS
}

pub(crate) fn store_compiled_artifact(
    compiled: &vb_core::CompiledWorkflow,
    db: &std::path::Path,
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
    crate::run::submit_cli_compiled_artifact(&journal, compiled)
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

use crate::app_impl::prelude::*;
use crate::output::{json_out, output_error_exit};

pub(super) fn run(
    compiled: &vb_core::CompiledWorkflow,
    inputs: Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> ExitCode {
    let (run_id, mut runtime) = match prepare_run(compiled, durability, db, output) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    if let Err(code) = submit_and_tick(&mut runtime, run_id, compiled, inputs, output) {
        return code;
    }
    let counters = runtime.counters_snapshot();
    let traces = runtime.drain_trace();
    report_run(run_id, compiled, counters, &traces, output)
}

fn prepare_run(
    compiled: &vb_core::CompiledWorkflow,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> Result<(vb_core::RunId, vb_runtime::runtime::Runtime), ExitCode> {
    let run_id = super::fresh_durable_run_id(db, durability);
    let shard_count = required_shard_count(output)?;
    let config = crate::run_compiled::runtime_config_for_durability(durability);
    store_if_durable(compiled, durability, db, &config, output)?;
    let journal = crate::run_compiled_runtime::runtime_journal_for_mode(durability, db, output)?;
    build_runtime(shard_count, config, journal, compiled, output).map(|runtime| (run_id, runtime))
}

fn required_shard_count(output: OutputFormat) -> Result<NonZeroUsize, ExitCode> {
    NonZeroUsize::new(1).ok_or_else(|| {
        super::report_runtime_error(
            format_args!("runtime configuration error: shard count must be non-zero"),
            output,
        );
        CliExitCode::RuntimeFailed.into()
    })
}

fn store_if_durable(
    compiled: &vb_core::CompiledWorkflow,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    config: &vb_runtime::shard::ShardConfig,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    if durability != DurabilityMode::None
        && let Some(path) = db
    {
        super::store_compiled_artifact(compiled, path, output, config.policy)?;
    }
    Ok(())
}

fn build_runtime(
    shard_count: NonZeroUsize,
    config: vb_runtime::shard::ShardConfig,
    journal: vb_runtime::journal::SharedRuntimeJournal,
    compiled: &vb_core::CompiledWorkflow,
    output: OutputFormat,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    let mut runtime =
        vb_runtime::runtime::Runtime::new(shard_count, config, journal).map_err(|error| {
            super::report_runtime_error(
                format_args!("runtime construction failed: {error}"),
                output,
            );
            CliExitCode::RuntimeFailed
        })?;
    super::secret_presence::install(&mut runtime, compiled);
    Ok(runtime)
}

fn submit_and_tick(
    runtime: &mut vb_runtime::runtime::Runtime,
    run_id: vb_core::RunId,
    compiled: &vb_core::CompiledWorkflow,
    inputs: Box<[(vb_core::SlotIdx, vb_core::SlotValue)]>,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    if let Err(error) = runtime.submit_compiled_with_inputs(run_id, compiled.clone(), inputs) {
        super::report_runtime_error(format_args!("runtime submit error: {error}"), output);
        return Err(CliExitCode::RuntimeFailed.into());
    }
    if let Err(error) = runtime.tick_all() {
        super::report_runtime_error(format_args!("runtime tick error: {error}"), output);
        return Err(CliExitCode::RuntimeFailed.into());
    }
    Ok(())
}

fn report_run(
    run_id: vb_core::RunId,
    compiled: &vb_core::CompiledWorkflow,
    counters: vb_runtime::counters::CounterSnapshot,
    traces: &[vb_runtime::trace::TraceEvent],
    output: OutputFormat,
) -> ExitCode {
    if output == OutputFormat::Text {
        report_text(run_id, compiled, counters, traces);
    } else if let Err(code) = report_structured(run_id, counters, traces, output) {
        return code;
    }
    run_exit_code(counters)
}

fn report_structured(
    run_id: vb_core::RunId,
    counters: vb_runtime::counters::CounterSnapshot,
    traces: &[vb_runtime::trace::TraceEvent],
    output: OutputFormat,
) -> Result<(), ExitCode> {
    let value = serde_json::json!({
        "success": counters.runs_failed == 0 && counters.runs_completed != 0,
        "run_id": run_id.get(),
        "status": runtime_status(counters),
        "runtime": {
            "submitted": counters.runs_submitted,
            "completed": counters.runs_completed,
            "failed": counters.runs_failed,
            "steps": counters.steps_executed
        },
        "trace_count": traces.len()
    });
    json_out(&value, output).map_err(|error| output_error_exit(&error))
}

fn report_text(
    run_id: vb_core::RunId,
    compiled: &vb_core::CompiledWorkflow,
    counters: vb_runtime::counters::CounterSnapshot,
    traces: &[vb_runtime::trace::TraceEvent],
) {
    outln!(
        "run {}: submitted={} completed={} failed={} steps={}",
        run_id.get(),
        counters.runs_submitted,
        counters.runs_completed,
        counters.runs_failed,
        counters.steps_executed
    );
    traces.iter().for_each(super::print_trace_event);
    report_terminal_result(compiled, counters, traces);
}

fn report_terminal_result(
    compiled: &vb_core::CompiledWorkflow,
    counters: vb_runtime::counters::CounterSnapshot,
    traces: &[vb_runtime::trace::TraceEvent],
) {
    if counters.runs_failed != 0 {
        report_failed_step(compiled, traces);
    } else if counters.runs_completed != 0 {
        outln!("run completed");
    } else {
        outln!("run accepted but not terminal after one runtime tick");
    }
}

fn report_failed_step(
    compiled: &vb_core::CompiledWorkflow,
    traces: &[vb_runtime::trace::TraceEvent],
) {
    match super::last_started_step(traces) {
        Some(step) => {
            let name = compiled
                .step_name(step)
                .map_or_else(|| "?".to_string(), String::from);
            errln!("run failed at step {} ({name})", step.get());
        }
        None => errln!("run failed"),
    }
}

fn runtime_status(counters: vb_runtime::counters::CounterSnapshot) -> &'static str {
    if counters.runs_failed != 0 {
        "failed"
    } else if counters.runs_completed != 0 {
        "completed"
    } else {
        "accepted"
    }
}

fn run_exit_code(counters: vb_runtime::counters::CounterSnapshot) -> ExitCode {
    if counters.runs_failed != 0 || counters.runs_completed == 0 {
        CliExitCode::RuntimeFailed.into()
    } else {
        ExitCode::SUCCESS
    }
}

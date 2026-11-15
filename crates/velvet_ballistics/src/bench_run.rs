//! Module: bench_run

use crate::app_impl::prelude::*;

pub(crate) fn cmd_bench_run(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let (compiled, compile_elapsed) = match compile_benchmark_workflow(workflow, output) {
        Ok(result) => result,
        Err(code) => return code,
    };
    let (counters, run_elapsed) = match run_benchmark_workflow(compiled, output) {
        Ok(result) => result,
        Err(code) => return code,
    };
    let total_us = compile_elapsed
        .as_micros()
        .saturating_add(run_elapsed.as_micros());
    let report_code =
        report_benchmark_result(counters, compile_elapsed, run_elapsed, total_us, output);
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }
    benchmark_exit_code(counters)
}

fn compile_benchmark_workflow(
    workflow: &std::path::Path,
    output: OutputFormat,
) -> Result<(vb_core::CompiledWorkflow, std::time::Duration), ExitCode> {
    let bytes = read_file(workflow, output, CliExitCode::ValidationFailed)?;
    let compile_start = Instant::now();
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(compiled) => compiled,
        Err(errors) => {
            report_compile_errors(&errors, output);
            return Err(CliExitCode::CompileFailed.into());
        }
    };
    reject_unknown_actions(&compiled, output, CliExitCode::CompileFailed)?;
    Ok((compiled, compile_start.elapsed()))
}

fn report_compile_errors(errors: &vb_compile::CompileErrors, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": "compilation failed",
                "errors": errors.0.iter().map(ToString::to_string).collect::<Vec<_>>()
            }),
            output,
        );
    } else {
        errors
            .0
            .iter()
            .for_each(|error| errln!("compile error: {error}"));
    }
}

fn run_benchmark_workflow(
    compiled: vb_core::CompiledWorkflow,
    output: OutputFormat,
) -> Result<(vb_runtime::counters::CounterSnapshot, std::time::Duration), ExitCode> {
    let run_start = Instant::now();
    let mut runtime = create_benchmark_runtime(output)?;
    submit_benchmark_workflow(&mut runtime, compiled, output)?;
    runtime
        .tick_all()
        .map_err(|error| report_benchmark_tick_error(error, output))?;
    let run_elapsed = run_start.elapsed();
    Ok((runtime.counters_snapshot(), run_elapsed))
}

fn create_benchmark_runtime(
    output: OutputFormat,
) -> Result<vb_runtime::runtime::Runtime, ExitCode> {
    let Some(shard_count) = NonZeroUsize::new(1) else {
        report_benchmark_configuration_error(output);
        return Err(CliExitCode::RuntimeFailed.into());
    };
    let config = runtime_config_for_durability(DurabilityMode::None);
    match vb_runtime::runtime::Runtime::new_for_tests_and_benchmarks_only(shard_count, config) {
        Ok(runtime) => Ok(runtime),
        Err(error) => {
            crate::run_compiled_runtime::report_runtime_error(
                format_args!("runtime construction failed: {error}"),
                output,
            );
            Err(CliExitCode::RuntimeFailed.into())
        }
    }
}

fn report_benchmark_configuration_error(output: OutputFormat) {
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
}

fn submit_benchmark_workflow(
    runtime: &mut vb_runtime::runtime::Runtime,
    compiled: vb_core::CompiledWorkflow,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    crate::run_compiled_runtime::secret_presence::AvailableNames::default()
        .install(runtime, &compiled);
    runtime
        .submit_compiled(vb_core::RunId::new(1), compiled)
        .map_err(|error| report_benchmark_submit_error(error, output))
}

fn report_benchmark_submit_error(error: impl std::fmt::Display, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("runtime submit error: {error}")
            }),
            output,
        );
    } else {
        errln!("runtime submit error: {error}");
    }
    CliExitCode::RuntimeFailed.into()
}

fn report_benchmark_tick_error(error: impl std::fmt::Display, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("runtime tick error: {error}")
            }),
            output,
        );
    } else {
        errln!("runtime tick error: {error}");
    }
    CliExitCode::RuntimeFailed.into()
}

fn report_benchmark_result(
    counters: vb_runtime::counters::CounterSnapshot,
    compile_elapsed: std::time::Duration,
    run_elapsed: std::time::Duration,
    total_us: u128,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        report_benchmark_json(counters, compile_elapsed, run_elapsed, total_us, output)
    } else {
        report_benchmark_text(counters, compile_elapsed, run_elapsed, total_us);
        ExitCode::SUCCESS
    }
}

fn report_benchmark_json(
    counters: vb_runtime::counters::CounterSnapshot,
    compile_elapsed: std::time::Duration,
    run_elapsed: std::time::Duration,
    total_us: u128,
    output: OutputFormat,
) -> ExitCode {
    emit_json_or_return!(
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
    ExitCode::SUCCESS
}

fn report_benchmark_text(
    counters: vb_runtime::counters::CounterSnapshot,
    compile_elapsed: std::time::Duration,
    run_elapsed: std::time::Duration,
    total_us: u128,
) {
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

fn benchmark_exit_code(counters: vb_runtime::counters::CounterSnapshot) -> ExitCode {
    if counters.runs_failed != 0 {
        CliExitCode::RuntimeFailed.into()
    } else {
        ExitCode::SUCCESS
    }
}

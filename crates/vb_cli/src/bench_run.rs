#![forbid(unsafe_code)]
//! Benchmark workflow execution command.

fn cmd_bench_run(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
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
    let config = runtime_config_for_durability(DurabilityMode::None);
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


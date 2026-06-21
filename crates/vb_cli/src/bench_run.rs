#![forbid(unsafe_code)]
//! Benchmark workflow execution command.

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
use crate::run_compiled_runtime::runtime_config_for_durability;
use crate::run_id::generate_run_id_from_clock;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

pub(crate) fn cmd_bench_run(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
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
                    CliExitCode::CompileFailed,
                    output,
                );
            } else {
                for err in &errors.0 {
                    crate::errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
        }
    };
    let compile_elapsed = compile_start.elapsed();

    let run_start = Instant::now();
    let run_id = generate_run_id_from_clock();
    let Some(shard_count) = NonZeroUsize::new(1) else {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": "runtime configuration error: shard count must be non-zero"
                }),
                CliExitCode::RuntimeFailed,
                output,
            );
        } else {
            crate::errln!("runtime configuration error: shard count must be non-zero");
        }
        return CliExitCode::RuntimeFailed.into();
    };
    let config = runtime_config_for_durability(DurabilityMode::None);
    let mut runtime = match vb_runtime::runtime::Runtime::new(shard_count, config) {
        Ok(runtime) => runtime,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("runtime construction error: {e}")
                    }),
                    CliExitCode::RuntimeFailed,
                    output,
                );
            } else {
                crate::errln!("runtime construction error: {e}");
            }
            return CliExitCode::RuntimeFailed.into();
        }
    };
    if let Err(e) = runtime.submit_compiled(run_id, compiled) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("runtime submit error: {e}")
                }),
                CliExitCode::RuntimeFailed,
                output,
            );
        } else {
            crate::errln!("runtime submit error: {e}");
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
                CliExitCode::RuntimeFailed,
                output,
            );
        } else {
            crate::errln!("runtime tick error: {e}");
        }
        return CliExitCode::RuntimeFailed.into();
    }
    let run_elapsed = run_start.elapsed();
    let counters = runtime.counters_snapshot();

    let total_us = compile_elapsed
        .as_micros()
        .saturating_add(run_elapsed.as_micros());

    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
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
        crate::outln!("compile: {}us", compile_elapsed.as_micros());
        crate::outln!("execute: {}us", run_elapsed.as_micros());
        crate::outln!("total:   {}us", total_us);
        crate::outln!(
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

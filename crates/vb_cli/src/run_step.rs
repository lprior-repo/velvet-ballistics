#![forbid(unsafe_code)]
//! Step execution command and helpers.

use std::process::ExitCode;
use std::io::{self, Write};
use std::sync::Arc;
use std::num::NonZeroUsize;
use crate::args::{ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message, write_contract_error_json};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::step_helpers::{decode_step_inputs, execute_step_isolated};

pub(crate) fn cmd_run_step(
    workflow: &std::path::Path,
    durability: DurabilityMode,
    target: &StepTarget,
    output: OutputFormat,
) -> ExitCode {
    if durability != DurabilityMode::None {
        let msg = "step isolation requires --durability none";
        if output != OutputFormat::Text {
            write_contract_error_json(
                &serde_json::json!({
                    "error": "durability_not_none",
                    "message": msg
                }),
                output,
            );
        } else {
            crate::errln!("{msg}");
        }
        return CliExitCode::ValidationFailed.into();
    }
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let step_idx = vb_core::StepIdx::new(target.step_id);
    let node = match compiled.node(step_idx) {
        Some(n) => n,
        None => {
            let msg = format!("step {} not found in workflow", target.step_id);
            if output != OutputFormat::Text {
                write_contract_error_json(
                    &serde_json::json!({
                        "error": "step_not_found",
                        "step": target.step_id,
                        "message": msg
                    }),
                    output,
                );
            } else {
                crate::errln!("{msg}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    };
    let input_data = match read_file(&target.step_input, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };
    let inputs = match decode_step_inputs(&input_data, output) {
        Ok(v) => v,
        Err(code) => return code,
    };
    execute_step_isolated(&compiled, step_idx, node, &inputs, output)
}


pub(crate) fn setup_exit_code() -> ExitCode {
    CliExitCode::VerificationFailed.into()
}


pub(crate) fn compile_bytes_json(
    bytes: &[u8],
    output: OutputFormat,
) -> Result<vb_core::CompiledWorkflow, ExitCode> {
    match vb_compile::compile_workflow(bytes) {
        Ok(c) => Ok(c),
        Err(errors) => {
            if output != OutputFormat::Text {
                write_failure_message(
                    &compile_errors_message(&errors.0),
                    output,
                    CliExitCode::CompileFailed,
                );
            } else {
                for err in &errors.0 {
                    crate::errln!("compile error: {err}");
                }
            }
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}


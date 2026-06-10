#![forbid(unsafe_code)]
//! Run compiled workflow command.

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
use crate::run::{
    INPUT_MAPPING_DECODE_FAILED_MESSAGE, INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
    INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
};
use crate::run_compiled_runtime::{map_runtime_inputs, run_compiled_workflow};
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn cmd_run_compiled(
    vbir_path: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> ExitCode {
    let input_data = match read_file(input_bin, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let ir_bytes = match read_file(vbir_path, output, CliExitCode::CompileFailed) {
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
                            CliExitCode::CompileFailed,
                            output,
                        );
                    } else {
                        crate::errln!("compiled IR validation error: {e}");
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
                        CliExitCode::CompileFailed,
                        output,
                    );
                } else {
                    crate::errln!("error deserializing compiled IR: {e}");
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
                    CliExitCode::CompileFailed,
                    output,
                );
            } else {
                crate::errln!("{error}");
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    run_compiled_workflow(&compiled, inputs, durability, db, output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMappingError {
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

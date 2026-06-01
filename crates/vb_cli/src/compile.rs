#![forbid(unsafe_code)]
//! Workflow compilation command.

use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError, StepTarget,
};
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_contract_error_json, write_failure_message,
    write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn cmd_compile(
    workflow: &std::path::Path,
    emit: EmitTarget,
    out: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::CompileFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
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
                        write_failure_message(
                            &format!("IR serialization error: {e}"),
                            output,
                            CliExitCode::CompileFailed,
                        );
                    } else {
                        crate::errln!("IR serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                if output != OutputFormat::Text {
                    write_failure_message(
                        &format!("error writing {}: {e}", out.display()),
                        output,
                        CliExitCode::CompileFailed,
                    );
                } else {
                    crate::errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                crate::emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "ir"
                    }),
                    output,
                );
            } else {
                crate::outln!("compiled IR written to {}", out.display());
            }
        }
        EmitTarget::Yaml => {
            let parts = compiled.to_parts();
            let yaml_str = match serde_saphyr::to_string(&parts) {
                Ok(s) => s,
                Err(e) => {
                    if output != OutputFormat::Text {
                        write_failure_message(
                            &format!("YAML serialization error: {e}"),
                            output,
                            CliExitCode::CompileFailed,
                        );
                    } else {
                        crate::errln!("YAML serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, yaml_str.as_bytes()) {
                if output != OutputFormat::Text {
                    write_failure_message(
                        &format!("error writing {}: {e}", out.display()),
                        output,
                        CliExitCode::CompileFailed,
                    );
                } else {
                    crate::errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                crate::emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "yaml"
                    }),
                    output,
                );
            } else {
                crate::outln!("compiled YAML written to {}", out.display());
            }
        }
        EmitTarget::Postcard => {
            let parts = compiled.to_parts();
            let encoded = match postcard::to_allocvec(&parts) {
                Ok(data) => data,
                Err(e) => {
                    if output != OutputFormat::Text {
                        write_failure_message(
                            &format!("postcard serialization error: {e}"),
                            output,
                            CliExitCode::CompileFailed,
                        );
                    } else {
                        crate::errln!("postcard serialization error: {e}");
                    }
                    return CliExitCode::CompileFailed.into();
                }
            };
            if let Err(e) = std::fs::write(out, &encoded) {
                if output != OutputFormat::Text {
                    write_failure_message(
                        &format!("error writing {}: {e}", out.display()),
                        output,
                        CliExitCode::CompileFailed,
                    );
                } else {
                    crate::errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                crate::emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "postcard"
                    }),
                    output,
                );
            } else {
                crate::outln!("compiled postcard written to {}", out.display());
            }
        }
    }

    ExitCode::SUCCESS
}

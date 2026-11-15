#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::app_impl::prelude::*;

pub(crate) fn cmd_explain(workflow: &Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(bytes) => bytes,
        Err(code) => return code,
    };

    match vb_compile::compile_workflow(&bytes) {
        Ok(compiled) => {
            match reject_unknown_actions(&compiled, output, CliExitCode::ValidationFailed) {
                Ok(()) => explain_valid(&compiled, &bytes, output),
                Err(code) => code,
            }
        }
        Err(errors) => explain_compile_errors(&errors.0, output),
    }
}

fn explain_valid(
    compiled: &vb_core::CompiledWorkflow,
    source: &[u8],
    output: OutputFormat,
) -> ExitCode {
    let report = match crate::execution_plan::report(compiled, source) {
        Ok(report) => report,
        Err(error) => {
            write_failure_message(&error, output, CliExitCode::ValidationFailed);
            return CliExitCode::ValidationFailed.into();
        }
    };
    emit_plan(&report, output)
}

fn emit_plan(report: &serde_json::Value, output: OutputFormat) -> ExitCode {
    if output == OutputFormat::Text {
        match serde_saphyr::to_string(report) {
            Ok(plan) => outln!("{plan}"),
            Err(error) => {
                write_failure_message(&error.to_string(), output, CliExitCode::ValidationFailed);
                return CliExitCode::ValidationFailed.into();
            }
        }
    } else {
        emit_json_or_return!(&report, output);
    }
    ExitCode::SUCCESS
}

fn explain_compile_errors(errors: &[vb_compile::CompileError], output: OutputFormat) -> ExitCode {
    let message = compile_errors_message(errors);
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        write_failure_message(&message, output, CliExitCode::ValidationFailed);
    }
    CliExitCode::ValidationFailed.into()
}

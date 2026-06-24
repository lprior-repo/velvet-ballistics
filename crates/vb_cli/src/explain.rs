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
        Ok(compiled) => explain_valid(&compiled, output),
        Err(errors) => explain_compile_errors(&errors.0, output),
    }
}

fn explain_valid(compiled: &vb_core::CompiledWorkflow, output: OutputFormat) -> ExitCode {
    if output == OutputFormat::Text {
        outln!("valid workflow");
        outln!("nodes: {}", compiled.node_count());
        outln!("entry step: {}", compiled.entry().get());
        return ExitCode::SUCCESS;
    }

    emit_json_or_return!(
        &serde_json::json!({
            "schema_version": cli_envelope::SCHEMA_VERSION,
            "kind": "explain_report",
            "success": true,
            "status": "valid",
            "node_count": compiled.node_count(),
            "entry": compiled.entry().get(),
            "artifact": {
                "node_count": compiled.node_count(),
                "entry": compiled.entry().get()
            },
            "repair_hints": []
        }),
        output,
    );
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

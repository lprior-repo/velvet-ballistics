#![forbid(unsafe_code)]
//! Workflow validation command and helpers.

pub(crate) fn cmd_validate(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            write_failure_message(
                &format!("file is not valid UTF-8: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Phase 1: strict YAML profile and AST parse via vb_yaml
    match vb_yaml::parse_workflow_source(text) {
        Ok(_ast) => {}
        Err(e) => {
            write_failure_message(
                &format!("YAML parse error: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    }

    // Phase 2: full compilation pipeline (schema, references, control flow, type/taint)
    match vb_compile::compile_workflow(&bytes) {
        Ok(_compiled) => {}
        Err(errors) => {
            let message = compile_errors_message(&errors.0);
            write_failure_message(&message, output, CliExitCode::ValidationFailed);
            return CliExitCode::ValidationFailed.into();
        }
    }

    if output == OutputFormat::Text {
        outln!("valid");
    } else {
        emit_json_or_return!(&validate_success_report(), output);
    }
    ExitCode::SUCCESS
}


pub(crate) fn validate_success_report() -> serde_json::Value {
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "validate_report",
        "success": true,
        "status": "valid",
        "exit_code": cli_exit_code_number(CliExitCode::Success),
        "repair_hints": []
    })
}


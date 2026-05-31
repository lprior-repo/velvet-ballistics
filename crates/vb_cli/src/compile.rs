//! Workflow compilation command.
    match err {
        commands_verify::VerifyError::YamlParse(msg)
        | commands_verify::VerifyError::IrValidation(msg)
        | commands_verify::VerifyError::BudgetPolicy(msg)
        | commands_verify::VerifyError::StorageError(msg)
        | commands_verify::VerifyError::ReplayDivergence(msg) => msg.clone(),
        commands_verify::VerifyError::Compile(errors) => {
            let mut message = String::from("compilation failed");
            for error in errors {
                message.push_str("; compile error: ");
                message.push_str(error);
            }
            message
        }
    }
}

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
                    errln!("compile error: {err}");
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
                        errln!("IR serialization error: {e}");
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
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "ir"
                    }),
                    output,
                );
            } else {
                outln!("compiled IR written to {}", out.display());
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
                        errln!("YAML serialization error: {e}");
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
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "yaml"
                    }),
                    output,
                );
            } else {
                outln!("compiled YAML written to {}", out.display());
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
                        errln!("postcard serialization error: {e}");
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
                    errln!("error writing {}: {e}", out.display());
                }
                return CliExitCode::CompileFailed.into();
            }
            if output != OutputFormat::Text {
                emit_json_or_return!(
                    &serde_json::json!({
                        "success": true,
                        "output": out.display().to_string(),
                        "format": "postcard"
                    }),
                    output,
                );
            } else {
                outln!("compiled postcard written to {}", out.display());
            }
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn cmd_run(

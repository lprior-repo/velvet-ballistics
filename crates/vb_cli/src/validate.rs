//! Workflow validation command and helpers.
        Err(err) => {
            let code = commands_verify::exit_code_for_error(&err);
            if output != OutputFormat::Text {
                write_failure_message(&verify_error_message(&err), output, code);
                return code.into();
            }
            match &err {
                commands_verify::VerifyError::YamlParse(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::Compile(errors) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": "compilation failed",
                                "errors": errors
                            }),
                            output,
                        );
                    } else {
                        for e in errors {
                            errln!("compile error: {e}");
                        }
                    }
                }
                commands_verify::VerifyError::IrValidation(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::BudgetPolicy(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::StorageError(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
                commands_verify::VerifyError::ReplayDivergence(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
            }
            code.into()
        }
    }
}

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

pub(crate) fn verify_success_report(
    result: &commands_verify::VerifyOk,
    profile: VerifyProfile,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "verify_report",
        "success": true,
        "profile": profile.as_str(),
        "digest": result.digest_hex.as_str(),
        "node_count": result.node_count,
        "checks": &result.checks,
        "warnings": &result.warnings,
        "artifact": {
            "source_digest_hex": result.digest_hex.as_str(),
            "ir_digest_hex": result.digest_hex.as_str(),
            "node_count": result.node_count
        },
        "replay": {
            "gates_passed": &result.checks,
            "gate_sequence": &result.checks,
            "replay_safe": true
        },
        "durability": {
            "profile": "none",
            "journal_written": false
        },
        "repair_hints": [],
        "exit_code": cli_exit_code_number(CliExitCode::Success)
    })
}

pub(crate) fn verify_error_message(err: &commands_verify::VerifyError) -> String {

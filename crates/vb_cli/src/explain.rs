fn cmd_explain(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
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

    // Phase 1: YAML parse
    if let Err(e) = vb_yaml::parse_workflow_source(text) {
        if output == OutputFormat::Text {
            outln!("YAML Parse Error:");
            outln!("  {e}");
            outln!("");
            explain_repair_hint(
                "yaml_parse",
                &[
                    "Check YAML syntax: use spaces for indentation, not tabs",
                    "Ensure all quotes are matched",
                    "Verify the file uses valid UTF-8 encoding",
                ],
            );
        } else {
            emit_json_or_return!(
                &explain_failure_report(
                    "yaml_parse",
                    &format!("YAML parse error: {e}"),
                    &["Check YAML syntax: use spaces for indentation, not tabs"],
                    CliExitCode::ValidationFailed,
                ),
                output,
            );
        }
        return CliExitCode::ValidationFailed.into();
    }

    // Phase 2: Compilation
    match vb_compile::compile_workflow(&bytes) {
        Ok(_) => {}
        Err(errors) => {
            if output == OutputFormat::Text {
                outln!("Workflow has {} validation error(s):", errors.0.len());
                outln!("");
                for (i, err) in errors.0.iter().enumerate() {
                    if i > 0 {
                        outln!("---");
                    }
                    explain_error(err);
                }
            } else {
                let error_messages: Vec<String> = errors
                    .0
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                emit_json_or_return!(&explain_compile_failure_report(&error_messages), output);
            }
            return CliExitCode::ValidationFailed.into();
        }
    }

    // Phase 3: Verification (runs all gates)
    match commands_verify::run_verification(text, &bytes, VerifyProfile::Standard) {
        Ok(result) => {
            if output == OutputFormat::Text {
                outln!("Workflow verification certificate:");
                outln!("  digest:  {}", result.digest_hex);
                outln!("  nodes:   {}", result.node_count);
                outln!("");
                outln!("Passed gates ({}):", result.checks.len());
                for check in &result.checks {
                    explain_gate_pass(check);
                }
                if !result.warnings.is_empty() {
                    outln!("");
                    outln!("Warnings ({}):", result.warnings.len());
                    for warning in &result.warnings {
                        outln!("  - {warning}");
                    }
                    outln!("");
                    explain_repair_hint(
                        "verification_warnings",
                        &[
                            "Review warnings and address them before production use",
                            "Use 'vb verify --profile full' for exhaustive validation",
                        ],
                    );
                }
                outln!("All gates passed. Workflow is correct and verifiable.");
            } else {
                emit_json_or_return!(&explain_success_report(&result), output);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = commands_verify::exit_code_for_error(&err);
            if output == OutputFormat::Text {
                explain_verification_failure(&err);
            } else {
                emit_json_or_return!(&explain_verification_failure_report(&err, code), output);
            }
            code.into()
        }
    }
}

fn explain_failure_report(
    phase: &'static str,
    message: &str,
    repair_hints: &[&'static str],
    code: CliExitCode,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": false,
        "status": "invalid",
        "phase": phase,
        "errors": [{ "phase": phase, "message": message }],
        "repair_hints": repair_hints,
        "exit_code": cli_exit_code_number(code)
    })
}

fn explain_compile_failure_report(errors: &[String]) -> serde_json::Value {
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": false,
        "status": "invalid",
        "phase": "compile",
        "errors": errors,
        "repair_hints": ["Run validate to isolate syntax and schema errors"],
        "exit_code": cli_exit_code_number(CliExitCode::ValidationFailed)
    })
}

fn explain_success_report(result: &commands_verify::VerifyOk) -> serde_json::Value {
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": true,
        "status": "valid",
        "artifact": {
            "ir_digest_hex": result.digest_hex.as_str(),
            "node_count": result.node_count
        },
        "passed_gates": &result.checks,
        "warnings": &result.warnings,
        "repair_hints": [],
        "exit_code": cli_exit_code_number(CliExitCode::Success)
    })
}

fn explain_verification_failure_report(
    err: &commands_verify::VerifyError,
    code: CliExitCode,
) -> serde_json::Value {
    let message = verify_error_message(err);
    explain_failure_report(
        "verification",
        &message,
        &["Run verify --profile full for details"],
        code,
    )
}


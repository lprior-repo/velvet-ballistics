#![forbid(unsafe_code)]
//! Step execution command and helpers.

fn cmd_run_step(
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
            errln!("{msg}");
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
                errln!("{msg}");
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


fn setup_exit_code() -> ExitCode {
    CliExitCode::VerificationFailed.into()
}


fn compile_bytes_json(
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
                    errln!("compile error: {err}");
                }
            }
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}


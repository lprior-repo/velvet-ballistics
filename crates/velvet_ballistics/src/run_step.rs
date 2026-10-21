//! Module: run_step

use crate::app_impl::prelude::*;

/// Executes a single step in isolation using `step_once`.
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

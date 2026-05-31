//! Single-step run command and step helpers.
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

pub(crate) fn setup_exit_code() -> ExitCode {
    CliExitCode::VerificationFailed.into()
}

pub(crate) fn compile_bytes_json(
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

pub(crate) fn decode_step_inputs(
    data: &[u8],
    output: OutputFormat,
) -> Result<Box<[vb_core::SlotValue]>, ExitCode> {
    if data.is_empty() {
        return Ok(Box::from([]));
    }
    match postcard::from_bytes::<Box<[vb_core::SlotValue]>>(data) {
        Ok(values) => Ok(values),
        Err(e) => {
            let msg = format!("step-input decode error: {e}");
            if output != OutputFormat::Text {
                write_contract_error_json(
                    &serde_json::json!({
                        "error": "step_input_decode_error",
                        "message": msg
                    }),
                    output,
                );
            } else {
                errln!("{msg}");
            }
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

pub(crate) fn execute_step_isolated(
    compiled: &vb_core::CompiledWorkflow,
    step_idx: vb_core::StepIdx,
    node: &vb_core::workflow::CompiledNode,
    inputs: &[vb_core::SlotValue],
    output: OutputFormat,
) -> ExitCode {
    let mut frame = match build_step_frame(compiled, step_idx) {
        Ok(f) => f,
        Err(code) => return code,
    };
    if let Err(code) = write_step_inputs(&mut frame, inputs) {
        return code;
    }

    // Capture before state for delta computation
    let before_pc = frame.pc();
    let before_slots = frame.slots_snapshot();
    let before_taint = frame.taint_snapshot();
    let before_states = frame.states_snapshot();

    let mut store = vb_core::ValueStore::new();
    let signal = match vb_core::step_once(compiled, &mut frame, &mut store) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("step error: {e}");
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "error": error_name(&e),
                        "message": msg
                    }),
                    output,
                );
            } else {
                errln!("{msg}");
            }
            return CliExitCode::RuntimeFailed.into();
        }
    };

    // Capture after state for delta computation
    let after_pc = frame.pc();
    let after_slots = frame.slots_snapshot();
    let after_taint = frame.taint_snapshot();
    let after_states = frame.states_snapshot();

    // Compute deltas
    let pc_delta = serde_json::json!({
        "before": before_pc.get(),
        "after": after_pc.get()
    });
    let slot_deltas = compute_slot_deltas(&before_slots, &after_slots);
    let taint_deltas = compute_taint_deltas(&before_taint, &after_taint);
    let state_deltas = compute_state_deltas(&before_states, &after_states);

    let deltas = serde_json::json!({
        "pc_delta": pc_delta,
        "slot_deltas": slot_deltas,
        "taint_deltas": taint_deltas,
        "state_deltas": state_deltas
    });

    // Build state snapshots for structured output
    let snapshots = StepStateSnapshots {
        before_pc,
        after_pc,
        before_slots,
        after_slots,
        before_taint,
        after_taint,
        before_states,
        after_states,
    };

    match print_step_result(step_idx, node, &frame, &signal, output, deltas, snapshots) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output_error_exit(&error),
    }
}

pub(crate) fn build_step_frame(

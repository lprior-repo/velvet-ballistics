//! Module: step_helpers

use crate::app_impl::prelude::*;

pub(crate) struct StepStateSnapshots {
    pub(crate) before_pc: vb_core::StepIdx,
    pub(crate) after_pc: vb_core::StepIdx,
    pub(crate) before_slots: Vec<Option<vb_core::SlotValue>>,
    pub(crate) after_slots: Vec<Option<vb_core::SlotValue>>,
    pub(crate) before_taint: Vec<vb_core::Taint>,
    pub(crate) after_taint: Vec<vb_core::Taint>,
    pub(crate) before_states: Vec<vb_core::frame::StepState>,
    pub(crate) after_states: Vec<vb_core::frame::StepState>,
}

impl StepStateSnapshots {
    pub(crate) fn to_before_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pc": self.before_pc.get(),
            "slots": self.before_slots,
            "taint": self.before_taint,
            "states": self.before_states,
        })
    }

    pub(crate) fn to_after_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pc": self.after_pc.get(),
            "slots": self.after_slots,
            "taint": self.after_taint,
            "states": self.after_states,
        })
    }
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
    compiled: &vb_core::CompiledWorkflow,
    step_idx: vb_core::StepIdx,
) -> Result<vb_core::RunFrame, ExitCode> {
    let step_count = compiled.node_count();
    let slot_count = compiled.slot_count();
    let run_id = vb_core::RunId::new(0);
    match vb_core::RunFrame::new(run_id, step_idx, step_count, slot_count) {
        Ok(frame) => Ok(frame),
        Err(e) => {
            errln!("frame build error: {e}");
            Err(setup_exit_code())
        }
    }
}

pub(crate) fn write_step_inputs(
    frame: &mut vb_core::RunFrame,
    inputs: &[vb_core::SlotValue],
) -> Result<(), ExitCode> {
    for (i, value) in inputs.iter().enumerate() {
        if let Ok(slot) = u16::try_from(i) {
            let slot_idx = vb_core::SlotIdx::new(slot);
            if let Err(error) = frame.write_slot(slot_idx, *value) {
                errln!("step input write error: {error}");
                return Err(setup_exit_code());
            }
        }
    }
    Ok(())
}

pub(crate) fn compute_slot_deltas(
    before: &[Option<vb_core::SlotValue>],
    after: &[Option<vb_core::SlotValue>],
) -> Vec<serde_json::Value> {
    let mut deltas = Vec::new();
    let len = usize::min(before.len(), after.len());
    for i in 0..len {
        if before.get(i) != after.get(i) {
            deltas.push(serde_json::json!({
                "slot": i,
                "before": before.get(i),
                "after": after.get(i)
            }));
        }
    }
    deltas
}

pub(crate) fn compute_taint_deltas(
    before: &[vb_core::Taint],
    after: &[vb_core::Taint],
) -> Vec<serde_json::Value> {
    let mut deltas = Vec::new();
    let len = usize::min(before.len(), after.len());
    for i in 0..len {
        if before.get(i) != after.get(i) {
            deltas.push(serde_json::json!({
                "slot": i,
                "before": before.get(i),
                "after": after.get(i)
            }));
        }
    }
    deltas
}

pub(crate) fn compute_state_deltas(
    before: &[vb_core::frame::StepState],
    after: &[vb_core::frame::StepState],
) -> Vec<serde_json::Value> {
    let mut deltas = Vec::new();
    let len = usize::min(before.len(), after.len());
    for i in 0..len {
        if before.get(i) != after.get(i) {
            deltas.push(serde_json::json!({
                "step": i,
                "before": before.get(i),
                "after": after.get(i)
            }));
        }
    }
    deltas
}

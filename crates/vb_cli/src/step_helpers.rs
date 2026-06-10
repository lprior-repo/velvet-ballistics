#![forbid(unsafe_code)]
//! Step execution helpers for building frames and formatting output.

use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, EventStatus, OutputFormat, ParseError, StepTarget,
};
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    OutputError, json_error, json_out, output_error_exit, write_contract_error_json,
    write_failure_message, write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;

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
                crate::errln!("{msg}");
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
                    CliExitCode::RuntimeFailed,
                    output,
                );
            } else {
                crate::errln!("{msg}");
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
            crate::errln!("frame build error: {e}");
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
                crate::errln!("step input write error: {error}");
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

/// Captures before/after state snapshots for structured output.
pub(crate) struct StepStateSnapshots {
    before_pc: vb_core::StepIdx,
    after_pc: vb_core::StepIdx,
    before_slots: Vec<Option<vb_core::SlotValue>>,
    after_slots: Vec<Option<vb_core::SlotValue>>,
    before_taint: Vec<vb_core::Taint>,
    after_taint: Vec<vb_core::Taint>,
    before_states: Vec<vb_core::frame::StepState>,
    after_states: Vec<vb_core::frame::StepState>,
}

impl StepStateSnapshots {
    pub(crate) fn to_before_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pc": self.before_pc.get(),
            "slots": self.before_slots,
            "taint": self.before_taint,
            "states": self.before_states
        })
    }

    pub(crate) fn to_after_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pc": self.after_pc.get(),
            "slots": self.after_slots,
            "taint": self.after_taint,
            "states": self.after_states
        })
    }
}

pub(crate) fn build_step_result_json(
    step: vb_core::StepIdx,
    node: &vb_core::workflow::CompiledNode,
    frame: &vb_core::RunFrame,
    signal: &vb_core::EngineSignal,
    deltas: serde_json::Value,
    before: serde_json::Value,
    after: serde_json::Value,
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    // vb-k8ut.5: the envelope `kind` field is the typed `CliPostcardKind`
    // discriminant, NOT the workflow `CompiledNodeKind` name. Setting
    // `kind` to e.g. "SetConst" makes the postcard classifier reject the
    // envelope with "unknown envelope kind: SetConst" because node kind
    // names are not in the `CliPostcardKind` taxonomy. The actual node
    // kind is preserved under `node_kind` so downstream consumers can
    // still inspect it.
    map.insert("kind".to_string(), serde_json::json!("run_report"));
    map.insert(
        "node_kind".to_string(),
        serde_json::json!(node_kind_name(&node.kind)),
    );
    map.insert("step".to_string(), serde_json::json!(step.get()));
    map.insert("signal".to_string(), serde_json::json!(signal_name(signal)));
    map.insert("before".to_string(), before);
    map.insert("after".to_string(), after);
    map.insert("deltas".to_string(), deltas);

    // Add output slot if present
    #[allow(clippy::collapsible_if)]
    if let Some(output_slot) = node.output {
        if let (Ok(value), Ok(taint)) =
            (frame.read_slot(output_slot), frame.read_taint(output_slot))
        {
            let mut output_map = serde_json::Map::new();
            output_map.insert("slot".to_string(), serde_json::json!(output_slot.get()));
            output_map.insert("value".to_string(), serde_json::json!(value));
            output_map.insert("taint".to_string(), serde_json::json!(taint));
            map.insert(
                "output_slot".to_string(),
                serde_json::Value::Object(output_map),
            );
        }
    }

    serde_json::Value::Object(map)
}

pub(crate) fn error_name(error: &vb_core::EngineError) -> &'static str {
    match error {
        vb_core::EngineError::InvalidProgramCounter { .. } => "invalid_program_counter",
        vb_core::EngineError::MissingNextStep { .. } => "missing_next_step",
        vb_core::EngineError::SlotOutOfBounds { .. } => "slot_out_of_bounds",
        vb_core::EngineError::SlotUninitialized { .. } => "slot_uninitialized",
        vb_core::EngineError::MissingOutputSlot { .. } => "missing_output_slot",
        vb_core::EngineError::StepStateOutOfBounds { .. } => "step_state_out_of_bounds",
        vb_core::EngineError::TypeMismatch { .. } => "type_mismatch",
        vb_core::EngineError::DivisionByZero => "division_by_zero",
        vb_core::EngineError::NonFiniteNumber => "non_finite_number",
        vb_core::EngineError::ResourceLimitExceeded { .. } => "resource_limit_exceeded",
        vb_core::EngineError::BudgetParse { .. } => "budget_parse_error",
        vb_core::EngineError::StepCounterOverflow => "step_counter_overflow",
        vb_core::EngineError::UnsupportedPrimitive { .. } => "unsupported_primitive",
        _ => "internal_error",
    }
}

pub(crate) fn print_step_result(
    step: vb_core::StepIdx,
    node: &vb_core::workflow::CompiledNode,
    frame: &vb_core::RunFrame,
    signal: &vb_core::EngineSignal,
    output: OutputFormat,
    deltas: serde_json::Value,
    snapshots: StepStateSnapshots,
) -> Result<(), OutputError> {
    match output {
        OutputFormat::Text => {
            crate::outln!("step: {}", step.get());
            crate::outln!("kind: {}", node_kind_name(&node.kind));
            print_input_slots(frame);
            if let Some(output_slot) = node.output {
                print_output_slot(frame, output_slot);
            }
            crate::outln!("signal: {}", signal_name(signal));
            if let Some(output_slot) = node.output {
                print_taint(frame, output_slot);
            }
            Ok(())
        }
        OutputFormat::Yaml | OutputFormat::Postcard => {
            let json = build_step_result_json(
                step,
                node,
                frame,
                signal,
                deltas,
                snapshots.to_before_json(),
                snapshots.to_after_json(),
            );
            json_out(&json, output)
        }
    }
}

pub(crate) fn print_input_slots(frame: &vb_core::RunFrame) {
    let count = frame.slot_count();
    for i in 0..count {
        let slot = vb_core::SlotIdx::new(i);
        if let Ok(value) = frame.read_slot(slot) {
            crate::outln!("  slot[{i}]: {value:?}");
        }
    }
}

pub(crate) fn print_output_slot(frame: &vb_core::RunFrame, slot: vb_core::SlotIdx) {
    if let Ok(value) = frame.read_slot(slot) {
        crate::outln!("output: {value:?}");
    }
}

pub(crate) fn print_taint(frame: &vb_core::RunFrame, slot: vb_core::SlotIdx) {
    if let Ok(taint) = frame.read_taint(slot) {
        crate::outln!("taint: {taint:?}");
    }
}

pub(crate) fn node_kind_name(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
    match kind {
        vb_core::workflow::CompiledNodeKind::Nop => "Nop",
        vb_core::workflow::CompiledNodeKind::SetConst { .. } => "SetConst",
        vb_core::workflow::CompiledNodeKind::Copy { .. } => "Copy",
        vb_core::workflow::CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        vb_core::workflow::CompiledNodeKind::BuildObject { .. } => "BuildObject",
        vb_core::workflow::CompiledNodeKind::BuildList { .. } => "BuildList",
        vb_core::workflow::CompiledNodeKind::Do { .. } => "Do",
        vb_core::workflow::CompiledNodeKind::Choose { .. } => "Choose",
        vb_core::workflow::CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        vb_core::workflow::CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        vb_core::workflow::CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        vb_core::workflow::CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        vb_core::workflow::CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        vb_core::workflow::CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        vb_core::workflow::CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        vb_core::workflow::CompiledNodeKind::CollectStart { .. } => "CollectStart",
        vb_core::workflow::CompiledNodeKind::CollectPage { .. } => "CollectPage",
        vb_core::workflow::CompiledNodeKind::CollectNext { .. } => "CollectNext",
        vb_core::workflow::CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        vb_core::workflow::CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        vb_core::workflow::CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        vb_core::workflow::CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        vb_core::workflow::CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        vb_core::workflow::CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        vb_core::workflow::CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        vb_core::workflow::CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        vb_core::workflow::CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        vb_core::workflow::CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        vb_core::workflow::CompiledNodeKind::Ask { .. } => "Ask",
        vb_core::workflow::CompiledNodeKind::AskResume { .. } => "AskResume",
        vb_core::workflow::CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        _ => "Unknown",
    }
}

pub(crate) fn signal_name(signal: &vb_core::EngineSignal) -> &'static str {
    match signal {
        vb_core::EngineSignal::Continue => "Continue",
        vb_core::EngineSignal::Finished(_, _) => "Finished",
        vb_core::EngineSignal::StepBudgetExhausted => "StepBudgetExhausted",
        vb_core::EngineSignal::AwaitingAction => "AwaitingAction",
        vb_core::EngineSignal::AwaitingWait { .. } => "AwaitingWait",
        vb_core::EngineSignal::AwaitingAsk { timeout_slot: None } => "AwaitingAsk",
        _ => "Unknown",
    }
}

fn setup_exit_code() -> std::process::ExitCode {
    std::process::ExitCode::FAILURE
}

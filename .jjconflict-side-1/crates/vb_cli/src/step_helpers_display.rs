//! Step result display helpers: JSON building, error naming, text printing.

use crate::args::OutputFormat;
use crate::step_helpers::StepStateSnapshots;
use crate::{OutputError, json_out};

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
    map.insert("step".to_string(), serde_json::json!(step.get()));
    map.insert(
        "kind".to_string(),
        serde_json::json!(node_kind_name(&node.kind)),
    );
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
            outln!("step: {}", step.get());
            outln!("kind: {}", node_kind_name(&node.kind));
            print_input_slots(frame);
            if let Some(output_slot) = node.output {
                print_output_slot(frame, output_slot);
            }
            outln!("signal: {}", signal_name(signal));
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
            outln!("  slot[{i}]: {value:?}");
        }
    }
}

pub(crate) fn print_output_slot(frame: &vb_core::RunFrame, slot: vb_core::SlotIdx) {
    if let Ok(value) = frame.read_slot(slot) {
        outln!("output: {value:?}");
    }
}

pub(crate) fn print_taint(frame: &vb_core::RunFrame, slot: vb_core::SlotIdx) {
    if let Ok(taint) = frame.read_taint(slot) {
        outln!("taint: {taint:?}");
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
        vb_core::EngineSignal::AwaitingWait => "AwaitingWait",
        vb_core::EngineSignal::AwaitingAsk => "AwaitingAsk",
        _ => "Unknown",
    }
}

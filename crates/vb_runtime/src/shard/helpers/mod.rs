#![forbid(unsafe_code)]
//! Pure helper functions for shard operations.

use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::primitives::collect::CollectStates;

use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::InspectSnapshot;

/// Seeds input slots on a frame before deterministic execution.
pub fn seed_input_slots(
    frame: &mut RunFrame,
    inputs: &[(SlotIdx, SlotValue)],
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    let inputs = prepare_input_slots(inputs, workflow)?;
    if inputs.iter().any(|(slot, _)| slot.get() >= frame.slot_count()) {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    for (slot, value) in inputs {
        let secret = workflow.input_slots().iter()
            .any(|input| input.slot == slot && input.constraint.secret);
        let taint = if secret { Taint::Secret } else { Taint::Clean };
        frame.write_slot_with_taint(slot, value, taint)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
    }
    Ok(())
}

pub(crate) fn prepare_input_slots(
    inputs: &[(SlotIdx, SlotValue)],
    workflow: &CompiledWorkflow,
) -> RuntimeResult<Vec<(SlotIdx, SlotValue)>> {
    for (slot, value) in inputs {
        if let Some(input) = workflow.input_slots().iter().find(|input| input.slot == *slot)
            && vb_core::input_validation::validate_input_value(
                input, *value, workflow.symbols_count(), None,
            ) == Err(vb_core::input_validation::InputValueIssue::Kind)
        {
            return Err(RuntimeError::RuntimeInputSlotKindMismatch {
                slot: *slot, expected: input.kind,
                actual: vb_core::input_validation::input_value_kind(*value),
            });
        }
    }
    vb_storage::inputs::prepare_scalar_inputs(workflow, inputs)
        .map_err(|_| RuntimeError::UnsupportedOperation { operation: "input_schema_mismatch" })
}

pub fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}

pub fn make_run_state(
    workflow: CompiledWorkflow,
    run_id: RunId,
) -> Option<crate::shard::types::RunState> {
    let step_count = workflow.node_count();
    let slot_count = workflow.slot_count();
    let frame = RunFrame::new(run_id, workflow.entry(), step_count, slot_count).ok()?;
    Some(crate::shard::types::RunState {
        registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
        frame,
        run_attempt: core::num::NonZeroU16::MIN,
        workflow,
        store: ValueStore::new(),
        action_attempts: new_action_attempts(step_count),
        admission: None,
        run_started_millis: None,
        collect_states: CollectStates::new(),
        journal_bytes_written: 0,
        action_contracts: Box::new([]),
    })
}

/// Returns the result slot for a finished run.
pub fn result_slot_for_finished_run(state: &crate::shard::types::RunState) -> Option<SlotIdx> {
    state
        .workflow
        .node(state.frame.pc())
        .and_then(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
}

/// Creates a snapshot from run state.
pub fn snapshot_from_state(
    run: RunId,
    correlation: u64,
    state: &crate::shard::types::RunState,
) -> InspectSnapshot {
    InspectSnapshot {
        run,
        correlation,
        pc: state.frame.pc(),
        executed: state.frame.executed(),
    }
}

pub mod action;
pub mod error_handler;
pub mod retry;
pub mod timer;

pub use action::{
    action_input_slot, action_output_slot, advance_after_action_completion,
    normalize_scheduled_ticket, record_scheduled_attempt, validate_action_completion,
};
pub use error_handler::find_error_handler_for_failure;
pub use retry::{record_retry_attempt, retry_metadata_exists, retry_policy_after_action};
pub use timer::{
    advance_after_timer_fire, timer_registration_required, validate_timer_fire,
    wall_clock_millis_since_epoch,
};

#[cfg(test)]
mod tests;

#![forbid(unsafe_code)]
//! Integration tests for the engine.

use crate::engine::new_run_frame;
use crate::ids::RunId;
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledWorkflow, InputSlotKind,
};

mod integration_accessor;
mod integration_basic;
mod integration_budget;
mod integration_capability_behavior;
mod integration_choose;
mod integration_error_routing_behavior;
mod integration_eval;
mod integration_frame_behavior;
mod integration_taint_propagation;
mod integration_workflow;

pub(super) fn external_input() -> Box<[CompiledInputSlot]> {
    Box::new([CompiledInputSlot {
        slot: crate::ids::SlotIdx::new(0),
        kind: InputSlotKind::Any,
        constraint: CompiledInputConstraint::required(),
    }])
}

pub(super) fn test_store() -> ValueStore {
    ValueStore::new()
}

pub(super) fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

pub(super) fn test_frame(
    run_id: RunId,
    workflow: &CompiledWorkflow,
) -> Result<crate::RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
}

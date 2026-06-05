#![forbid(unsafe_code)]
//! Kani model-checking harnesses for the replay module.
//!
//! Each harness is gated behind `#[cfg(kani)]` so that `cargo kani`
//! picks it up as a proof obligation while regular `cargo test` skips it.

use crate::errors::CoreError;
use crate::ids::StepIdx;
use crate::value::ConstValue;
use crate::workflow::{CompiledNode, ResourceContract, WorkflowParts};

fn make_minimal_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "kani_plan".into(),
        digest: crate::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: vec![].into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "kani test workflow validation failed",
    })
}

#[cfg(kani)]
#[path = "kani_choose_slot.rs"]
mod kani_choose_slot;
#[cfg(kani)]
#[path = "kani_terminal_states.rs"]
mod kani_terminal_states;

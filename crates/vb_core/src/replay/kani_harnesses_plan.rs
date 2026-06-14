#![forbid(unsafe_code)]

use crate::errors::CoreError;
use crate::ids::StepIdx;
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ResourceContract, SlotBranch, WorkflowParts,
};

pub(crate) fn make_minimal_plan(
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

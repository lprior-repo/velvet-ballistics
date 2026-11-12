#![forbid(unsafe_code)]

//! Tests for [`execute_node_full`] and the execute module.

use super::*;
use crate::primitives::collect::CollectStates;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::CapabilitySet;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, NodeOutput, WorkflowParts,
};

/// Terminal Finish node. Valid sink for any workflow path.
fn finish_node(id: u16, slot: u16) -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(id),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(slot),
        },
        Some(SlotIdx::new(slot)),
        None,
    )
}

/// Nop node that jumps forward to `next_id`. Used as filler.
fn nop_forward(id: u16, next_id: u16) -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(id),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(next_id)),
    )
}

/// Build a workflow from nodes. Panics on validation failure so tests
/// fail loudly instead of silently skipping.
fn make_workflow(nodes: Vec<CompiledNode>, slot_count: u16, inputs: &[u16]) -> CompiledWorkflow {
    make_workflow_with_constants(nodes, slot_count, Box::from([]), inputs)
}

/// Build a workflow with a non-empty constants pool.
fn make_workflow_with_constants(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    constants: Box<[vb_core::value::ConstValue]>,
    inputs: &[u16],
) -> CompiledWorkflow {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("test_exec"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: inputs
            .iter()
            .map(|slot| vb_core::CompiledInputSlot {
                slot: SlotIdx::new(*slot),
                kind: vb_core::InputSlotKind::Any,
                constraint: vb_core::CompiledInputConstraint::required(),
            })
            .collect(),
        step_names: Box::from([]),
    };
    match CompiledWorkflow::try_from_parts(parts) {
        Ok(w) => w,
        Err(e) => {
            let msg = format!("workflow validation failed: {e}");
            panic!("{msg}");
        }
    }
}

fn make_run(slot_count: u16, step_state_count: u16) -> RunFrame {
    match RunFrame::new(RunId::new(1), StepIdx::new(0), step_state_count, slot_count) {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("RunFrame::new failed: {e}");
            panic!("{msg}");
        }
    }
}

include!("execute_tests/execute_dispatch.rs");
include!("execute_tests/execute_collect_reduce.rs");
include!("execute_tests/execute_repeat_wait.rs");

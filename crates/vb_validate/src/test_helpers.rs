//! Shared test helpers for validation gate tests.
//!
//! These helpers are duplicated across multiple gate test modules. Extract them
//! here to avoid copy-paste maintenance burden.

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts, node};

pub(crate) fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

pub(crate) fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    node(
        StepIdx::new(index),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
        None,
        None,
    )
}

pub(crate) fn nop_node(index: u16) -> CompiledNode {
    node(
        StepIdx::new(index),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(index.saturating_add(1))),
    )
}

pub(crate) fn copy_node(index: u16, source: u16, output: u16) -> CompiledNode {
    node(
        StepIdx::new(index),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(source),
        },
        Some(SlotIdx::new(output)),
        Some(StepIdx::new(index.saturating_add(1))),
    )
}

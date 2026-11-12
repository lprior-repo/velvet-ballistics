#![forbid(unsafe_code)]
//! Shared node/part builders for validation integration tests.
//!
//! Consolidates the duplicated `make_parts` / `finish_node` / `nop_node`
//! copies (bead vb-9a2f7 Slice A). Variants with distinct routing semantics
//! stay local to their test file.

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprProgram, ResourceContract, WorkflowParts,
    node,
};

pub(crate) fn make_parts_full(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    symbols_count: u32,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    constants: Vec<ConstValue>,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: accessors.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count,
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

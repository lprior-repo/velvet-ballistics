//! Shared test helpers for validation gate tests.
//!
//! These helpers are duplicated across multiple gate test modules. Extract them
//! here to avoid copy-paste maintenance burden.

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, ErrorRouting, NodeContinuation, NodeOutput, NodeRouting,
    ResourceContract, WorkflowParts,
};

pub(crate) fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
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
    CompiledNode {
        id: StepIdx::new(index),
        output: NodeOutput::Discard,
        routing: NodeRouting {
            next: NodeContinuation::End,
            error: ErrorRouting::None,
        },
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

pub(crate) fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: NodeOutput::Discard,
        routing: NodeRouting {
            next: NodeContinuation::Step(StepIdx::new(index.saturating_add(1))),
            error: ErrorRouting::None,
        },
        kind: CompiledNodeKind::Nop,
    }
}

pub(crate) fn copy_node(index: u16, source: u16, output: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: NodeOutput::Slot(SlotIdx::new(output)),
        routing: NodeRouting {
            next: NodeContinuation::Step(StepIdx::new(index.saturating_add(1))),
            error: ErrorRouting::None,
        },
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(source),
        },
    }
}

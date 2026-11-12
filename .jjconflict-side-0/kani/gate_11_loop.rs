//! Kani harnesses for Gate 11 - Loop body graph well-formed.
//!
//! K13: ForEach body graph well-formed
//! K14: Together body graph well-formed

#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::gates::validate_gate_11_loop_body_graph;

/// K13: ForEachStart body subgraph leads to ForEachJoin.
#[kani::proof]
fn kani_gate_11_foreach_body_well_formed() {
    let body_idx: u16 = kani::any();
    let done_idx: u16 = kani::any();

    kani::assume(body_idx > 0);
    kani::assume(body_idx < done_idx);
    kani::assume(done_idx <= 20);

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(body_idx),
                done: StepIdx::new(done_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(body_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(body_idx),
                done: StepIdx::new(done_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(done_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let parts = WorkflowParts {
        name: Box::from("kani_g11_foreach"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_11_loop_body_graph(&parts);

    kani::assert(
        result.is_ok(),
        "ForEach body graph with body < done should be well-formed",
    );
}

/// K14: TogetherStart body subgraph leads to TogetherJoin.
#[kani::proof]
fn kani_gate_11_together_body_well_formed() {
    let join_idx: u16 = kani::any();

    kani::assume(join_idx > 0);
    kani::assume(join_idx <= 20);

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(join_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                accumulator: SlotIdx::new(0),
                entry: StepIdx::new(1),
                join: StepIdx::new(join_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                accumulator: SlotIdx::new(0),
                entry: StepIdx::new(2),
                join: StepIdx::new(join_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(join_idx),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let parts = WorkflowParts {
        name: Box::from("kani_g11_together"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_11_loop_body_graph(&parts);

    kani::assert(
        result.is_ok(),
        "Together body graph with valid join should be well-formed",
    );
}

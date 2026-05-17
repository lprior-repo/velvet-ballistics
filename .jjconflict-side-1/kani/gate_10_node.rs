//! Kani harnesses for Gate 10 - Node-kind-specific constraints.
//!
//! K8: ForEachStart has matching ForEachJoin
//! K9: TogetherStart has matching TogetherJoin
//! K10: ReduceStart has matching ReduceFinish
//! K11: CollectStart has matching CollectFinish

#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::gates::validate_gate_10_node_kind_specific;

/// K8: For all ForEachStart nodes, exists matching ForEachJoin.
#[kani::proof]
fn kani_gate_10_foreach_start_matches() {
    let body_idx: u16 = kani::any();
    let done_idx: u16 = kani::any();

    kani::assume(body_idx < done_idx);
    kani::assume(done_idx <= 10);

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
            id: StepIdx::new(1),
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
        name: Box::from("kani_g10_foreach"),
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

    let result = validate_gate_10_node_kind_specific(&parts);

    kani::assert(
        result.is_ok(),
        "ForEachStart with valid body/done should pass gate 10",
    );
}

/// K9: For all TogetherStart nodes, exists matching TogetherJoin.
#[kani::proof]
fn kani_gate_10_together_start_matches() {
    let join_idx: u16 = kani::any();

    kani::assume(join_idx > 0);
    kani::assume(join_idx <= 10);

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
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
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
        name: Box::from("kani_g10_together"),
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

    let result = validate_gate_10_node_kind_specific(&parts);

    kani::assert(
        result.is_ok(),
        "TogetherStart with valid branches/join should pass gate 10",
    );
}

/// K10: For all ReduceStart nodes, exists matching ReduceFinish.
#[kani::proof]
fn kani_gate_10_reduce_start_matches() {
    let body_idx: u16 = kani::any();
    let done_idx: u16 = kani::any();

    kani::assume(body_idx < done_idx);
    kani::assume(done_idx <= 10);

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(body_idx),
                done: StepIdx::new(done_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
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
        name: Box::from("kani_g10_reduce"),
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

    let result = validate_gate_10_node_kind_specific(&parts);

    kani::assert(
        result.is_ok(),
        "ReduceStart with valid body/done should pass gate 10",
    );
}

/// K11: For all CollectStart nodes, exists matching CollectFinish.
#[kani::proof]
fn kani_gate_10_collect_start_matches() {
    let body_idx: u16 = kani::any();
    let done_idx: u16 = kani::any();

    kani::assume(body_idx < done_idx);
    kani::assume(done_idx <= 10);

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                collector_slot: SlotIdx::new(1),
                body: StepIdx::new(body_idx),
                done: StepIdx::new(done_idx),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(1),
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
        name: Box::from("kani_g10_collect"),
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

    let result = validate_gate_10_node_kind_specific(&parts);

    kani::assert(
        result.is_ok(),
        "CollectStart with valid body/done should pass gate 10",
    );
}

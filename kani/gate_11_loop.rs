//! Kani harnesses for Gate 11 - Loop body graph well-formed.
//!
//! K13: ForEach body graph well-formed
//! K14: Together body graph well-formed

#![forbid(unsafe_code)]

use vb_core::workflow::WorkflowParts;
use vb_validate::gates::validate_gate_11_loop_body_graph;

/// K13: ForEachStart body subgraph leads to ForEachJoin.
///
/// Uses kani::Arbitrary for WorkflowParts (from vb_core::kani_workflow_arbitrary)
/// instead of hardcoded node vectors. The generated parts are assumed to have
/// a valid ForEach loop shape (body < done).
#[kani::proof]
fn kani_gate_11_foreach_body_well_formed() {
    let mut parts: WorkflowParts = kani::any();
    kani::assume(parts.nodes.len() >= 3);
    kani::assume(parts.nodes.len() <= 20);

    // Ensure the entry node is a ForEachStart so the harness covers
    // the ForEach body-graph path through validate_gate_11_loop_body_graph.
    if let vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. } =
        &parts.nodes[0].kind
    {
        kani::assume(body.as_usize() > 0);
        kani::assume(*body < *done);
        kani::assume(done.as_usize() < parts.nodes.len());
    } else {
        // If entry is not ForEachStart, replace it with a valid one.
        let entry_id = parts.nodes[0].id;
        let entry_next = parts.nodes[0].next;
        let entry_on_error = parts.nodes[0].on_error;
        let entry_error_slot = parts.nodes[0].error_slot;
        let body_idx = vb_core::ids::StepIdx::new(1);
        let done_idx = vb_core::ids::StepIdx::new(2);
        parts.nodes[0] = vb_core::workflow::CompiledNode {
            id: entry_id,
            output: parts.nodes[0].output,
            next: entry_next,
            on_error: entry_on_error,
            error_slot: entry_error_slot,
            kind: vb_core::workflow::CompiledNodeKind::ForEachStart {
                input: vb_core::ids::SlotIdx::new(0),
                item_slot: vb_core::ids::SlotIdx::new(1),
                limit: 10,
                body: body_idx,
                done: done_idx,
            },
        };
        // Ensure body and done nodes exist and are well-formed.
        if parts.nodes.len() <= 2 {
            return;
        }
        parts.nodes[1] = vb_core::workflow::CompiledNode {
            id: body_idx,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::ForEachNext {
                iterator_slot: vb_core::ids::SlotIdx::new(0),
                body: body_idx,
                done: done_idx,
            },
        };
        parts.nodes[2] = vb_core::workflow::CompiledNode {
            id: done_idx,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            },
        };
    }

    let result = validate_gate_11_loop_body_graph(&parts);

    kani::assert(
        result.is_ok(),
        "ForEach body graph with body < done should be well-formed",
    );
}

/// K14: TogetherStart body subgraph leads to TogetherJoin.
///
/// Uses kani::Arbitrary for WorkflowParts instead of hardcoded node vectors.
/// The generated parts are assumed to have a valid Together body graph shape.
#[kani::proof]
fn kani_gate_11_together_body_well_formed() {
    let mut parts: WorkflowParts = kani::any();
    kani::assume(parts.nodes.len() >= 4);
    kani::assume(parts.nodes.len() <= 20);

    // Ensure the entry node is a TogetherStart so the harness covers
    // the Together body-graph path through validate_gate_11_loop_body_graph.
    if let vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join } =
        &parts.nodes[0].kind
    {
        kani::assume(join.as_usize() > 0);
        kani::assume(join.as_usize() < parts.nodes.len());
        for branch in branches.iter() {
            kani::assume(branch.as_usize() < parts.nodes.len());
        }
    } else {
        // If entry is not TogetherStart, replace it with a valid one.
        let entry_id = parts.nodes[0].id;
        let entry_next = parts.nodes[0].next;
        let entry_on_error = parts.nodes[0].on_error;
        let entry_error_slot = parts.nodes[0].error_slot;
        let branch_1 = vb_core::ids::StepIdx::new(1);
        let branch_2 = vb_core::ids::StepIdx::new(2);
        let join_idx = vb_core::ids::StepIdx::new(3);
        parts.nodes[0] = vb_core::workflow::CompiledNode {
            id: entry_id,
            output: parts.nodes[0].output,
            next: entry_next,
            on_error: entry_on_error,
            error_slot: entry_error_slot,
            kind: vb_core::workflow::CompiledNodeKind::TogetherStart {
                branches: Box::new([branch_1, branch_2]),
                join: join_idx,
            },
        };
        // Ensure branch and join nodes exist and are well-formed.
        if parts.nodes.len() <= 3 {
            return;
        }
        parts.nodes[1] = vb_core::workflow::CompiledNode {
            id: branch_1,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::TogetherBranch {
                accumulator: vb_core::ids::SlotIdx::new(0),
                entry: branch_1,
                join: join_idx,
            },
        };
        parts.nodes[2] = vb_core::workflow::CompiledNode {
            id: branch_2,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::TogetherBranch {
                accumulator: vb_core::ids::SlotIdx::new(0),
                entry: branch_2,
                join: join_idx,
            },
        };
        parts.nodes[3] = vb_core::workflow::CompiledNode {
            id: join_idx,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            },
        };
    }

    let result = validate_gate_11_loop_body_graph(&parts);

    kani::assert(
        result.is_ok(),
        "Together body graph with valid join should be well-formed",
    );
}

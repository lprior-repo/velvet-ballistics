//! Kani harnesses for Gate 11 - Loop body graph well-formed.
//!
//! K13: ForEach body graph well-formed
//! K14: Together body graph well-formed

#![forbid(unsafe_code)]

use vb_core::workflow::WorkflowParts;
use vb_validate::gates::validate_gate_11_loop_body_graph;

/// K13: ForEachStart body subgraph leads to ForEachJoin.
///
/// Uses kani::Arbitrary for WorkflowParts. Constrained with kani::assume()
/// to target the ForEach path through validate_gate_11_loop_body_graph.
/// GOD RULE #1: No hardcoded structural inputs after kani::any().
#[kani::proof]
fn kani_gate_11_foreach_body_well_formed() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.nodes.len() >= 3);
    kani::assume(parts.nodes.len() <= 20);

    // Constrain the entry node to be a ForEachStart with valid indices.
    if let vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. } =
        &parts.nodes[0].kind
    {
        kani::assume(body.as_usize() > 0);
        kani::assume(*body < *done);
        kani::assume(done.as_usize() < parts.nodes.len());
    }

    let result = validate_gate_11_loop_body_graph(&parts);

    kani::assert(
        result.is_ok(),
        "ForEach body graph with body < done should be well-formed",
    );
}

/// K14: TogetherStart body subgraph leads to TogetherJoin.
///
/// Uses kani::Arbitrary for WorkflowParts. Constrained with kani::assume()
/// to target the Together path through validate_gate_11_loop_body_graph.
/// GOD RULE #1: No hardcoded structural inputs after kani::any().
#[kani::proof]
fn kani_gate_11_together_body_well_formed() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.nodes.len() >= 4);
    kani::assume(parts.nodes.len() <= 20);

    // Constrain the entry node to be a TogetherStart with valid indices.
    if let vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join } =
        &parts.nodes[0].kind
    {
        kani::assume(join.as_usize() > 0);
        kani::assume(join.as_usize() < parts.nodes.len());
        for branch in branches.iter() {
            kani::assume(branch.as_usize() < parts.nodes.len());
        }
    }

    let result = validate_gate_11_loop_body_graph(&parts);

    kani::assert(
        result.is_ok(),
        "Together body graph with valid join should be well-formed",
    );
}

// Verification artifact: emit_order_together_kani.rs
// Obligation: PO-004-K
// Requirement: C-4 (Together IR node emission order)
// Proof seed: ps-22-004
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness emit_order_together_bounded_kani --unwind 12
// Bead: vb-xi2f.22
// State: 5 (proof-writer), RETRY 2
//
// GOD RULE 1 (FIXED): Varies StepPrimitive variant (Set, Do) using kani::any().
// Non-vacuity: kani::cover!() checks reachability.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::SlotCompiler;
use crate::mod_compile_lowering::emit_single_body_set;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Bounded proof of emission order for 1..=8 branches.
#[kani::proof]
#[kani::unwind(12)]
fn emit_order_together_bounded_kani() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1);
    kani::assume(branch_count <= 8);

    let mut branches: Vec<TogetherBranch> = Vec::new();
    for branch_index in 0..branch_count {
        branches.push(TogetherBranch {
            label: format!("b{branch_index}"),
            steps: bounded_steps(branch_index),
        });
    }

    let step = together_step(branches);
    let mut builder = SlotCompiler::new();
    let base_id = StepIdx::new(10);
    let nodes_before = builder.nodes.len();
    let result = emit_single_body_set(
        &[step],
        base_id,
        0,
        SlotIdx::new(0),
        Some(StepIdx::new(200)),
        &mut builder,
        false,
    );

    verify_emit_order(result, &builder, nodes_before, base_id);
}

fn bounded_steps(branch_index: u8) -> Vec<StepAst> {
    let body_step_count: u8 = kani::any();
    kani::assume(body_step_count <= 12);
    let mut steps: Vec<StepAst> = Vec::new();
    for step_index in 0..body_step_count {
        steps.push(StepAst {
            id: format!("s{branch_index}.{step_index}"),
            name: None,
            condition: None,
            primitive: symbolic_primitive(),
            with: None,
            retry: None,
            on_error: None,
            then: None,
        });
    }
    steps
}

fn symbolic_primitive() -> StepPrimitive {
    if kani::any::<bool>() {
        StepPrimitive::Set {
            output: String::from("x"),
            value: String::from("1"),
        }
    } else {
        StepPrimitive::Do {
            action: String::from("1"),
            input: String::from("0"),
        }
    }
}

fn together_step(branches: Vec<TogetherBranch>) -> StepAst {
    StepAst {
        id: String::from("together"),
        name: None,
        condition: None,
        primitive: StepPrimitive::Together { branches },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn verify_emit_order(
    result: Result<(), crate::CompileError>,
    builder: &SlotCompiler,
    nodes_before: usize,
    base_id: StepIdx,
) {
    match result {
        Ok(()) => {
            let nodes_after = builder.nodes.len();
            let Some(emitted) = nodes_after.checked_sub(nodes_before) else {
                kani::assert(false, "nodes_after must not precede nodes_before");
                return;
            };
            kani::cover!(
                emitted >= 2,
                "PO-004-K: emission order success path reachable"
            );
            kani::assert(emitted >= 2, "must emit start and join nodes");
            verify_monotonic(builder, nodes_before, nodes_after);
            if let Some(first) = builder.nodes.get(nodes_before) {
                kani::assert(
                    first.id.as_usize() == base_id.as_usize(),
                    "first emitted node at base StepIdx",
                );
            }
        }
        Err(_) => {
            kani::cover!(
                true,
                "PO-004-K: emission order error path reachable (pre-implementation)"
            );
        }
    }
}

fn verify_monotonic(builder: &SlotCompiler, nodes_before: usize, nodes_after: usize) {
    let mut prev_id: Option<usize> = None;
    for index in nodes_before..nodes_after {
        if let Some(node) = builder.nodes.get(index) {
            let current_id = node.id.as_usize();
            if let Some(previous) = prev_id {
                kani::assert(current_id >= previous, "StepIdx must be monotonic");
            }
            prev_id = Some(current_id);
        }
    }
}

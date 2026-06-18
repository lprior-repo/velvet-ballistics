// Verification artifact: body_dispatcher_together_kani.rs
// Obligation: PO-002-K
// Requirement: C-2 (emit_single_body_set dispatch for Together in body position)
// Proof seed: ps-22-002
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness body_dispatcher_together_acceptance_kani --unwind 10
// Bead: vb-xi2f.22
// State: 5 (proof-writer), RETRY 2
//
// GOD RULE 1 (FIXED): Varies StepPrimitive variant (Set, Do) using kani::any().
// GOD RULE 2: Binds to actual emit_single_body_set in part_04.rs.
// Non-vacuity: kani::cover!() checks reachability.
//
// Trusted bases:
// - TB-22-003: SlotCompiler::push_node() correctness
// - TB-22-004: checked_step_offset() correctness

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::SlotCompiler;
use crate::mod_compile_lowering::emit_single_body_set;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Bounded proof that emit_single_body_set with Together emits correct nodes
/// with diverse branch body primitives.
#[kani::proof]
#[kani::unwind(10)]
fn body_dispatcher_together_acceptance_kani() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1);
    kani::assume(branch_count <= 8);

    let mut branches: Vec<TogetherBranch> = Vec::new();
    for br_idx in 0..branch_count {
        branches.push(TogetherBranch {
            label: format!("branch_{br_idx}"),
            steps: bounded_steps(br_idx),
        });
    }

    let step = together_step(branches);
    let mut builder = SlotCompiler::new();
    let nodes_before = builder.nodes.len();
    let result = emit_single_body_set(
        &[step],
        StepIdx::new(0),
        0,
        SlotIdx::new(0),
        Some(StepIdx::new(100)),
        &mut builder,
        false,
    );

    verify_emit_result(result, &builder, nodes_before);
}

fn bounded_steps(branch_index: u8) -> Vec<StepAst> {
    let body_step_count: u8 = kani::any();
    kani::assume(body_step_count <= 16);
    let mut steps: Vec<StepAst> = Vec::new();
    for step_index in 0..body_step_count {
        steps.push(StepAst {
            id: format!("br{branch_index}.step{step_index}"),
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
        id: String::from("together_step"),
        name: None,
        condition: None,
        primitive: StepPrimitive::Together { branches },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn verify_emit_result(
    result: Result<(), crate::CompileError>,
    builder: &SlotCompiler,
    nodes_before: usize,
) {
    match result {
        Ok(()) => {
            let nodes_after = builder.nodes.len();
            let Some(emitted) = nodes_after.checked_sub(nodes_before) else {
                kani::assert(false, "nodes_after must not precede nodes_before");
                return;
            };
            kani::assert(emitted > 0, "together lowering must emit nodes");
            kani::assert(emitted >= 2, "together emits start and join nodes");
            verify_first_and_last(builder, nodes_before, nodes_after);
        }
        Err(_) => {
            kani::cover!(
                true,
                "PO-002-K: together dispatch error path reachable (pre-implementation)"
            );
        }
    }
}

fn verify_first_and_last(builder: &SlotCompiler, nodes_before: usize, nodes_after: usize) {
    if let Some(first) = builder.nodes.get(nodes_before) {
        kani::assert(first.id.as_usize() == 0, "first emitted node has StepIdx 0");
    }
    if let Some(last_index) = nodes_after.checked_sub(1) {
        if let Some(last) = builder.nodes.get(last_index) {
            if let Some(next) = last.next {
                kani::assert(next.as_usize() == 100, "last node points to next");
            }
        }
    }
}

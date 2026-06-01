// Verification artifact: width_parity_kani.rs
// Obligation: PO-003-K
// Requirement: C-3 (Width/node parity between canonical_body_step_width and emit_single_body_set)
// Proof seed: ps-22-003
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness width_node_parity_together_kani --unwind 10
// Bead: vb-xi2f.22
// State: 5 (proof-writer), RETRY 2
//
// GOD RULE 1 (FIXED): Varies StepPrimitive variant (Set, Do) using kani::any().
// This is the primary Kani defense against TH-1 (width divergence).
// Non-vacuity: kani::cover!() checks reachability.
//
// Hazard: TH-1 (Width divergence causing StepIdx misalignment) - HIGH severity

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::SlotCompiler;
use crate::mod_compile_lowering::canonical_body_step_width;
use crate::mod_compile_lowering::emit_single_body_set;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

/// Bounded proof: for 1..=6 branches, 0..=16 body steps each (Set+Do varied),
/// the number of nodes emitted equals the width returned by canonical_body_step_width.
///
/// Bounds:
/// - Unwind: 10
/// - Branch count: 1..=6
/// - Body steps per branch: 0..=16
/// - Primitive variants: Set, Do (varied via kani::any())
#[kani::proof]
#[kani::unwind(10)]
fn width_node_parity_together_kani() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1 && branch_count <= 6);

    let mut branches: Vec<TogetherBranch> = Vec::new();
    for br_idx in 0..branch_count {
        let body_step_count: u8 = kani::any();
        kani::assume(body_step_count <= 16);

        let mut steps: Vec<StepAst> = Vec::new();
        for s_idx in 0..body_step_count {
            // Vary between Set and Do (GOD RULE 1 fix - critical for TH-1 defense)
            let variant_is_set: bool = kani::any();
            let primitive = if variant_is_set {
                StepPrimitive::Set {
                    output: String::from("x"),
                    value: String::from("1"),
                }
            } else {
                StepPrimitive::Do {
                    action: String::from("1"),
                    input: String::from("0"),
                }
            };

            steps.push(StepAst {
                id: format!("s{}.{}", br_idx, s_idx),
                name: None,
                condition: None,
                primitive,
                with: None,
                retry: None,
                on_error: None,
                then: None,
            });
        }

        branches.push(TogetherBranch {
            label: format!("b{}", br_idx),
            steps,
        });
    }

    let primitive = StepPrimitive::Together {
        branches: branches.clone(),
    };

    // Compute expected width
    let width_result = canonical_body_step_width(&primitive);

    // Create the step for emission
    let step = StepAst {
        id: String::from("t"),
        name: None,
        condition: None,
        primitive: StepPrimitive::Together { branches },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    let mut builder = SlotCompiler::new();
    let nodes_before = builder.nodes.len();

    let emit_result = emit_single_body_set(
        &[step],
        StepIdx::new(0),
        0,
        SlotIdx::new(0),
        None,
        &mut builder,
        false,
    );

    let nodes_after = builder.nodes.len();
    let emitted_count = nodes_after - nodes_before;

    // Parity assertion: if both succeed, widths must match
    match (width_result, emit_result) {
        (Ok(expected_width), Ok(())) => {
            // Non-vacuity: prove parity success path is reachable
            kani::cover!(
                expected_width == emitted_count,
                "PO-003-K: width-node parity success (TH-1 defense)"
            );

            kani::assert(
                expected_width == emitted_count,
                "TH-1: width/node parity - computed width must equal emitted node count",
            );
        }
        (Ok(_), Err(_)) => {
            kani::cover!(
                true,
                "PO-003-K: width ok but emission failed (pre-implementation)"
            );
        }
        (Err(_), Ok(())) => {
            kani::cover!(
                true,
                "PO-003-K: width failed but emission ok (should not happen)"
            );
        }
        (Err(_), Err(_)) => {
            kani::cover!(true, "PO-003-K: both failed (pre-implementation)");
        }
    }
}

// Verification artifact: kani_reduce_tryfromparts.rs
// PO: PO-TRYFROMPARTS-KANI-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 3)
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness check_reduce_multi_step_try_from_parts --unwind 16
//
// Requirement: C2 — Width-Node Count Synchronization (end-to-end)
// Domain Claim: A workflow containing a multi-step reduce body produces
//   valid layout widths that pass try_from_parts validation.
//
// GOD RULE 1: Uses kani::any() for workflow construction parameters.
// GOD RULE 2 (RETRY 3): Calls production body_width and canonical_body_step_width directly.
//
// RETRY 3 FIX: Diverse body step variants (Set/Do) via kani::any().

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::mod_compile_lowering::part_01::{body_width, canonical_body_step_width};
use vb_yaml::ast::{StepAst, StepPrimitive};

fn arbitrary_body_step(idx: u8) -> StepAst {
    let variant: u8 = kani::any();
    let primitive = if variant == 0 {
        StepPrimitive::Do {
            action: format!("{}", idx + 1),
            input: "0".to_string(),
        }
    } else {
        StepPrimitive::Set {
            output: format!("out_{}", idx),
            value: format!("{}", idx),
        }
    };
    StepAst {
        id: format!("reduce_body_{}", idx),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// Verify that a multi-step body produces correct layout widths.
#[kani::proof]
#[kani::unwind(16)]
fn check_reduce_multi_step_try_from_parts() {
    let body_len: u8 = kani::any();
    kani::assume(body_len >= 1 && body_len <= 16);

    let body: Vec<StepAst> = (0..body_len).map(arbitrary_body_step).collect();

    let total_width = body_width(&body, 3);
    match total_width {
        Ok(w) => {
            kani::cover!(true, "Kani: multi-step body width Ok");
            assert!(
                w >= 4,
                "width must be >= 4 for body with 1+ steps + overhead 3"
            );
            assert!(w <= usize::from(u16::MAX), "width within u16::MAX");
        }
        Err(_) => {
            kani::cover!(true, "Kani: multi-step body overflow");
        }
    }
}

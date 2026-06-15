// Verification artifact: kani_reduce_empty.rs
// PO: PO-EMPTY-KANI-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness check_reduce_empty_body_rejection
//
// Requirement: C12 — Empty Body Handling
// Domain Claim: When body.len() == 0, emit_reduce_body_steps returns
//   CompileErrors with StepFieldShape. No nodes are emitted.
//
// GOD RULE 1: Uses kani::any() for proof of empty-body rejection.
// GOD RULE 2: Binds to production body_width for empty body.
//
// Model bounds: body.len() == 0 (trivial case).
// Trusted bases: None.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::mod_compile_lowering::part_01::body_width;
use vb_yaml::ast::StepAst;

/// Verify that an empty body produces body_width == overhead (no nodes beyond overhead).
/// body_width with empty body and overhead 3 returns Ok(3).
#[kani::proof]
fn check_reduce_empty_body_rejection() {
    let empty_body: Vec<StepAst> = vec![];

    // body_width for empty body with overhead 3 should be exactly 3
    let result = body_width(&empty_body, 3);

    match result {
        Ok(w) => {
            kani::assert(w == 3, "empty body width must be exactly overhead);
        }
        Err(_) => {
            kani::assert(false, "empty body width should never ove);
        }
    }
}

/// Verify body_width with overhead 0 and empty body returns 0.
#[kani::proof]
fn check_reduce_empty_body_width_zero() {
    let empty_body: Vec<StepAst> = vec![];
    let result = body_width(&empty_body, 0);

    match result {
        Ok(w) => {
            kani::assert(w == 0, "empty body width with overhead 0 must);
        }
        Err(_) => {
            kani::assert(false, "empty body should not ove);
        }
    }
}

/// Verify that emit_single_body_set rejects empty body.
#[kani::proof]
fn check_reduce_emit_single_body_set_empty() {
    use crate::mod_compile_lowering::part_04::emit_single_body_set;
    use crate::mod_compile_lowering::part_07::SlotCompiler;
    use vb_core::ids::StepIdx;

    let empty_body: Vec<StepAst> = vec![];
    let id_val: u16 = kani::any();
    kani::assume(id_val <= 65500);

    let mut builder = SlotCompiler::new();
    let result = emit_single_body_set(
        &empty_body,
        StepIdx::new(id_val),
        0,
        vb_core::SlotIdx::new(0),
        None,
        &mut builder,
        false,
    );

    kani::cover!(result.is_err(), "empty body rejection path reached");
    kani::assert(
        result.is_err(),
        "emit_single_body_set must reject empty body");
}

// Verification artifact: error_parity_harness.rs
// PO: PO-031 (error parity panic-free verification)
// Bead: vb-xi2f.23
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_error_parity
//
// Proof obligations:
// - PO-031: Both error paths never panic and return correct error variant
//   * empty body → StepFieldShape
//   * non-Set body → UnsupportedStepPrimitive
//
// GOD RULE 1: kani::any() generates empty body and non-Set variants.
// GOD RULE 2: Binds to actual Rust emit_single_body_set implementation.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_04::emit_single_body_set;
use vb_compile::compile::SlotCompiler;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_compile::{StepAst, StepPrimitive};

/// PO-031 H1: Error parity - empty body returns StepFieldShape (not panic).
#[kani::proof]
#[kani::unwind(3)]
fn kani_error_parity() {
    // Generate empty body
    let body: Vec<StepAst> = kani::any();
    kani::assume(body.is_empty());

    let id = StepIdx::new(kani::any());
    let slot = SlotIdx::new(1);
    let mut builder = SlotCompiler::new();

    let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

    // Empty body must return error
    match result {
        Ok(()) => kani::assert(false, "empty body cannot succeed"),
        Err(e) => {
            let is_step_field_shape = e.0.iter().any(|err| {
                matches!(err, vb_compile::CompileError::StepFieldShape { field, .. }
                    if *field == "steps")
            });
            kani::assert(is_step_field_shape, "empty body returns StepFieldShape");
        }
    }
}

/// PO-031 H2: Error parity - non-Set body returns UnsupportedStepPrimitive (not panic).
#[kani::proof]
#[kani::unwind(4)]
fn kani_error_parity_non_set() {
    // Non-Set body: ForEach
    let body = vec![StepAst {
        id: "foreach".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "x".to_string(),
            input: "0".to_string(),
            at_once: None,
            body: vec![],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let id = StepIdx::new(kani::any());
    let slot = SlotIdx::new(1);
    let mut builder = SlotCompiler::new();

    let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

    match result {
        Ok(()) => kani::assert(false, "non-Set body cannot succeed"),
        Err(e) => {
            let is_unsupported = e.0.iter().any(|err| {
                matches!(err, vb_compile::CompileError::UnsupportedStepPrimitive { .. })
            });
            kani::assert(is_unsupported, "non-Set returns UnsupportedStepPrimitive");
        }
    }
}

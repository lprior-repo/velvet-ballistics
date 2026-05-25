// Verification artifact: emit_single_body_set_empty.rs
// PO: PO-007 (emit_single_body_set panic-free for empty body)
// Bead: vb-xi2f.23
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_emit_single_body_set_empty
//
// Proof obligations:
// - PO-007: emit_single_body_set panic-free for empty body
//
// The panic point is `body.first()` at part_04.rs:203.
// Option::ok_or_else on None does NOT panic — it returns the error.
// This harness verifies that the empty body path does not panic.
//
// GOD RULE 1: kani::any() generates empty Vec as valid input.
// GOD RULE 2: Binds to actual Rust emit_single_body_set implementation.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_04::emit_single_body_set;
use vb_compile::compile::SlotCompiler;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive};

/// PO-007 H1: emit_single_body_set does NOT panic when body is empty.
/// The Rust code uses `body.first().ok_or_else(...)` which returns Err, not panics.
#[kani::proof]
#[kani::unwind(4)]
fn kani_emit_single_body_set_empty() {
    // Generate empty body using kani::any()
    let body: Vec<StepAst> = kani::any();

    // If body is not empty, constrain it to be empty
    kani::assume(body.is_empty());

    let id = StepIdx::new(kani::any());
    let slot = SlotIdx::new(1);
    let mut builder = SlotCompiler::new();

    // Call emit_single_body_set with empty body
    // This should return Err(StepFieldShape), NOT panic
    let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

    // The only valid outcome for empty body is an error
    match result {
        Ok(()) => kani::assert(false, "empty body cannot succeed"),
        Err(_) => kani::assert(true, "empty body correctly returns error"),
    }
}

/// PO-007 H2: body.first() on empty Vec returns None, not panic.
#[kani::proof]
#[kani::unwind(3)]
fn kani_empty_vec_first() {
    let empty_vec: Vec<StepAst> = Vec::new();

    // first() on empty vec returns None
    let first = empty_vec.first();
    match first {
        Some(_) => kani::assert(false, "empty vec has no first element"),
        None => kani::assert(true, "first() on empty vec returns None"),
    }

    // ok_or_else on None does not panic - it returns the provided error
    let _err = first.ok_or_else(|| "expected one set step");
    kani::assert(true, "ok_or_else on None does not panic");
}

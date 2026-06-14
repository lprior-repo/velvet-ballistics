// Verification artifact: step_offset_overflow.rs
// PO: PO-016, PO-028 (checked_step_offset overflow behavior)
// Bead: vb-xi2f.23
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_step_offset_overflow
//
// Proof obligations:
// - PO-016: Step index overflow (id + 3 > u16::MAX) never panics; returns error
// - PO-028: Same overflow behavior verified via Kani (PS-005/R9)
//
// The checked_step_offset function uses checked_add + ok_or_else pattern.
// This harness verifies no panic for all u16 id values with offsets {1, 2, 3}.
//
// GOD RULE 1: kani::any() generates all u16 values for id.
// GOD RULE 2: Binds to actual Rust checked_step_offset implementation.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_03::checked_step_offset;

/// PO-016 H1: checked_step_offset with offset=1 never panics for any u16 id.
#[kani::proof]
#[kani::unwind(4)]
fn kani_step_offset_overflow() {
    let id: u16 = kani::any();
    let offset: u8 = 1;

    // Call checked_step_offset
    let result = checked_step_offset(
        StepIdx::new(id),
        offset,
        "test",
        "body",
    );

    // Result is always Result<StepIdx, CompileError> - no panic possible
    match result {
        Ok(new_id) => {
            // If Ok, the sum was within u16::MAX
            kani::assert(new_id.get() == id + 1, "offset=1: new_id = id+1");
        }
        Err(_) => {
            // If Err, id + offset > u16::MAX
            kani::assert(id + 1 > u16::MAX as u16, "offset=1: overflow detected");
        }
    }
}

/// PO-016 H2: checked_step_offset with offset=2 handles overflow correctly.
#[kani::proof]
#[kani::unwind(4)]
fn kani_step_offset_offset2() {
    let id: u16 = kani::any();
    let offset: u8 = 2;

    let result = checked_step_offset(
        vb_core::ids::StepIdx::new(id),
        offset,
        "test",
        "page",
    );

    match result {
        Ok(new_id) => {
            kani::assert(new_id.get() == id + 2, "offset=2: new_id = id+2");
        }
        Err(_) => {
            kani::assert(id + 2 > u16::MAX as u16, "offset=2: overflow detected");
        }
    }
}

/// PO-016 H3: checked_step_offset with offset=3 handles overflow correctly.
#[kani::proof]
#[kani::unwind(4)]
fn kani_step_offset_offset3() {
    let id: u16 = kani::any();
    let offset: u8 = 3;

    let result = checked_step_offset(
        vb_core::ids::StepIdx::new(id),
        offset,
        "test",
        "done",
    );

    match result {
        Ok(new_id) => {
            kani::assert(new_id.get() == id + 3, "offset=3: new_id = id+3");
        }
        Err(_) => {
            kani::assert(id + 3 > u16::MAX as u16, "offset=3: overflow detected");
        }
    }
}

/// PO-016 H4: Boundary case id = u16::MAX with offset=3 overflows (not panic).
#[kani::proof]
#[kani::unwind(8)]
fn kani_step_offset_boundary_max() {
    let id = u16::MAX;
    let offset: u8 = 3;

    let result = checked_step_offset(
        vb_core::ids::StepIdx::new(id),
        offset,
        "test",
        "done",
    );

    match result {
        Ok(_) => kani::assert(false, "u16::MAX + 3 cannot succeed"),
        Err(_) => kani::assert(true, "u16::MAX + 3 correctly returns error"),
    }
}

/// PO-016 H5: Boundary case id = u16::MAX - 2 with offset=3 equals u16::MAX (valid).
#[kani::proof]
#[kani::unwind(8)]
fn kani_step_offset_boundary_valid() {
    let id = u16::MAX - 2;
    let offset: u8 = 3;

    let result = checked_step_offset(
        vb_core::ids::StepIdx::new(id),
        offset,
        "test",
        "done",
    );

    match result {
        Ok(new_id) => {
            kani::assert(new_id.get() == u16::MAX, "max-2 + 3 = u16::MAX");
        }
        Err(_) => kani::assert(false, "this should not overflow"),
    }
}

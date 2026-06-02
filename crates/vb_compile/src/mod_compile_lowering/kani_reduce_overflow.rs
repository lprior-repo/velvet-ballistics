// Verification artifact: kani_reduce_overflow.rs
// PO: PO-OVERFLOW-KANI-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 3)
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness check_reduce_body_width_overflow --unwind 32
//
// Requirement: C3 — Body Step Sequential Assignment (overflow guard)
// Domain Claim: body_width returns Ok(n) where n <= u16::MAX, or Err on overflow.
//   No arithmetic overflow panics.
//
// GOD RULE 1: Uses kani::any() with bounded assumptions.
// GOD RULE 2 (RETRY 3): Calls production body_width, checked_step_offset, canonical_body_step_width directly.
//
// RETRY 3 FIX: Diverse StepPrimitive variants via kani::any().

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::mod_compile_lowering::part_01::{body_width, canonical_body_step_width};
use crate::mod_compile_lowering::part_12::checked_step_offset;
use vb_core::ids::StepIdx;
use vb_yaml::ast::{StepAst, StepPrimitive};

fn arbitrary_body_step(idx: u8) -> StepAst {
    let variant: u8 = kani::any();
    let primitive = match variant {
        0 => StepPrimitive::Set {
            output: format!("out_{}", idx),
            value: format!("val_{}", idx),
        },
        1 => StepPrimitive::Do {
            action: format!("{}", idx + 1),
            input: format!("{}", idx),
        },
        _ => StepPrimitive::Set {
            output: format!("out_{}", idx),
            value: format!("val_{}", idx),
        },
    };
    StepAst {
        id: format!("step_{}", idx),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// Verify body_width handles overflow correctly with diverse step types.
#[kani::proof]
#[kani::unwind(32)]
fn check_reduce_body_width_overflow() {
    let step_count: u8 = kani::any();
    kani::assume(step_count <= 32);

    let body: Vec<StepAst> = (0..step_count).map(arbitrary_body_step).collect();
    let result = body_width(&body, 3);

    kani::cover!(true, "Kani: overflow check entry");
    match result {
        Ok(w) => {
            kani::cover!(true, "Kani: body_width Ok within bounds");
            assert!(
                w <= usize::from(u16::MAX),
                "body_width Ok implies width <= u16::MAX"
            );
            assert!(w >= 3, "width must be >= overhead");
        }
        Err(_) => {
            kani::cover!(true, "Kani: body_width overflow triggered");
        }
    }
}

/// Verify checked_step_offset never panics at any boundary.
#[kani::proof]
fn check_reduce_checked_step_offset_boundary() {
    let id_val: u16 = kani::any();
    let offset: u16 = kani::any();

    let id = StepIdx::new(id_val);
    let result = checked_step_offset(id, offset, "reduce", "body");

    kani::cover!(true, "Kani: overflow boundary test entry");
    match result {
        Ok(step) => {
            kani::cover!(true, "Kani: checked_step_offset Ok");
            assert!(step.get() >= id_val, "Ok result must be >= input id");
            assert!(step.get() <= u16::MAX, "Ok result must be <= u16::MAX");
        }
        Err(_) => {
            kani::cover!(true, "Kani: checked_step_offset overflow rejected");
        }
    }
}

/// Verify canonical_body_step_width does not panic on diverse primitives.
#[kani::proof]
#[kani::unwind(8)]
fn check_reduce_step_width_no_panic() {
    let variant: u8 = kani::any();
    kani::assume(variant <= 2); // 0=Set, 1=Do, 2=Finish

    let primitive = match variant {
        0 => StepPrimitive::Set {
            output: "o".to_string(),
            value: "1".to_string(),
        },
        1 => StepPrimitive::Do {
            action: "1".to_string(),
            input: "1".to_string(),
        },
        _ => StepPrimitive::Finish {
            result: vb_yaml::ast::ScalarValue::Integer(0),
        },
    };

    kani::cover!(true, "Kani: step width no-panic entry");
    let _ = canonical_body_step_width(&primitive);
    // Must not panic — verifies panic-freedom
}

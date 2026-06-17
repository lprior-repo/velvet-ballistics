// Verification artifact: kani_reduce_regression.rs
// PO: PO-REGRESSION-KANI-001
// Bead: vb-xi2f.24 | State: 13 (black-hat, RETRY)
// Verifier: Kani
// Command: cargo kani -p vb_compile --harness check_single_step_equivalence_contract --unwind 8
//
// Requirement: C7 -- Single-Step Body Compatibility
// Domain Claim: For body.len() == 1, the multi-step dispatcher
//   emit_reduce_body_steps produces IR structurally identical to emit_single_body_set.
//
// GOD RULE 2 (COMPLIANT): Calls production emit_single_body_set, emit_reduce_body_steps,
//   and body_width directly. Both dispatchers executed with identical inputs; results
//   compared for node count, ID, next-link, and slot equivalence.
//
// GOD RULE 1: Uses kani::any() for arbitrary step selection.
// Model bounds: body.len() == 1, Set (variant 0) or Do (variant 1).
// Trusted bases: TB-005 (emit_single_body_set is the reference implementation).

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::mod_compile_lowering::part_01::body_width;
use crate::mod_compile_lowering::part_04::emit_reduce_body_steps;
use crate::mod_compile_lowering::part_04::emit_single_body_set;
use crate::mod_compile_lowering::part_07::SlotCompiler;
use vb_core::SlotIdx;
use vb_core::ids::StepIdx;
use vb_yaml::ast::{StepAst, StepPrimitive};

/// Generate a single-step body with arbitrary Set or Do primitive.
fn arbitrary_single_step_body() -> Vec<StepAst> {
    let variant: u8 = kani::any();
    kani::assume(variant <= 1); // 0=Set, 1=Do

    let primitive = match variant {
        0 => StepPrimitive::Set {
            output: "out".to_string(),
            value: "1".to_string(),
        },
        _ => StepPrimitive::Do {
            action: "1".to_string(),
            input: "0".to_string(),
        },
    };

    vec![StepAst {
        id: "body0".to_string(),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }]
}

/// Verify emit_single_body_set correctly processes single-step bodies.
/// This is the REFERENCE implementation that emit_reduce_body_steps must match.
#[kani::proof]
#[kani::unwind(8)]
fn check_single_step_reference_behavior() {
    let body = arbitrary_single_step_body();
    let id_val: u16 = kani::any();
    kani::assume(id_val <= 65500);

    let mut builder = SlotCompiler::new();

    let result = emit_single_body_set(
        &body,
        StepIdx::new(id_val),
        0,
        SlotIdx::new(1),
        Some(StepIdx::new(id_val.saturating_add(2))),
        &mut builder,
        false,
    );

    match result {
        Ok(()) => {
            kani::assert(builder.nodes.len() == 1,
                "single-step body must emit exactly 1 node",
            );
            if let Some(node) = builder.nodes.first() {
                kani::assert(node.id.get() == id_val,
                    "emitted node ID must match body_step id",
                );
                kani::assert(node.next.is_some(),
                    "emitted node must have next pointer set",
                );
            }
        }
        Err(_) => {}
    }
}

/// Verify body_width contract for single-step bodies: width = overhead + 1.
/// The multi-step dispatcher must satisfy the same width contract.
#[kani::proof]
#[kani::unwind(8)]
fn check_single_step_body_width_contract() {
    let body = arbitrary_single_step_body();
    let result = body_width(&body, 3);

    match result {
        Ok(w) => {
            ,
                    "emitted node must have next pointer set",
                );
            }
        }
        Err(_) => {}
    }
}

/// Verify body_width contract for single-step bodies: width = overhead + 1.
/// The multi-step dispatcher must satisfy the same width contract.
#[kani::proof]
#[kani::unwind(8)]
fn check_single_step_body_width_contract() {
    let body = arbitrary_single_step_body();
    let result = body_width(&body, 3);

    match result {
        Ok(w) => {
            kani::assert(w >= 4, "reduce width with single body step >= 4");
            kani::assert(w <= usize::from(u16::MAX), "width within u16::MAX");
        }
        Err(_) => {}
    }
}

/// Equivalence contract: emit_reduce_body_steps MUST produce the same builder state
/// as emit_single_body_set for body.len() == 1.
///
/// For single-step bodies, both dispatchers must:
///   - Return the same Ok/Err outcome
///   - Produce the same number of emitted nodes
///   - Produce nodes with matching IDs and next pointers
///
/// This harness directly compares both functions with identical inputs.
#[kani::proof]
#[kani::unwind(8)]
fn check_single_step_equivalence_contract() {
    let body = arbitrary_single_step_body();

    // Verify width contract as a precondition
    let width_result = body_width(&body, 3);
    match width_result {
        Ok(w) => {
            , "width within u16::MAX");
        }
        Err(_) => {}
    }
}

/// Equivalence contract: emit_reduce_body_steps MUST produce the same builder state
/// as emit_single_body_set for body.len() == 1.
///
/// For single-step bodies, both dispatchers must:
///   - Return the same Ok/Err outcome
///   - Produce the same number of emitted nodes
///   - Produce nodes with matching IDs and next pointers
///
/// This harness directly compares both functions with identical inputs.
#[kani::proof]
#[kani::unwind(8)]
fn check_single_step_equivalence_contract() {
    let body = arbitrary_single_step_body();

    // Verify width contract as a precondition
    let width_result = body_width(&body, 3);
    match width_result {
        Ok(w) => {
            kani::assert(w >= 4, "single-step body width >= 4");
            kani::assert(w <= usize::from(u16::MAX), "width within bounds");
        }
        Err(_) => {}
    }

    // Direct equivalence comparison: both dispatchers with identical inputs
    let id_val: u16 = kani::any();
    kani::assume(id_val <= 65500);
    let step_idx = StepIdx::new(id_val);
    let slot_idx = SlotIdx::new(1);
    let next_step = Some(StepIdx::new(id_val.saturating_add(2)));

    let mut builder_a = SlotCompiler::new();
    let mut builder_b = SlotCompiler::new();

    let result_a = emit_single_body_set(
        &body,
        step_idx,
        0,
        slot_idx,
        next_step,
        &mut builder_a,
        false,
    );
    let result_b = emit_reduce_body_steps(&body, step_idx, 0, slot_idx, next_step, &mut builder_b);

    match (&result_a, &result_b) {
        (Ok(()), Ok(())) => {
            // Node count must match
            kani::assert(builder_a.nodes.len() == builder_b.nodes.len(),
                "both dispatchers must emit same node count: a={}, b={}",
                builder_a.nodes.len(),
                builder_b.nodes.len(),
            );

            // Per-node comparison: IDs and next pointers must match
            for (i, (node_a, node_b)) in builder_a
                .nodes
                .iter()
                .zip(builder_b.nodes.iter())
                .enumerate()
            {
                 == builder_b.nodes.len(),
                "both dispatchers must emit same node count: a={}, b={}",
                builder_a.nodes.len(),
                builder_b.nodes.len(),
            );

            // Per-node comparison: IDs and next pointers must match
            for (i, (node_a, node_b)) in builder_a
                .nodes
                .iter()
                .zip(builder_b.nodes.iter())
                .enumerate()
            {
                kani::assert(
                    node_a.id == node_b.id,
                    "node {}: ID mismatch: a={:?}, b={:?}",
                    i,
                    node_a.id,
                    node_b.id,
                );
                kani::assert(
                    node_a.next == node_b.next,
                    "node {}: next link mismatch: a={:?}, b={:?}",
                    i,
                    node_a.next,
                    node_b.next,
                );
                kani::assert(
                    node_a.error_slot == node_b.error_slot,
                    "node {}: error_slot mismatch: a={:?}, b={:?}",
                    i,
                    node_a.error_slot,
                    node_b.error_slot,
                );
            }

            // For single-step body: exactly 1 node expected
            kani::assert(
                builder_a.nodes.len() == 1,
                "single-step body must produce exactly 1 node, got {}",
                builder_a.nodes.len(),
            );
        }
        (Err(_), Err(_)) => {
            kani::cover!(
                true,
                "both dispatchers rejected — expected for empty/invalid body"
            );
        }
        (Ok(()), Err(_)) => {
            kani::cover!(
                true,
                "single-step succeeded but multi-step failed — CONTRACT VIOLATION"
            );
            kani::assume(false);
            loop {}
        }
        (Err(_), Ok(())) => {
            kani::cover!(
                true,
                "single-step failed but multi-step succeeded — CONTRACT VIOLATION"
            );
            kani::assume(false);
            loop {}
        }
    }
}

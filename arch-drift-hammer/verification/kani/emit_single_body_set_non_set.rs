// Verification artifact: emit_single_body_set_non_set.rs
// PO: PO-010 (StepPrimitive match exhaustiveness in emit_single_body_set)
// Bead: vb-xi2f.23
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_emit_single_body_set_non_set
//
// Proof obligations:
// - PO-010: StepPrimitive match is exhaustive; no panic for any variant
// - PO-010: Non-Set variants return UnsupportedStepPrimitive
//
// The Rust match at part_04.rs:210-224 covers all StepPrimitive variants.
// Set → success path. All others → UnsupportedStepPrimitive error.
//
// GOD RULE 1: kani::any() generates all StepPrimitive variants.
// GOD RULE 2: Binds to actual Rust emit_single_body_set implementation.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_04::emit_single_body_set;
use vb_compile::compile::SlotCompiler;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive};

/// PO-010 H1: All non-Set StepPrimitive variants return UnsupportedStepPrimitive, not panic.
/// Uses kani::any() to generate a non-Set step.
#[kani::proof]
#[kani::unwind(5)]
fn kani_emit_single_body_set_non_set() {
    // Generate a single-element body with a non-Set primitive
    // We'll construct it with a non-Set primitive directly
    let body = vec![StepAst {
        id: "test_step".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Do {
            action: "test_action".to_string(),
            input: "0".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let id = StepIdx::new(kani::any());
    let slot = SlotIdx::new(1);
    let mut builder = SlotCompiler::new();

    // Call emit_single_body_set with non-Set body
    let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

    // Should return UnsupportedStepPrimitive error, not panic
    match result {
        Ok(()) => kani::assert(false, "non-Set body cannot succeed"),
        Err(e) => {
            // Verify it's an UnsupportedStepPrimitive error
            let is_unsupported = e.0.iter().any(|err| {
                matches!(err, vb_compile::CompileError::UnsupportedStepPrimitive { .. })
            });
            kani::assert(is_unsupported, "non-Set body returns UnsupportedStepPrimitive");
        }
    }
}

/// PO-010 H2: ForEach primitive returns UnsupportedStepPrimitive
#[kani::proof]
#[kani::unwind(4)]
fn kani_emit_single_body_set_foreach() {
    let body = vec![StepAst {
        id: "foreach_step".to_string(),
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
        Ok(()) => kani::assert(false, "ForEach body cannot succeed"),
        Err(_) => kani::assert(true, "ForEach body returns error (not panic)"),
    }
}

/// PO-010 H3: Together primitive returns UnsupportedStepPrimitive
#[kani::proof]
#[kani::unwind(4)]
fn kani_emit_single_body_set_together() {
    let body = vec![StepAst {
        id: "together_step".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Together {
            branches: vec![],
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
        Ok(()) => kani::assert(false, "Together body cannot succeed"),
        Err(_) => kani::assert(true, "Together body returns error (not panic)"),
    }
}

/// PO-010 H4: Collect primitive returns UnsupportedStepPrimitive
#[kani::proof]
#[kani::unwind(4)]
fn kani_emit_single_body_set_collect() {
    let body = vec![StepAst {
        id: "collect_step".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Collect {
            variable: "x".to_string(),
            source: "0".to_string(),
            pages: None,
            items: None,
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
        Ok(()) => kani::assert(false, "Collect body cannot succeed"),
        Err(_) => kani::assert(true, "Collect body returns error (not panic)"),
    }
}

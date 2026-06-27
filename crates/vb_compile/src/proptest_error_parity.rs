// Verification artifact: proptest_error_parity.rs
// PO: PO-032 (Error parity combinatorial property)
// Bead: vb-xi2f.23
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_error_parity --test-threads=1
//
// Proof obligations:
// - PO-032: Unsupported variants return UnsupportedStepPrimitive; empty body returns StepFieldShape
//
// GOD RULE 1: Uses proptest with explicit non-Set variant generation.
// GOD RULE 2: Binds to actual Rust emit_single_body_set implementation.

#![cfg(test)]
#![forbid(unsafe_code)]

use super::SlotCompiler;
use super::part_04::emit_single_body_set;
use crate::CompileError;
use crate::ast::{StepAst, StepPrimitive};
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};

// ─────────────────────────────────────────────────────────────────
// Error parity strategies
// ─────────────────────────────────────────────────────────────────

/// Strategy for StepPrimitive variants that `emit_single_body_set` does not support.
fn unsupported_primitive_strategy() -> impl Strategy<Value = StepPrimitive> {
    prop_oneof![
        // ForEach
        ("[a-z]+", any::<Option<u32>>()).prop_map(|(input, at_once)| StepPrimitive::ForEach {
            variable: "x".to_string(),
            input,
            at_once,
            body: vec![],
        }),
        // Together
        Just(StepPrimitive::Together { branches: vec![] }),
        // Collect
        ("\\d+", any::<Option<u32>>(), any::<Option<u32>>()).prop_map(|(source, pages, items)| {
            StepPrimitive::Collect {
                variable: "x".to_string(),
                source,
                pages,
                items,
                body: vec![],
            }
        }),
        // Aggregate
        ("[a-z]+", "\\d+").prop_map(|(input, initial)| StepPrimitive::Aggregate {
            variable: "acc".to_string(),
            input,
            initial,
            body: vec![],
        }),
        // Repeat
        (1u16..100).prop_map(|max_attempts| StepPrimitive::Repeat {
            max_attempts,
            body: vec![],
        }),
        // Wait
        (any::<Option<String>>()).prop_map(|event| StepPrimitive::Wait {
            event,
            timeout: None
        }),
        // Ask
        (Just("prompt".to_string()), any::<Option<String>>())
            .prop_map(|(prompt, timeout)| { StepPrimitive::Ask { prompt, timeout } }),
        // Finish
        Just(StepPrimitive::Finish {
            result: crate::ast::ScalarValue::Integer(0),
        }),
    ]
}

/// Strategy for body with a specific primitive.
fn body_with_primitive(primitive: StepPrimitive) -> Vec<StepAst> {
    vec![StepAst {
        id: "test".to_string(),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }]
}

// ─────────────────────────────────────────────────────────────────
// PO-032: Error parity combinatorial
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-032 H1: Unsupported StepPrimitive variants return UnsupportedStepPrimitive.
    #[test]
    fn proptest_error_parity(primitive in unsupported_primitive_strategy()) {
        let body = body_with_primitive(primitive);
        let id = StepIdx::new(42);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, id.as_usize(), slot, None, &mut builder, false);

        // Must return error
        prop_assert!(result.is_err(), "unsupported primitive must return error");

        // Must be UnsupportedStepPrimitive
        let err = result.unwrap_err();
        let is_unsupported = err.0.iter().any(|e| {
            matches!(e, CompileError::UnsupportedStepPrimitive { step, .. }
                if *step == 42)
        });
        prop_assert!(is_unsupported, "unsupported primitive returns UnsupportedStepPrimitive with correct step");
    }

    /// PO-032 H2: Empty body returns StepFieldShape with correct step index.
    #[test]
    fn proptest_error_parity_empty(step_idx: u16) {
        let empty_body: Vec<StepAst> = vec![];
        let id = StepIdx::new(step_idx);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&empty_body, id, id.as_usize(), slot, None, &mut builder, false);

        prop_assert!(result.is_err(), "empty body must error");
        let err = result.unwrap_err();
        let is_correct = err.0.iter().any(|e| {
            matches!(e, CompileError::StepFieldShape { step, field, .. }
                if *step == usize::from(step_idx) && *field == "steps")
        });
        prop_assert!(is_correct, "StepFieldShape with correct step and field");
    }
}

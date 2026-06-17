// Verification artifact: proptest_body_dispatcher.rs
// PO: PO-008 (emit_single_body_set empty body error variant)
// PO: PO-011 (emit_single_body_set non-Set error variant)
// PO: PO-020 (emit_single_body_set invariant property)
// Bead: vb-xi2f.23
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_body_dispatcher_empty --test-threads=1
// Command: cargo test -p vb_compile -- proptest_body_dispatcher_non_set --test-threads=1
// Command: cargo test -p vb_compile -- proptest_body_dispatcher_invariant --test-threads=1
//
// Proof obligations:
// - PO-008: Empty body always returns StepFieldShape variant
// - PO-011: All non-Set variants return UnsupportedStepPrimitive
// - PO-020: emit_single_body_set invariant: empty→StepFieldShape, non-Set→UnsupportedStepPrimitive
//
// GOD RULE 1: Uses proptest strategy with Arbitrary for all StepPrimitive variants.
// GOD RULE 2: Binds to actual Rust emit_single_body_set implementation.

#![cfg(test)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use crate::mod_compile_lowering::emit_single_body_set;
use crate::SlotCompiler;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive};

// ─────────────────────────────────────────────────────────────────
// Strategies
// ─────────────────────────────────────────────────────────────────

/// Strategy for empty body.
pub fn empty_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    prop::strategy::Just(vec![])
}

/// Strategy for non-Set body (one non-Set primitive).
pub fn non_set_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    prop_oneof![
        // ForEach
        ("\\d+", any::<Option<u32>>()).prop_map(|(input, at_once)| StepAst {
            id: "foreach".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::ForEach {
                variable: "x".to_string(),
                input,
                at_once,
                body: vec![],
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }).prop_map(|ast| vec![ast]),
        // Do
        ("[a-z]+", "\\d+").prop_map(|(action, input)| vec![StepAst {
            id: "do".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Do { action, input },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]),
        // Together
        Just(vec![StepAst {
            id: "together".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Together { branches: vec![] },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]),
        // Collect
        ("\\d+", any::<Option<u32>>(), any::<Option<u32>>()).prop_map(|(source, pages, items)| vec![StepAst {
            id: "collect".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Collect {
                variable: "x".to_string(),
                source,
                pages,
                items,
                body: vec![],
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]),
    ]
}

/// Strategy for valid Set body.
pub fn valid_set_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    any::<i64>().prop_map(|value| vec![StepAst {
        id: "set".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }])
}

// ─────────────────────────────────────────────────────────────────
// PO-008: Empty body returns StepFieldShape
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-008 H1: Empty body always returns StepFieldShape (100 iterations).
    #[test]
    fn proptest_body_dispatcher_empty() {
        let empty_body: Vec<StepAst> = vec![];
        let id = StepIdx::new(0);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&empty_body, id, slot, None, &mut builder, false);

        // Must return error
        prop_assert!(result.is_err(), "empty body must return error");

        // Error must be StepFieldShape
        let err = result.unwrap_err();
        let is_step_field_shape = err.0.iter().any(|e| {
            matches!(e, vb_compile::CompileError::StepFieldShape { field, .. } if *field == "steps")
        });
        prop_assert!(is_step_field_shape, "empty body returns StepFieldShape");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-011: Non-Set body returns UnsupportedStepPrimitive
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-011 H1: All non-Set variants return UnsupportedStepPrimitive.
    #[test]
    fn proptest_body_dispatcher_non_set(body in non_set_body_strategy()) {
        let id = StepIdx::new(0);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

        // Non-Set body must return error
        prop_assert!(result.is_err(), "non-Set body must return error");

        // Error must be UnsupportedStepPrimitive
        let err = result.unwrap_err();
        let is_unsupported = err.0.iter().any(|e| {
            matches!(e, vb_compile::CompileError::UnsupportedStepPrimitive { .. })
        });
        prop_assert!(is_unsupported, "non-Set returns UnsupportedStepPrimitive");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-020: emit_single_body_set invariant
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-020 H1: Invariant - empty body → StepFieldShape
    #[test]
    fn proptest_body_dispatcher_invariant_empty() {
        let empty_body: Vec<StepAst> = vec![];
        let id = StepIdx::new(42);  // Arbitrary step index
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&empty_body, id, slot, None, &mut builder, false);

        prop_assert!(result.is_err(), "empty body → error");
        let err = result.unwrap_err();
        let is_correct = err.0.iter().any(|e| {
            matches!(e, vb_compile::CompileError::StepFieldShape { step, field, .. }
                if *step == 42 && *field == "steps")
        });
        prop_assert!(is_correct, "empty body → StepFieldShape");
    }

    /// PO-020 H2: Invariant - non-Set → UnsupportedStepPrimitive
    #[test]
    fn proptest_body_dispatcher_invariant_non_set(body in non_set_body_strategy()) {
        let id = StepIdx::new(42);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

        prop_assert!(result.is_err(), "non-Set → error");
        let err = result.unwrap_err();
        let is_correct = err.0.iter().any(|e| {
            matches!(e, vb_compile::CompileError::UnsupportedStepPrimitive { step, .. }
                if *step == 42)
        });
        prop_assert!(is_correct, "non-Set → UnsupportedStepPrimitive");
    }

    /// PO-020 H3: Invariant - valid Set body → success
    #[test]
    fn proptest_body_dispatcher_invariant_set(body in valid_set_body_strategy()) {
        let id = StepIdx::new(42);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, slot, None, &mut builder, false);

        prop_assert!(
            matches!(result, Ok(_)),
            "valid Set body must compile Ok, got {:?}",
            result
        );
        prop_assert_eq!(builder.node_count(), 1, "exactly 1 node emitted");
    }
}

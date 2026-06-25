// Verification artifact: proptest_body_dispatcher.rs
// PO: PO-008 (emit_single_body_set empty body error variant)
// PO: PO-011 (emit_single_body_set unsupported primitive error variant)
// PO: PO-020 (emit_single_body_set invariant property)
// Bead: vb-xi2f.23
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_body_dispatcher_empty --test-threads=1
// Command: cargo test -p vb_compile -- proptest_body_dispatcher_non_set --test-threads=1
// Command: cargo test -p vb_compile -- proptest_body_dispatcher_invariant --test-threads=1
//
// Proof obligations:
// - PO-008: Empty body always returns StepFieldShape variant
// - PO-011: All unsupported variants return UnsupportedStepPrimitive
// - PO-020: emit_single_body_set invariant: empty→StepFieldShape,
//   unsupported→UnsupportedStepPrimitive, Set/Do→success
//
// GOD RULE 1: Uses proptest strategy with Arbitrary for all StepPrimitive variants.
// GOD RULE 2: Binds to actual Rust emit_single_body_set implementation.

#![cfg(test)]
#![forbid(unsafe_code)]

use super::SlotCompiler;
use super::part_04::emit_single_body_set;
use crate::CompileError;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive};

// ─────────────────────────────────────────────────────────────────
// Strategies
// ─────────────────────────────────────────────────────────────────

/// Strategy for one primitive that `emit_single_body_set` must reject.
fn unsupported_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    prop_oneof![
        // ForEach
        ("\\d+", any::<Option<u32>>())
            .prop_map(|(input, at_once)| StepAst {
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
            })
            .prop_map(|ast| vec![ast]),
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
        ("\\d+", any::<Option<u32>>(), any::<Option<u32>>()).prop_map(
            |(source, pages, items)| vec![StepAst {
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
            }]
        ),
    ]
}

/// Strategy for a valid numeric Do body.
fn valid_do_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    (0u16..128, 0u16..128).prop_map(|(action, input)| {
        vec![StepAst {
            id: "do".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Do {
                action: action.to_string(),
                input: input.to_string(),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }]
    })
}

/// Strategy for valid Set body.
fn valid_set_body_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    any::<i64>().prop_map(|value| {
        vec![StepAst {
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
        }]
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-008: Empty body returns StepFieldShape
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-008 H1: Empty body always returns StepFieldShape (100 iterations).
    #[test]
    fn proptest_body_dispatcher_empty(_unit in Just(())) {
        let empty_body: Vec<StepAst> = vec![];
        let id = StepIdx::new(0);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&empty_body, id, id.as_usize(), slot, None, &mut builder, false);

        // Must return error
        prop_assert!(result.is_err(), "empty body must return error");

        // Error must be StepFieldShape
        let err = result.unwrap_err();
        let is_step_field_shape = err.0.iter().any(|e| {
            matches!(e, CompileError::StepFieldShape { field, .. } if *field == "steps")
        });
        prop_assert!(is_step_field_shape, "empty body returns StepFieldShape");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-011: Non-Set body returns UnsupportedStepPrimitive
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-011 H1: Unsupported variants return UnsupportedStepPrimitive.
    #[test]
    fn proptest_body_dispatcher_unsupported(body in unsupported_body_strategy()) {
        let id = StepIdx::new(0);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, id.as_usize(), slot, None, &mut builder, false);

        // Unsupported body must return error.
        prop_assert!(result.is_err(), "unsupported body must return error");

        // Error must be UnsupportedStepPrimitive
        let err = result.unwrap_err();
        let is_unsupported = err.0.iter().any(|e| {
            matches!(e, CompileError::UnsupportedStepPrimitive { .. })
        });
        prop_assert!(is_unsupported, "unsupported body returns UnsupportedStepPrimitive");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-020: emit_single_body_set invariant
// ─────────────────────────────────────────────────────────────────

proptest! {
    /// PO-020 H1: Invariant - empty body → StepFieldShape
    #[test]
    fn proptest_body_dispatcher_invariant_empty(_unit in Just(())) {
        let empty_body: Vec<StepAst> = vec![];
        let id = StepIdx::new(42);  // Arbitrary step index
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&empty_body, id, id.as_usize(), slot, None, &mut builder, false);

        prop_assert!(result.is_err(), "empty body → error");
        let err = result.unwrap_err();
        let is_correct = err.0.iter().any(|e| {
            matches!(e, CompileError::StepFieldShape { step, field, .. }
                if *step == 42 && *field == "steps")
        });
        prop_assert!(is_correct, "empty body → StepFieldShape");
    }

    /// PO-020 H2: Invariant - unsupported → UnsupportedStepPrimitive
    #[test]
    fn proptest_body_dispatcher_invariant_unsupported(body in unsupported_body_strategy()) {
        let id = StepIdx::new(42);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, id.as_usize(), slot, None, &mut builder, false);

        prop_assert!(result.is_err(), "unsupported → error");
        let err = result.unwrap_err();
        let is_correct = err.0.iter().any(|e| {
            matches!(e, CompileError::UnsupportedStepPrimitive { step, .. }
                if *step == 42)
        });
        prop_assert!(is_correct, "unsupported → UnsupportedStepPrimitive");
    }

    /// PO-020 H3: Invariant - valid Set body → success
    #[test]
    fn proptest_body_dispatcher_invariant_set(body in valid_set_body_strategy()) {
        let id = StepIdx::new(42);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, id.as_usize(), slot, None, &mut builder, false);

        prop_assert!(result.is_ok(), "valid Set → Ok");
        prop_assert_eq!(builder.nodes.len(), 1, "exactly 1 node emitted");
    }

    /// PO-020 H4: Invariant - valid Do body → success
    #[test]
    fn proptest_body_dispatcher_invariant_do(body in valid_do_body_strategy()) {
        let id = StepIdx::new(42);
        let slot = SlotIdx::new(1);
        let mut builder = SlotCompiler::new();

        let result = emit_single_body_set(&body, id, id.as_usize(), slot, None, &mut builder, false);

        prop_assert!(result.is_ok(), "valid Do → Ok");
        prop_assert_eq!(builder.nodes.len(), 1, "exactly 1 node emitted");
    }
}

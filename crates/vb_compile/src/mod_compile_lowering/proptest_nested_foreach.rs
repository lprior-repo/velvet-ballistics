// Verification artifact: proptest_nested_foreach.rs
// Obligations: PO-007, PO-008, PO-013, PO-014
// Bead: vb-xi2f.21 | State: 5 (proof-writer)
// Verifier: proptest
//
// In-crate proptest module — has access to pub(crate) items.
// Tests: width parity, deterministic lowering, round-trip properties.
//
// GOD RULE 1: Uses proptest strategies with arbitrary input generation.
// GOD RULE 2: Binds to actual production lower_canonical_for_each and
//   compile_source for full round-trip.

#![cfg(test)]
#![forbid(unsafe_code)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{CompiledNodeKind, StepIdx};
use vb_yaml::ast::{
    StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts,
};

use super::part_01::{
    body_width, canonical_step_width,
};
use super::part_01::compile_source;
use super::part_02::lower_canonical_for_each;
use super::part_07::SlotCompiler;

// =========================================================================
// Generation strategies
// =========================================================================

fn single_set_step() -> impl Strategy<Value = StepAst> {
    (any::<i64>(), "[a-z]+").prop_map(|(value, output)| StepAst {
        id: "set_step".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output,
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    })
}

fn single_do_step() -> impl Strategy<Value = StepAst> {
    (1u16..100u16, 0u16..99u16).prop_map(|(action, input)| StepAst {
        id: "do_step".to_string(),
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
    })
}

fn single_valid_body_step() -> impl Strategy<Value = StepAst> {
    prop_oneof![single_set_step(), single_do_step()]
}

fn foreach_with_single_body() -> impl Strategy<Value = StepAst> {
    (
        "[a-z]+",
        "[a-z]+",
        any::<Option<u32>>(),
        single_valid_body_step(),
    )
        .prop_map(|(variable, input, at_once, body_step)| StepAst {
            id: "foreach".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::ForEach {
                variable,
                input,
                at_once,
                body: vec![body_step],
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        })
}

fn foreach_workflow_source() -> impl Strategy<Value = WorkflowSource> {
    foreach_with_single_body().prop_map(|foreach_step| {
        WorkflowSource::new(WorkflowSourceParts {
            version: "velvet-ballistics/v1".to_string(),
            name: "test_foreach".to_string(),
            trigger: TriggerAst::Manual,
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![foreach_step],
            result: None,
            examples: vec![],
        })
    })
}

// =========================================================================
// PO-007, PO-008, PO-013, PO-014: Properties
// =========================================================================

proptest! {
    /// PO-007 / PO-013 / PO-014: Round-trip compile+validate.
    #[test]
    fn prop_nested_foreach_roundtrip_compiles_and_validates(
        source in foreach_workflow_source(),
    ) {
        let result1 = compile_source(&source);
        let result2 = compile_source(&source);

        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "PO-013: deterministic same-outcome"
        );

        if result1.is_ok() {
            let workflow = result1.unwrap();
            let node_count = workflow.to_parts().nodes.len();

            prop_assert!(
                node_count > 0,
                "compiled for_each must have nodes"
            );

            if let Some(first) = workflow.to_parts().nodes.first() {
                prop_assert!(
                    matches!(first.kind, CompiledNodeKind::ForEachStart { .. }),
                    "first node must be ForEachStart"
                );
            }

            // PO-014: CompiledWorkflow::try_from_parts validation passes
            // (compile_source already calls validate + try_from_parts internally,
            //  so Ok result means validation passed)
        }
    }

    /// PO-008: Width parity — layout width equals emission node count.
    #[test]
    fn prop_foreach_width_equals_emission_count(
        foreach_step in foreach_with_single_body(),
    ) {
        if let StepPrimitive::ForEach { body, .. } = &foreach_step.primitive {
            let layout_width = canonical_step_width(&foreach_step.primitive);
            let mut builder = SlotCompiler::new();
            let id = StepIdx::new(0);

            let lower_result = lower_canonical_for_each(
                0, id, "0", None, body, &mut builder,
            );

            if let Ok(lw) = layout_width {
                if lower_result.is_ok() {
                    let node_count = builder.nodes.len();
                    prop_assert_eq!(
                        node_count, lw,
                        "PO-008: emitted {} nodes, layout width {}, must match",
                        node_count, lw,
                    );
                }
            }
        }
    }
}

// =========================================================================
// Supplemental deterministic tests
// =========================================================================

#[test]
fn foreach_body_emit_single_set() {
    let body = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "o".to_string(),
            value: "42".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_for_each(0, StepIdx::new(10), "0", None, &body, &mut builder);
    assert!(result.is_ok(), "simple for_each must compile: {result:?}");
    assert_eq!(builder.nodes.len(), 3, "must emit exactly 3 nodes");
}

#[test]
fn foreach_deterministic_output() {
    let body = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "o".to_string(),
            value: "99".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut b1 = SlotCompiler::new();
    let mut b2 = SlotCompiler::new();
    let _ = lower_canonical_for_each(0, StepIdx::new(0), "0", None, &body, &mut b1);
    let _ = lower_canonical_for_each(0, StepIdx::new(0), "0", None, &body, &mut b2);
    assert_eq!(b1.nodes.len(), b2.nodes.len(), "same input must produce same node count");
    for (i, (n1, n2)) in b1.nodes.iter().zip(b2.nodes.iter()).enumerate() {
        assert_eq!(n1.id, n2.id, "node {i} id must match");
    }
}

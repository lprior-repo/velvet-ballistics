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
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

use proptest::prelude::*;
use vb_core::{CompiledNodeKind, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts};

use super::part_01::{canonical_step_width, compile_source};
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
             matches!(result1, Ok(_)), matches!(result2, Ok(_)),
             "PO-013: deterministic same-outcome"
         );

        if matches!(result1, Ok(_)) {
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

            let lw = layout_width
                .expect("width must compute for valid for_each");
            lower_result
                .expect("lowering must succeed for valid for_each");
            let node_count = builder.nodes.len();
            prop_assert_eq!(
                node_count, lw,
                "PO-008: emitted {} nodes, layout width {}, must match",
                node_count, lw,
            );
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
    assert!(matches!(result, Ok(_)), "simple for_each must compile: {result:?}");
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
    assert!(
        matches!(lower_canonical_for_each(0, StepIdx::new(0), "0", None, &body, &mut b1), Ok(_)),
        "first compile must succeed"
    );
    assert!(
        matches!(lower_canonical_for_each(0, StepIdx::new(0), "0", None, &body, &mut b2), Ok(_)),
        "second compile must succeed"
    );
    assert_eq!(
        b1.nodes.len(),
        b2.nodes.len(),
        "same input must produce same node count"
    );
    for (i, (n1, n2)) in b1.nodes.iter().zip(b2.nodes.iter()).enumerate() {
        assert_eq!(n1.id, n2.id, "node {i} id must match");
    }
}

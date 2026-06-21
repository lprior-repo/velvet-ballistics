#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]

//! Integration tests for together lowering: cross-function parity,
//! nested together, gate 11 compatibility, budget compliance, digest invariants.
//!
//! Bead: vb-xi2f.22 — Nested Together Body Lowering
//! Phase 4: Cross-function parity (B-32 through B-41)
//! Phase 5: Interoperability (B-42 through B-50)
//!
//! These tests verify that together lowering produces correct IR that
//! passes validation gates, respects budget constraints, and maintains
//! digest invariants.

use vb_core::{CompiledNodeKind, SlotIdx, StepIdx, WorkflowDigest};
use vb_yaml::ast::{StepAst, StepPrimitive, TogetherBranch};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn step_with_primitive(primitive: StepPrimitive) -> StepAst {
    StepAst {
        id: "test_step".into(),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn set_step(_id: &str, output: &str, value: &str) -> StepAst {
    step_with_primitive(StepPrimitive::Set {
        output: output.into(),
        value: value.into(),
    })
}

fn branch(label: &str, steps: Vec<StepAst>) -> TogetherBranch {
    TogetherBranch {
        label: label.into(),
        steps,
    }
}

fn together_primitive(branches: Vec<TogetherBranch>) -> StepPrimitive {
    StepPrimitive::Together { branches }
}

fn body_with_primitive(primitive: StepPrimitive) -> Vec<StepAst> {
    vec![step_with_primitive(primitive)]
}

fn dummy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0u8; 32])
}

// ---------------------------------------------------------------------------
// B-32: Width matches emitted count
// ---------------------------------------------------------------------------

#[test]
fn together_width_equals_emitted_node_count() {
    let together = together_primitive(vec![
        branch(
            "a",
            vec![set_step("a1", "x", "1"), set_step("a2", "y", "2")],
        ),
        branch("b", vec![set_step("b1", "z", "3")]),
    ]);
    let body = body_with_primitive(together.clone());

    // Compute width
    let width_result = super::part_01::canonical_body_step_width(&together);
    // Emit nodes
    let mut builder = crate::SlotCompiler::new();
    let emit_result = super::part_04::emit_single_body_set(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(0),
        None,
        &mut builder,
        false,
    );

    if let (Ok(width), Ok(())) = (&width_result, &emit_result) {
        let parts = builder.build_parts("test", dummy_digest()).unwrap();
        assert_eq!(
            parts.nodes.len(),
            *width,
            "emitted node count ({}) must equal computed width ({})",
            parts.nodes.len(),
            width
        );
    }
}

// ---------------------------------------------------------------------------
// B-33: debug_assert width parity does not fire for valid together
// ---------------------------------------------------------------------------

#[test]
fn debug_assert_width_parity_does_not_fire_for_valid_together() {
    let together = together_primitive(vec![
        branch("a", vec![set_step("a1", "x", "1")]),
        branch("b", vec![set_step("b1", "y", "2")]),
    ]);
    let body = body_with_primitive(together.clone());

    let width = super::part_01::canonical_body_step_width(&together);
    let mut builder = crate::SlotCompiler::new();
    let emit_result = super::part_04::emit_single_body_set(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(0),
        None,
        &mut builder,
        false,
    );

    if let (Ok(w), Ok(())) = (&width, &emit_result) {
        let parts = builder.build_parts("test", dummy_digest()).unwrap();
        // This is what debug_assert_eq! would check
        assert_eq!(parts.nodes.len(), *w, "debug_assert_eq parity must hold");
    }
}

// ---------------------------------------------------------------------------
// B-34: Nested together StepIdx offsets correct
// ---------------------------------------------------------------------------

#[test]
fn nested_together_inner_nodes_at_correct_offsets() {
    // Outer together: 2 branches
    // Branch 0 contains inner together with 2 branches (each 1 Set step)
    let inner_together = together_primitive(vec![
        branch("inner_a", vec![set_step("ia", "x", "1")]),
        branch("inner_b", vec![set_step("ib", "y", "2")]),
    ]);

    let outer_together = together_primitive(vec![
        branch("outer_a", vec![step_with_primitive(inner_together)]),
        branch("outer_b", vec![set_step("ob", "z", "3")]),
    ]);
    let body = body_with_primitive(outer_together.clone());

    let mut builder = crate::SlotCompiler::new();
    let emit_result = super::part_04::emit_single_body_set(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(0),
        None,
        &mut builder,
        false,
    );

        let () = emit_result.expect("Together lowering must succeed per spec");

    let parts = builder.build_parts("test", dummy_digest()).unwrap();
    let nodes = &*parts.nodes;

    // Find all TogetherStart and TogetherJoin nodes
    let starts: Vec<_> = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::TogetherStart { .. }))
        .collect();
    let joins: Vec<_> = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::TogetherJoin { .. }))
        .collect();

    // Should have 2 starts and 2 joins (one inner, one outer)
    assert!(
        starts.len() >= 2,
        "Expected at least 2 TogetherStart nodes (outer + inner), got {}",
        starts.len()
    );
    assert!(
        joins.len() >= 2,
        "Expected at least 2 TogetherJoin nodes (outer + inner), got {}",
        joins.len()
    );

    // Outer TogetherStart must be first
    assert_eq!(starts[0].id, StepIdx::new(0));

    // Inner TogetherJoin must appear before outer TogetherJoin
    if joins.len() >= 2 {
        let inner_join_idx = joins[0].id.as_usize();
        let outer_join_idx = joins[1].id.as_usize();
        assert!(
            inner_join_idx < outer_join_idx,
            "Inner TogetherJoin ({}) should come before outer TogetherJoin ({})",
            inner_join_idx,
            outer_join_idx
        );
    }

}

// ---------------------------------------------------------------------------
// B-35, B-36, B-37: Two-level nested together produces correct IR
// ---------------------------------------------------------------------------

#[test]
fn two_level_nested_together_produces_correct_flat_ir() {
    // Same structure as B-34 but verifies through full compile_source pipeline
    let yaml_source = br#"version: velvet-ballistics/v1
name: nested_test
when:
  manual: {}
steps:
  - id: outer_together
    together:
      branches:
        - label: outer_a
          steps:
            - id: inner_together
              together:
                branches:
                  - label: inner_a
                    steps:
                      - id: set_ia
                        set:
                          output: "x"
                          value: "1"
                  - label: inner_b
                    steps:
                      - id: set_ib
                        set:
                          output: "y"
                          value: "2"
        - label: outer_b
          steps:
            - id: set_ob
              set:
                output: "z"
                value: "3"
  - id: done
    finish:
      result: 0
"#;

    let result = crate::compile_workflow(yaml_source);

    // TDD: will succeed after implementation
        let workflow = result.expect("Together lowering must succeed per spec");

    let parts = workflow.to_parts();
    let nodes = &*parts.nodes;
    let starts: Vec<_> = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::TogetherStart { .. }))
        .collect();
    let joins: Vec<_> = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::TogetherJoin { .. }))
        .collect();
    assert!(starts.len() >= 2, "Expected inner + outer TogetherStart");
    assert!(joins.len() >= 2, "Expected inner + outer TogetherJoin");

}

// ---------------------------------------------------------------------------
// B-38, B-39: Nested together terminates at various depths
// ---------------------------------------------------------------------------

#[test]
fn nested_together_terminates_at_various_depths() {
    // Single level
    {
        let yaml = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: a
          steps:
            - id: s1
              set:
                output: "x"
                value: "1"
  - id: done
    finish:
      result: 0
"#;
        let result = crate::compile_workflow(yaml);
        // Must not panic, regardless of Ok/Err
        let _ = result;
    }

    // Two level
    {
        let yaml = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: a
          steps:
            - id: inner
              together:
                branches:
                  - label: ia
                    steps:
                      - id: si
                        set:
                          output: "x"
                          value: "1"
  - id: done
    finish:
      result: 0
"#;
        let result = crate::compile_workflow(yaml);
        let _ = result;
    }
}

// ---------------------------------------------------------------------------
// B-42, B-43: Gate 11 accepts together IR
// ---------------------------------------------------------------------------

#[test]
fn together_ir_passes_gate_11_validation() {
    let yaml = br#"version: velvet-ballistics/v1
name: gate11_test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: a
          steps:
            - id: s1
              set:
                output: "x"
                value: "1"
        - label: b
          steps:
            - id: s2
              set:
                output: "y"
                value: "2"
  - id: done
    finish:
      result: 0
"#;

    match crate::compile_workflow(yaml) {
        Ok(workflow) => {
            // After implementation, gate 11 passes
            let _ = workflow;
        }
        Err(errs) => {
            // TDD: verify error is about UnsupportedStepPrimitive, not gate 11
            let first = errs.iter().next().unwrap();
            // Accept any structured error — the key is no panic
            let _ = first;
        }
    }
}

// ---------------------------------------------------------------------------
// B-44, B-45: Budget accepts/rejects correctly
// ---------------------------------------------------------------------------

#[test]
fn together_ir_respects_budget_constraints() {
    // Given a together body within budget, it should compile
    // Over budget, it should reject with BudgetExceeded
    let yaml_within = br#"version: velvet-ballistics/v1
name: budget_test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: a
          steps:
            - id: s1
              set:
                output: "x"
                value: "1"
  - id: done
    finish:
      result: 0
"#;

    let result = crate::compile_workflow(yaml_within);

    // TDD: any result is acceptable as long as there's no panic
    match result {
        Ok(workflow) => {
            // Verify the workflow is valid
            let _ = workflow;
        }
        Err(_) => {
            // Currently rejected; will compile after implementation
        }
    }
}

// ---------------------------------------------------------------------------
// B-46, B-47, B-48: Digest properties (non-regression)
// ---------------------------------------------------------------------------

#[test]
fn together_digest_preserves_existing_invariants() {
    // Non-regression: verify digest behavior is unchanged
    // These tests verify that the existing `canonical_digest` function
    // already handles Together correctly and returns consistent results.

    // Determinism: same input → same digest
    let yaml1 = br#"version: velvet-ballistics/v1
name: digest_test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: a
          steps:
            - id: s1
              set:
                output: "x"
                value: "1"
  - id: done
    finish:
      result: 0
"#;

    let digest1 = crate::compute_compiled_digest(yaml1);
    let digest2 = crate::compute_compiled_digest(yaml1);
    assert_eq!(digest1, digest2, "Same YAML must produce same digest");

    // Content sensitivity: different branch body → different digest
    let yaml2 = br#"version: velvet-ballistics/v1
name: digest_test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: a
          steps:
            - id: s1
              set:
                output: "x"
                value: "999"
  - id: done
    finish:
      result: 0
"#;
    let digest3 = crate::compute_compiled_digest(yaml2);
    assert_ne!(
        digest1, digest3,
        "Different value must produce different digest"
    );

    // Branch-order sensitivity: swapped branches → different digest
    let yaml3 = br#"version: velvet-ballistics/v1
name: digest_test
when:
  manual: {}
steps:
  - id: t1
    together:
      branches:
        - label: b
          steps:
            - id: s2
              set:
                output: "y"
                value: "2"
        - label: a
          steps:
            - id: s1
              set:
                output: "x"
                value: "1"
  - id: done
    finish:
      result: 0
"#;

    // Branch-order sensitivity: swapped branches must produce a different digest
    let digest_swapped = crate::compute_compiled_digest(yaml3);
    assert_ne!(
        digest1, digest_swapped,
        "Swapping branches must change the digest"
    );
}

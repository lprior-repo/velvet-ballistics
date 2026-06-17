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
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
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
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
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
    clippy::useless_asref,
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
//! Topology tests for bead vb-a001: for_each compiled parity fix.
//!
//! Verifies that `v1_primitive_lowering` emits the correct
//! ForEachNext next-edge on the body SetConst so run-compiled
//! does not reject the compiled IR as unreachable.

use vb_compile::{YamlCompiler, compile_workflow, lower_set, lower_steps_to_ir};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};

// ---------------------------------------------------------------------------
// Corpus YAML for the for_each workflow under vb-a001
// ---------------------------------------------------------------------------

const FOREACH_YAML: &str = "version: velvet-ballistics/v1
name: fuzz-foreach
when:
  manual: {}
steps:
  - id: loop
    for_each:
      variable: item
      input: \"0\"
      at_once: 2
      steps:
        - id: capture
          set:
            output: seen
            value: \"1\"
  - id: done
    finish:
      result: 0
";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compile_yaml(yaml: &str) -> Result<vb_core::CompiledWorkflow, Box<dyn std::error::Error>> {
    let workflow = compile_workflow(yaml.as_bytes()).map_err(|errors| {
        let msgs: Vec<_> = errors.iter().map(|e| e.to_string()).collect();
        msgs.join("; ")
    })?;
    Ok(workflow)
}

/// Returns node kind names in StepIdx order.
fn node_kind_names(parts: &WorkflowParts) -> Vec<&'static str> {
    parts
        .nodes
        .iter()
        .map(|n| match &n.kind {
            CompiledNodeKind::Nop => "Nop",
            CompiledNodeKind::SetConst { .. } => "SetConst",
            CompiledNodeKind::Copy { .. } => "Copy",
            CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
            CompiledNodeKind::BuildObject { .. } => "BuildObject",
            CompiledNodeKind::BuildList { .. } => "BuildList",
            CompiledNodeKind::Do { .. } => "Do",
            CompiledNodeKind::Choose { .. } => "Choose",
            CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
            CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
            CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
            CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
            CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
            CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
            CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
            CompiledNodeKind::CollectStart { .. } => "CollectStart",
            CompiledNodeKind::CollectPage { .. } => "CollectPage",
            CompiledNodeKind::CollectNext { .. } => "CollectNext",
            CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
            CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
            CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
            CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
            CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
            CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
            CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
            CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
            CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
            CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
            CompiledNodeKind::Ask { .. } => "Ask",
            CompiledNodeKind::AskResume { .. } => "AskResume",
            CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
            CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
            CompiledNodeKind::Jump { .. } => "Jump",
            CompiledNodeKind::Finish { .. } => "Finish",
            _ => "Unknown",
        })
        .collect()
}

// ---------------------------------------------------------------------------
// vb-a001 topology tests (compiled YAML path)
// ---------------------------------------------------------------------------

/// TEST-001: for_each compiled IR has the expected node kind sequence.
///
/// After v1_primitive_lowering the sequence must be:
///   [ForEachStart, SetConst(body), ForEachNext, Finish]
#[test]
fn vb_a001_for_each_node_kind_sequence() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let kinds = node_kind_names(&workflow.to_parts());
    let expected = vec!["ForEachStart", "SetConst", "ForEachNext", "Finish"];
    assert_eq!(
        kinds, expected,
        "for_each node kinds must match expected sequence"
    );
}

/// TEST-002: body SetConst next edge points to ForEachNext (the vb-a001 fix).
#[test]
fn vb_a001_body_setconst_next_points_to_foreachnext() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let parts = workflow.to_parts();

    // Node 0: ForEachStart — body→1, done→3, next is None (loop semantics)
    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            assert_eq!(
                body,
                &StepIdx::new(1),
                "ForEachStart body → node 1 (SetConst)"
            );
            assert_eq!(
                done,
                &StepIdx::new(3),
                "ForEachStart done → node 3 (Finish)"
            );
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ForEachStart { .. }),
            "node 0 expected ForEachStart, got {other:?}"
        ),
    }

    // Node 1: SetConst (body) — THE FIX: next must be Some(StepIdx(2))
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    match &node1.kind {
        CompiledNodeKind::SetConst { value } => {
            assert_eq!(value, &ConstIdx::new(0), "body SetConst const index");
            let next = node1
                .next
                .expect("body SetConst must have a next edge to ForEachNext");
            assert_eq!(
                next,
                StepIdx::new(2),
                "body SetConst next must point to ForEachNext at index 2"
            );
        }
        other => assert!(
            matches!(other, CompiledNodeKind::SetConst { .. }),
            "node 1 expected SetConst, got {other:?}"
        ),
    }

    // Node 2: ForEachNext — body→1 (loop), done→3 (exit)
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            assert_eq!(body, &StepIdx::new(1), "ForEachNext body → SetConst at 1");
            assert_eq!(done, &StepIdx::new(3), "ForEachNext done → Finish at 3");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ForEachNext { .. }),
            "node 2 expected ForEachNext, got {other:?}"
        ),
    }

    // Node 3: Finish — terminal
    let node3 = parts.nodes.get(3).expect("node 3 must exist");
    match &node3.kind {
        CompiledNodeKind::Finish { result } => {
            assert_eq!(result, &SlotIdx::new(0));
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Finish { .. }),
            "node 3 expected Finish, got {other:?}"
        ),
    }
}

/// TEST-003: manually constructed for_each with correct body→ForEachNext chain
/// passes lower_steps_to_ir validation.
///
/// This is the direct parity test: building the IR node-by-node and verifying
/// that a properly connected for_each passes validation.
#[test]
fn vb_a001_lower_steps_to_ir_accepts_connected_foreach() {
    let foreach_start = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 2,
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    };

    // THE FIX: body SetConst next → ForEachNext (index 2)
    let body_set = lower_set(
        StepIdx::new(1),
        SlotIdx::new(1),
        ConstIdx::new(0),
        Some(StepIdx::new(2)), // ← this is the vb-a001 fix
    );

    let foreach_next = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    };

    let finish = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };

    let result = lower_steps_to_ir(
        vec![foreach_start, body_set, foreach_next, finish],
        vec![],
        vec![],
        vec![vb_core::value::ConstValue::I64(1)],
        2,
        0,
        "vb-a001-connected",
        WorkflowDigest::from_bytes([1; 32]),
    );

    assert!(
        result.is_ok(),
        "for_each with body→ForEachNext chain must pass validation, got: {:?}",
        result
    );
}

/// TEST-004: lower_steps_to_ir rejects for_each where body SetConst has no
/// next edge (the vb-a001 bug condition).
#[test]
fn vb_a001_lower_steps_to_ir_rejects_disconnected_body() {
    // Node 0: ForEachStart with body→1, done→3
    let foreach_start = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 2,
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    };

    // BUG: body SetConst next = None → unreachable nodes
    let body_set = lower_set(
        StepIdx::new(1),
        SlotIdx::new(1),
        ConstIdx::new(0),
        None, // ← missing next edge
    );

    let foreach_next = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    };

    let finish = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };

    let result = lower_steps_to_ir(
        vec![foreach_start, body_set, foreach_next, finish],
        vec![],
        vec![],
        vec![vb_core::value::ConstValue::I64(1)],
        2,
        0,
        "vb-a001-disconnected",
        WorkflowDigest::from_bytes([2; 32]),
    );

    assert!(
        result.is_err(),
        "for_each with disconnected body must be rejected by validation"
    );
}

/// TEST-005: all three compile APIs produce identical for_each topology.
#[test]
fn vb_a001_all_compile_apis_agree_on_foreach_topology() {
    let w1 = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let w2 = YamlCompiler::default()
        .compile(FOREACH_YAML.as_bytes())
        .expect("YamlCompiler::compile must succeed for FOREACH_YAML");

    let kinds1 = node_kind_names(&w1.to_parts());
    let kinds2 = node_kind_names(&w2.to_parts());

    assert_eq!(
        kinds1, kinds2,
        "compile_workflow and YamlCompiler::compile must produce same node kinds"
    );
    assert_eq!(
        w1.to_parts().slot_count,
        w2.to_parts().slot_count,
        "slot_count must agree across APIs"
    );
}

/// TEST-006: slot count for for_each is exactly 2 (input + item).
#[test]
fn vb_a001_for_each_slot_count_is_two() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    assert_eq!(
        workflow.to_parts().slot_count,
        2,
        "for_each must allocate exactly 2 slots (input + item)"
    );
}

/// TEST-007: ForEachStart body/done/iterator_slot are all valid.
#[test]
fn vb_a001_foreachstart_fields_valid() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let parts = workflow.to_parts();
    let slot_count = usize::try_from(parts.slot_count).expect("slot_count fits in usize");

    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => {
            assert!(*limit > 0, "limit must be positive");
            assert!(body.as_usize() < parts.nodes.len(), "body in-range");
            assert!(done.as_usize() < parts.nodes.len(), "done in-range");
            assert!(input.as_usize() < slot_count, "input slot in-range");
            assert!(item_slot.as_usize() < slot_count, "item_slot in-range");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ForEachStart { .. }),
            "node 0 expected ForEachStart, got {other:?}"
        ),
    }
}

/// TEST-008: ForEachNext targets are in-bounds.
#[test]
fn vb_a001_foreachnext_targets_in_bounds() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let parts = workflow.to_parts();

    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            assert!(
                body.as_usize() < parts.nodes.len(),
                "ForEachNext body in-range"
            );
            assert!(
                done.as_usize() < parts.nodes.len(),
                "ForEachNext done in-range"
            );
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ForEachNext { .. }),
            "node 2 expected ForEachNext, got {other:?}"
        ),
    }
}

/// TEST-009: node count for canonical for_each is exactly 4.
#[test]
fn vb_a001_for_each_node_count_is_four() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    assert_eq!(
        workflow.to_parts().nodes.len(),
        4,
        "canonical for_each must produce exactly 4 nodes"
    );
}

/// TEST-010: finish node is last and has no next edge.
#[test]
fn vb_a001_finish_is_last_with_no_next() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let parts = workflow.to_parts();
    let last = parts.nodes.last().expect("nodes must be non-empty");
    assert!(
        matches!(&last.kind, CompiledNodeKind::Finish { .. }),
        "last node must be Finish"
    );
    assert!(last.next.is_none(), "finish must have no next edge");
}

/// TEST-011: for_each nodes are at expected StepIdx positions.
#[test]
fn vb_a001_foreach_nodes_at_expected_positions() {
    let workflow = compile_yaml(FOREACH_YAML).expect("compile_yaml must succeed for FOREACH_YAML");
    let parts = workflow.to_parts();

    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    let node3 = parts.nodes.get(3).expect("node 3 must exist");
    assert_eq!(node0.id, StepIdx::new(0), "ForEachStart at 0");
    assert_eq!(node1.id, StepIdx::new(1), "body SetConst at 1");
    assert_eq!(node2.id, StepIdx::new(2), "ForEachNext at 2");
    assert_eq!(node3.id, StepIdx::new(3), "Finish at 3");
}

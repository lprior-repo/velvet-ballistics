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
    unused_variables
)]
#![forbid(unsafe_code)]
//! For-each ordering property tests: the bounded `ForEachStart`,
//! `ForEachNext`, and `ForEachJoin` triples agree on `limit` and slot
//! references; randomized limits and slots reveal ordering regressions.

use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

// =========================================================================
// Strategies
// =========================================================================

/// Strategy for a small `limit` value within the for-each bound (u32).
fn arb_limit() -> impl Strategy<Value = u32> {
    0u32..16
}

/// Strategy for the three slots that must remain distinct in a for-each
/// triple (input, item, output).
fn arb_three_distinct_slots() -> impl Strategy<Value = (u16, u16, u16)> {
    (0u16..16, 0u16..16, 0u16..16).prop_filter(
        "input, item, and output slots must be pairwise distinct",
        |(a, b, c)| a != b && b != c && a != c,
    )
}

/// Builds a 3-node workflow with ForEachStart at step 0, ForEachNext at
/// step 1, and ForEachJoin at step 2, where the body and done targets
/// point to forward steps. `body` must be strictly greater than `done`
/// so the join sits at step 2 (the highest index).
fn build_for_each_workflow(
    input: u16,
    item: u16,
    output: u16,
    limit: u32,
    digest_byte: u8,
) -> WorkflowParts {
    // Layout:
    //   0: ForEachStart { input, item, limit, body=1, done=2 }
    //   1: ForEachNext { iterator=input, body=1, done=2 }
    //   2: ForEachJoin { output }
    WorkflowParts {
        name: Box::<str>::from("for_each_ordering"),
        digest: WorkflowDigest::from_bytes([digest_byte; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(item)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(input),
                    item_slot: SlotIdx::new(item),
                    limit,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(input),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: Some(SlotIdx::new(output)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(output),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 16,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

// =========================================================================
// Properties
// =========================================================================

proptest! {
    /// `ForEachStart.limit` is preserved through a `to_parts` round-trip: the
    /// workflow cannot silently rewrite the bound.
    #[test]
    fn proptest_for_each_start_preserves_limit(
        limit in arb_limit(),
        slots in arb_three_distinct_slots(),
    ) {
        let (input, item, output) = slots;
        let parts = build_for_each_workflow(input, item, output, limit, 0xFE);
        let workflow = CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("for_each workflow must compile: {e}")))?;
        let recovered = workflow.to_parts();
        let recovered_kind = &recovered.nodes[0].kind;
        match recovered_kind {
            CompiledNodeKind::ForEachStart {
                input: ri,
                item_slot: ritem,
                limit: rlimit,
                body: _,
                done: _,
            } => {
                prop_assert_eq!(*ri, SlotIdx::new(input), "input slot must round-trip");
                prop_assert_eq!(*ritem, SlotIdx::new(item), "item_slot must round-trip");
                prop_assert_eq!(*rlimit, limit, "limit must round-trip identically");
            }
            _ => prop_assert!(false, "expected ForEachStart variant after round-trip"),
        }
    }

    /// `ForEachNext` and `ForEachJoin` references remain stable through
    /// round-trip: iterator slot, body step, done step, and output slot
    /// are not silently rewritten by the compiler or workflow constructor.
    #[test]
    fn proptest_for_each_next_join_preserve_references(
        limit in arb_limit(),
        slots in arb_three_distinct_slots(),
    ) {
        let (input, item, output) = slots;
        let parts = build_for_each_workflow(input, item, output, limit, 0xFC);
        let workflow = CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("for_each pair must compile: {e}")))?;
        let recovered = workflow.to_parts();
        match &recovered.nodes[1].kind {
            CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            } => {
                prop_assert_eq!(*iterator_slot, SlotIdx::new(input), "iterator_slot must round-trip");
                prop_assert_eq!(*body, StepIdx::new(1), "body must round-trip");
                prop_assert_eq!(*done, StepIdx::new(2), "done must round-trip");
            }
            _ => prop_assert!(false, "expected ForEachNext variant after round-trip"),
        }
        match &recovered.nodes[2].kind {
            CompiledNodeKind::ForEachJoin { output: o } => {
                prop_assert_eq!(*o, SlotIdx::new(output), "join output slot must round-trip");
            }
            _ => prop_assert!(false, "expected ForEachJoin variant after round-trip"),
        }
    }

    /// For-each ordering: across many randomized limits, the workflow
    /// `node_count()` always equals the number of compiled nodes (3 for a
    /// start/next/join triple) regardless of how many iterations the limit
    /// permits. The iteration count lives in the runtime, not in the IR.
    #[test]
    fn proptest_for_each_node_count_is_independent_of_limit(
        limit in arb_limit(),
        slots in arb_three_distinct_slots(),
    ) {
        let (input, item, output) = slots;
        let parts = build_for_each_workflow(input, item, output, limit, 0xFD);
        let workflow = CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("for_each triple must compile: {e}")))?;
        prop_assert_eq!(
            workflow.node_count(), 3u16,
            "for_each triple must always have exactly 3 nodes (limit is runtime data)"
        );
    }
}

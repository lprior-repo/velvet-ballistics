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

//! Blake3 digest coherence tests (B25-B27).

use crate::ids::{SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

fn make_minimal_workflow_parts(name: &str, entry: StepIdx, slot_count: u16) -> WorkflowParts {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    WorkflowParts {
        name: name.into(),
        digest,
        nodes: Box::new([CompiledNode {
            id: entry,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[test]
fn blake3_digest_is_deterministic_for_identical_parts() {
    let parts1 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts2 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let bytes1 = postcard::to_allocvec(&parts1).expect("serialize should succeed");
    let bytes2 = postcard::to_allocvec(&parts2).expect("serialize should succeed");
    let hash1 = blake3::hash(&bytes1);
    let hash2 = blake3::hash(&bytes2);
    assert_eq!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "identical WorkflowParts must produce identical digests"
    );
}

#[test]
fn blake3_digest_differs_when_name_differs() {
    let parts_alpha = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts_beta = make_minimal_workflow_parts("beta", StepIdx::ZERO, 1);
    let bytes_alpha = postcard::to_allocvec(&parts_alpha).expect("serialize should succeed");
    let bytes_beta = postcard::to_allocvec(&parts_beta).expect("serialize should succeed");
    let hash_alpha = blake3::hash(&bytes_alpha);
    let hash_beta = blake3::hash(&bytes_beta);
    assert_ne!(
        hash_alpha.as_bytes(),
        hash_beta.as_bytes(),
        "different name must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_node_count_differs() {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts1 = WorkflowParts {
        name: "test".into(),
        digest,
        nodes: Box::new([node.clone()]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let node2 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts2 = WorkflowParts {
        name: "test".into(),
        digest,
        nodes: Box::new([node, node2]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash1 = blake3::hash(&postcard::to_allocvec(&parts1).expect("serialize should succeed"));
    let hash2 = blake3::hash(&postcard::to_allocvec(&parts2).expect("serialize should succeed"));
    assert_ne!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "different node_count must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_entry_step_differs() {
    let parts_entry0 = make_minimal_workflow_parts("test", StepIdx::ZERO, 1);
    let parts_entry1 = make_minimal_workflow_parts("test", StepIdx::new(1), 1);
    let hash0 =
        blake3::hash(&postcard::to_allocvec(&parts_entry0).expect("serialize should succeed"));
    let hash1 =
        blake3::hash(&postcard::to_allocvec(&parts_entry1).expect("serialize should succeed"));
    assert_ne!(
        hash0.as_bytes(),
        hash1.as_bytes(),
        "different entry step must produce different digest"
    );
}

#[test]
fn blake3_digest_valid_for_zero_slot_workflow() {
    let parts = make_minimal_workflow_parts("zero_slot", StepIdx::ZERO, 0);
    let bytes = postcard::to_allocvec(&parts).expect("serialize should succeed");
    let hash = blake3::hash(&bytes);
    let hash_bytes = hash.as_bytes();
    assert_eq!(hash_bytes.len(), 32, "blake3 must produce 32-byte hash");
    assert_ne!(hash_bytes, &[0u8; 32], "hash must not be all zeros");
}

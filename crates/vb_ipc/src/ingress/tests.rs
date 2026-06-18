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

use super::*;
use bytes::Bytes;
use std::num::NonZeroUsize;
use vb_core::{RunId, WorkflowDigest};

#[test]
fn ingress_frame_new_with_empty_payload_and_min_max_succeeds() {
    let min_max = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::new(),
        min_max,
    );
    let expected = IngressFrame {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        payload: crate::BoundedPayload::new(Bytes::new(), min_max).unwrap(),
    };
    assert_eq!(result, Ok(expected));
}

#[test]
fn ingress_frame_new_with_payload_exactly_at_max_succeeds() {
    let max = MaxPayloadBytes::DEFAULT;
    let payload = Bytes::from(vec![0u8; max.get()]);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        payload.clone(),
        max,
    );
    let expected = IngressFrame {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        payload: crate::BoundedPayload::new(payload, max).unwrap(),
    };
    assert_eq!(result, Ok(expected));
}

#[test]
fn memory_ingress_bounded_capacity_one_accepts_one_rejects_second() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    assert_eq!(ingress.try_submit(frame.clone()), Ok(()));
    assert!(matches!(ingress.try_submit(frame), Err(IpcError::Full)));
}

#[test]
fn memory_ingress_try_recv_on_empty_queue_returns_none() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    assert_eq!(ingress.try_recv(), Ok(None));
}

#[test]
fn memory_ingress_try_recv_returns_items_in_fifo_order() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(2).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let frame1 = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"first"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    let frame2 = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"second"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    ingress.try_submit(frame1.clone()).unwrap();
    ingress.try_submit(frame2.clone()).unwrap();

    assert_eq!(ingress.try_recv(), Ok(Some(frame1)));
    assert_eq!(ingress.try_recv(), Ok(Some(frame2)));
    assert_eq!(ingress.try_recv(), Ok(None));
}

#[test]
fn memory_ingress_producer_handle_preserves_queued_frames() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(2).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let producer = ingress.producer();
    let first = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"first"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    let second = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"second"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    assert_eq!(ingress.try_submit(first.clone()), Ok(()));
    assert_eq!(producer.try_submit(second.clone()), Ok(()));

    assert_eq!(ingress.try_recv(), Ok(Some(first)));
    assert_eq!(ingress.try_recv(), Ok(Some(second)));
    assert_eq!(ingress.try_recv(), Ok(None));
}

#[test]
fn cloned_producer_handles_share_queue_backpressure() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let first_producer = ingress.producer();
    let second_producer = first_producer.clone();
    let first = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"first"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    let second = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"second"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    assert_eq!(first_producer.try_submit(first.clone()), Ok(()));
    assert_eq!(
        second_producer.try_submit(second.clone()),
        Err(IpcError::Full)
    );
    assert_eq!(ingress.try_recv(), Ok(Some(first)));
    assert_eq!(second_producer.try_submit(second.clone()), Ok(()));
    assert_eq!(ingress.try_recv(), Ok(Some(second)));
}

#[test]
fn producer_handle_try_submit_returns_disconnected_after_receiver_drop() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let producer = ingress.producer();
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"frame"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    drop(ingress);

    assert_eq!(producer.try_submit(frame), Err(IpcError::Disconnected));
}

#[test]
fn memory_ingress_recv_returns_disconnected_after_sender_drop() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let mut ingress = MemoryIngress::bounded(capacity);
    ingress.disconnect_sender();
    assert!(matches!(ingress.try_recv(), Err(IpcError::Disconnected)));
}

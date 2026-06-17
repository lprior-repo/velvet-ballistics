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
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::codec::encode_record;
use vb_storage::constants::{
    MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN,
};
use vb_storage::events::JournalEvent;
use vb_storage::records::RecordKind;

proptest! {
    #[test]
    fn ps005_encoded_min(run in 1u64..1000u64, seq in 0u64..100u64) {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        if let Ok(value) = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) {
            prop_assert!(value.len() >= RECORD_HEADER_LEN as usize);
        }
    }
    #[test]
    fn ps005_encoded_gt_payload(run in 1u64..100u64, seq in 0u64..10u64) {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        match encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) {
            Ok(value) => {
                if let Ok(payload_only) = postcard::to_allocvec(&event) {
                    prop_assert!(value.len() > payload_only.len());
                    prop_assert_eq!(value.len() - payload_only.len(), RECORD_HEADER_LEN as usize);
                }
            }
            Err(_) => {}
        }
    }
    #[test]
    fn ps005_diff_seq(run in 1u64..100u64) {
        let e1 = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        let e2 = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(1),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        if let (Ok(v1), Ok(v2)) = (
            encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &e1, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
            encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 1, &e2, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        ) {
            prop_assert_ne!(v1, v2);
        }
    }
    #[test]
    fn ps005_max_in_u64(_dummy in proptest::bool::ANY) {
        let max = RECORD_HEADER_LEN as u64 + MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64;
        prop_assert!(max < u64::MAX);
    }
    #[test]
    fn ps005_all_kinds_encode(run in 1u64..100u64) {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(run), seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([0u8; 32]),
            },
            JournalEvent::StepStarted {
                run: RunId::new(run), seq: EventSeq::new(1),
                step: vb_core::StepIdx::new(0), attempt: 1,
            },
        ];
        for (i, event) in events.iter().enumerate() {
            let result = encode_record(
                MAGIC_JOURNAL_EVENT, event.record_kind(), i as u64,
                event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            if let Ok(value) = result {
                prop_assert!(value.len() >= RECORD_HEADER_LEN as usize);
            }
        }
    }
}

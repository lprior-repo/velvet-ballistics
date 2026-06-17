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

//! Proptest artifact for `obl-vb-mrwe-5-ps004-proptest-019`.

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::codec::{
    decode_journal_event, encode_record, is_known_record_kind, validate_record_kind_family,
};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::{EventSeq, JournalError, JournalEvent, RecordKind};

proptest! {
    #[test]
    fn vb_mrwe5_kind_family_and_legacy_policy_props(kind in 0_u16..=64_u16) {
        if kind == RecordKind::SlotWritten.id() {
            prop_assert!(is_known_record_kind(kind));
            let kind_result = validate_record_kind_family(MAGIC_JOURNAL_EVENT, kind);
            let ok = kind_result.is_ok();
            prop_assert!(ok,
                "SlotWritten (kind={kind}) must validate as known family");
        }

        if kind == RecordKind::StepSucceeded.id() {
            prop_assert!(is_known_record_kind(kind));
            let kind_result = validate_record_kind_family(MAGIC_JOURNAL_EVENT, kind);
            let ok = kind_result.is_ok();
            prop_assert!(ok,
                "StepSucceeded (kind={kind}) must validate as known family");
        }

        if !is_known_record_kind(kind) {
            let kind_result = validate_record_kind_family(MAGIC_JOURNAL_EVENT, kind);
            let is_err = kind_result.is_err();
            prop_assert!(is_err,
                "unknown kind (kind={kind}) must be rejected by family validation");
        }
    }

    #[test]
    fn vb_mrwe5_mismatch_matrix_fails_closed(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        step in any::<u16>(),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
        legacy_like in any::<bool>(),
    ) {
        let (event, wrong_kind) = if legacy_like {
            (
                JournalEvent::StepSucceeded {
                    run: RunId::new(run),
                    seq: EventSeq::new(seq),
                    step: StepIdx::new(step),
                    output: SlotIdx::new(slot),
                },
                RecordKind::SlotWritten,
            )
        } else {
            (
                JournalEvent::SlotWrittenEvent {
                    run: RunId::new(run),
                    seq: EventSeq::new(seq),
                    slot: SlotIdx::new(slot),
                    value: None,
                    extra: None,
                    attempt,
                },
                RecordKind::StepSucceeded,
            )
        };
        prop_assert_ne!(wrong_kind, event.record_kind());
        let bytes_result = encode_record(
            MAGIC_JOURNAL_EVENT,
            wrong_kind,
            seq,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let bytes = bytes_result.expect(
            "mismatch record encoding must succeed (digest validation is permissive)"
        );
        prop_assert!(matches!(
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
            Err(JournalError::InvalidEvent)
        ));
    }
}

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

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

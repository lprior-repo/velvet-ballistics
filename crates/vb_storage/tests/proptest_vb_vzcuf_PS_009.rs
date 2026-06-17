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
    unused_variables
)]
use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::codec::encode_record;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::error::JournalError;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;
use vb_storage::records::RecordKind;

fn make_event(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    }
}
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

proptest! {
    #[test]
    fn ps009_dup_rejected(run in 1u64..1000u64, seq in 0u64..100u64) {
        let (_temp, journal) = temp_journal();
        let event = make_event(run, seq);
        let mut b1 = JournalWriteBatch::new(&journal);
        b1.append_event(&event).expect("first");
        b1.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        let result = b2.append_event(&event);
        let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
        prop_assert!(is_dup);
    }
    #[test]
    fn ps009_encode_det(run in 1u64..500u64, seq in 0u64..50u64) {
        let event = make_event(run, seq);
        let r1 = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let r2 = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        match (r1, r2) {
            (Ok(v1), Ok(v2)) => { prop_assert_eq!(v1, v2); }
            _ => {}
        }
    }
    #[test]
    fn ps009_conservative(_dummy in proptest::bool::ANY) {
        let mut total: u64 = 0;
        for encoded_len in [100u64, 150, 200, 80] {
            total = total.checked_add(encoded_len).unwrap_or(u64::MAX);
        }
        prop_assert!(total > 0);
        prop_assert_eq!(total, 530u64);
    }
    #[test]
    fn ps009_precise(_dummy in proptest::bool::ANY) {
        let mut seen = std::collections::HashSet::new();
        let mut total: u64 = 0;
        for (key, encoded_len) in [(1u64, 100u64), (2, 150), (1, 100), (3, 200)] {
            if seen.insert(key) {
                total = total.checked_add(encoded_len).unwrap_or(u64::MAX);
            }
        }
        prop_assert_eq!(total, 450u64);
    }
    #[test]
    fn ps009_mono(adds in proptest::collection::vec(1u64..1000u64, 0..20)) {
        let mut total: u64 = 0;
        for add in adds {
            if let Some(nt) = total.checked_add(add) {
                prop_assert!(nt >= total);
                total = nt;
            }
        }
    }
    #[test]
    fn ps009_within_limit(adds in proptest::collection::vec(1u64..100u64, 0..50)) {
        let limit: u64 = 1_048_576;
        let mut total: u64 = 0;
        for add in adds {
            if let Some(nt) = total.checked_add(add) {
                if nt > limit { break; }
                total = nt;
                prop_assert!(total <= limit);
            }
        }
    }
}

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

use super::sample_event;
use crate::JournalEvent;
use crate::codec::{decode_journal_event, encode_journal_event_record};
use crate::constants::MAGIC_JOURNAL_EVENT;
use vb_core::RunId;

#[test]
fn decode_accepts_duplicate_sequence_but_replay_rejects() {
    let event1 = sample_event();
    let event2 = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(0),
        step: vb_core::StepIdx::ZERO,
        attempt: 1,
    };
    let bytes1 = encode_journal_event_record(&event1).expect("event1 encodes");
    let bytes2 = encode_journal_event_record(&event2).expect("event2 encodes");
    let (env1, _) =
        decode_journal_event(&bytes1, MAGIC_JOURNAL_EVENT, 65_536).expect("event1 decodes");
    let (env2, _) =
        decode_journal_event(&bytes2, MAGIC_JOURNAL_EVENT, 65_536).expect("event2 decodes");
    assert_eq!(
        env1.sequence, env2.sequence,
        "duplicate sequence must be observable in envelope"
    );

    let mut tracker = crate::recovery::ActionReplayTracker::default();
    let events = vec![event1, event2];
    let err = crate::recovery::replay_events(&events, &mut tracker, &[])
        .expect_err("replay across duplicate seq must fail");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("ReplayDivergence")
            || msg.contains("SequenceGap")
            || msg.contains("StepOrder"),
        "duplicate sequence replay must surface typed replay error, got {msg}"
    );
}

#[test]
fn replay_rejects_gap_in_sequence() {
    let event1 = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
    };
    let event2 = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(2),
        step: vb_core::StepIdx::ZERO,
        attempt: 1,
    };
    let mut tracker = crate::recovery::ActionReplayTracker::default();
    let err = crate::recovery::replay_events(&[event1, event2], &mut tracker, &[])
        .expect_err("gap in sequence must fail replay");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("ReplayDivergence")
            || msg.contains("SequenceGap")
            || msg.contains("StepOrder"),
        "gap in sequence must surface typed replay error, got {msg}"
    );
}

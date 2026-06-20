#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    clippy::len_zero,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::needless_bool,
    clippy::single_match,
    clippy::single_match_else,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args,
    clippy::large_digit_groups,
    clippy::unreadable_literal,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::wrong_self_convention,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::should_implement_trait,
    clippy::result_large_err,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::comparison_chain,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::explicit_counter_loop,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::needless_update,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    clippy::large_types_passed_by_value,
    clippy::large_futures,
    clippy::type_complexity,
    clippy::needless_collect,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::suspicious_operation_groupings,
    clippy::field_reassign_with_default,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::borrow_deref_ref,
    clippy::cloned_ref_to_slice_refs,
    clippy::inefficient_to_string,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::get_first,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::implicit_saturating_sub,
    clippy::unwrap_or_default,
    clippy::default_trait_access
)]
#![forbid(unsafe_code)]

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod mrwe6_index_contract_tests {
    use crate::{EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits};
    use vb_core::{ActionId, RunId, StepIdx};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    #[test]
    fn action_scheduled_strict_reopens_with_pending_index() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let run = RunId::new(81);
        let action = ActionId::new(12);
        let step = StepIdx::new(2);
        let mut journal = FjallJournal::open(temp.path(), None).expect("journal open");
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal.append_strict(&event).expect("append strict");
        journal.close().expect("close journal");
        drop(journal);

        let reopened = FjallJournal::open(temp.path(), None).expect("reopen journal");
        let key = crate::keys::index_action_key(action, run, step).expect("action key");
        assert!(reopened.has_action_index_entry(key).expect("index read"));
    }

    #[test]
    fn action_completion_removes_pending_index() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(82);
        let action = ActionId::new(13);
        let step = StepIdx::new(3);
        let schedule = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        let complete = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_journaled(&schedule)
            .expect("schedule append");
        journal
            .append_journaled(&complete)
            .expect("completion append");
        let key = crate::keys::index_action_key(action, run, step).expect("action key");
        assert!(!journal.has_action_index_entry(key).expect("index read"));
    }

    #[test]
    fn queued_schedule_flush_writes_pending_index() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(83);
        let action = ActionId::new(14);
        let step = StepIdx::new(4);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        queue.enqueue_journaled(event).expect("enqueue");
        let report = queue.flush_batch(&journal).expect("flush");
        let key = crate::keys::index_action_key(action, run, step).expect("action key");
        assert_eq!(report.written, 1);
        assert!(journal.has_action_index_entry(key).expect("index read"));
    }
}

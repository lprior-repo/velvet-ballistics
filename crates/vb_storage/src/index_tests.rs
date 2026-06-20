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
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod index_tests {
    use crate::{FjallJournal, IndexStatusState};
    use vb_core::{ActionId, RunId, StepIdx, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    #[test]
    fn put_status_index_stores_submitted_marker() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        journal
            .put_status_index(IndexStatusState::Submitted, 1000, run)
            .expect("put_status_index should succeed");

        let mut found = false;
        for item in journal.index_status.iter() {
            let key = item.key().expect("key should be readable");
            assert!(!key.is_empty(), "key should not be empty");
            found = true;
        }
        assert!(found, "status index should contain a marker after put");
    }

    #[test]
    fn put_status_index_stores_active_marker() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2);
        journal
            .put_status_index(IndexStatusState::Active, 2000, run)
            .expect("put_status_index should succeed");

        let mut count = 0usize;
        for item in journal.index_status.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 1, "should have exactly 1 status marker");
    }

    #[test]
    fn put_status_index_stores_completed_marker() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3);
        journal
            .put_status_index(IndexStatusState::Completed, 0, run)
            .expect("put_status_index should succeed");

        let mut count = 0usize;
        for item in journal.index_status.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 1, "should have exactly 1 status marker");
    }

    #[test]
    fn put_status_index_stores_custom_other_marker() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4);
        journal
            .put_status_index(IndexStatusState::Other(42), 9999, run)
            .expect("put_status_index should succeed");

        let mut count = 0usize;
        for item in journal.index_status.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 1, "should have exactly 1 status marker");
    }

    #[test]
    fn put_workflow_index_stores_marker() {
        let (_temp, journal) = temp_journal();
        let wf = WorkflowId::new(5);
        let run = RunId::new(10);
        journal
            .put_workflow_index(wf, run)
            .expect("put_workflow_index should succeed");

        let mut count = 0usize;
        for item in journal.index_workflow.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 1, "should have exactly 1 workflow index marker");
    }

    #[test]
    fn put_workflow_index_multiple_entries() {
        let (_temp, journal) = temp_journal();
        journal
            .put_workflow_index(WorkflowId::new(1), RunId::new(100))
            .expect("put_workflow_index 1");
        journal
            .put_workflow_index(WorkflowId::new(1), RunId::new(101))
            .expect("put_workflow_index 2");
        journal
            .put_workflow_index(WorkflowId::new(2), RunId::new(200))
            .expect("put_workflow_index 3");

        let mut count = 0usize;
        for item in journal.index_workflow.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 3, "should have exactly 3 workflow index markers");
    }

    #[test]
    fn put_action_index_stores_marker() {
        let (_temp, journal) = temp_journal();
        let action = ActionId::new(7);
        let run = RunId::new(20);
        let step = StepIdx::new(0);
        journal
            .put_action_index(action, run, step)
            .expect("put_action_index should succeed");

        let mut count = 0usize;
        for item in journal.index_action.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 1, "should have exactly 1 action index marker");
    }

    #[test]
    fn put_action_index_multiple_entries_for_different_steps() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(30);
        journal
            .put_action_index(ActionId::new(1), run, StepIdx::new(0))
            .expect("put action 1");
        journal
            .put_action_index(ActionId::new(1), run, StepIdx::new(1))
            .expect("put action 1 step 1");
        journal
            .put_action_index(ActionId::new(2), run, StepIdx::new(0))
            .expect("put action 2");

        let mut count = 0usize;
        for item in journal.index_action.iter() {
            let _ = item.key().expect("key ok");
            count = count.saturating_add(1);
        }
        assert_eq!(count, 3, "should have exactly 3 action index markers");
    }

    #[test]
    fn all_three_index_types_can_coexist() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(40);
        journal
            .put_status_index(IndexStatusState::Active, 5000, run)
            .expect("status index");
        journal
            .put_workflow_index(WorkflowId::new(3), run)
            .expect("workflow index");
        journal
            .put_action_index(ActionId::new(99), run, StepIdx::new(5))
            .expect("action index");

        let status_count = journal.index_status.iter().count();
        let wf_count = journal.index_workflow.iter().count();
        let action_count = journal.index_action.iter().count();
        assert_eq!(status_count, 1);
        assert_eq!(wf_count, 1);
        assert_eq!(action_count, 1);
    }

    #[test]
    fn status_index_preserves_empty_value() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(50);
        journal
            .put_status_index(IndexStatusState::Submitted, 7000, run)
            .expect("put should succeed");

        for item in journal.index_status.iter() {
            let val = item.value().expect("value should be readable");
            assert!(val.is_empty(), "status index value should be empty bytes");
        }
    }

    #[test]
    fn workflow_index_preserves_empty_value() {
        let (_temp, journal) = temp_journal();
        journal
            .put_workflow_index(WorkflowId::new(9), RunId::new(60))
            .expect("put should succeed");

        for item in journal.index_workflow.iter() {
            let val = item.value().expect("value should be readable");
            assert!(val.is_empty(), "workflow index value should be empty bytes");
        }
    }

    #[test]
    fn action_index_preserves_empty_value() {
        let (_temp, journal) = temp_journal();
        journal
            .put_action_index(ActionId::new(11), RunId::new(70), StepIdx::new(3))
            .expect("put should succeed");

        for item in journal.index_action.iter() {
            let val = item.value().expect("value should be readable");
            assert!(val.is_empty(), "action index value should be empty bytes");
        }
    }
}

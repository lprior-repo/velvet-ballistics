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

use proptest::prelude::*;

const DETERMINISTIC_REPLAY_CASES: u32 = 64;

fn deterministic_replay_config() -> ProptestConfig {
    ProptestConfig {
        cases: DETERMINISTIC_REPLAY_CASES,
        failure_persistence: None,
        ..Default::default()
    }
}

proptest! {
    #![proptest_config(deterministic_replay_config())]

    #[test]
    fn ppi_001_deterministic_replay_invariant(
        run_val in 1u64..=1000u64,
        step_count in 1u16..=5u16,
        seed_val in 0u8..=99u8,
    ) {
        // PO-010: deterministic replay property registered under the planned
        // `proptests::ppi_001_deterministic_replay_invariant` cargo-test filter.
        use crate::recovery::recover_runtime_summary;
        use crate::{EventSeq, FjallConfig, FjallJournal, JournalEvent};
        use tempfile::TempDir;
        use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([seed_val; 32]);
        let mut events = Vec::new();
        events.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        });

        let mut seq = 1u64;
        for step_idx in 0..step_count {
            events.push(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(seq),
                step: StepIdx::new(step_idx),
                attempt: 1,
            });
            seq = seq.saturating_add(1);
            events.push(JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(seq),
                step: StepIdx::new(step_idx),
                output: SlotIdx::ZERO,
            });
            seq = seq.saturating_add(1);
        }

        let dir1 = match TempDir::new() {
            Ok(d) => d,
            Err(_e) => return Err(TestCaseError::fail("tempdir one creation failed")),
        };
        let journal1 = match FjallJournal::open(dir1.path(), Some(FjallConfig::default())) {
            Ok(j) => j,
            Err(_e) => return Err(TestCaseError::fail("journal one open failed")),
        };
        for event in &events {
            let append = journal1.append_strict(event);
            prop_assert!(matches!(append, Ok(())), "journal one append must succeed with Ok(()), got {append:?}");
        }
        let summary1 = recover_runtime_summary(&journal1, run);

        let dir2 = match TempDir::new() {
            Ok(d) => d,
            Err(_e) => return Err(TestCaseError::fail("tempdir two creation failed")),
        };
        let journal2 = match FjallJournal::open(dir2.path(), Some(FjallConfig::default())) {
            Ok(j) => j,
            Err(_e) => return Err(TestCaseError::fail("journal two open failed")),
        };
        for event in &events {
            let append = journal2.append_strict(event);
            prop_assert!(matches!(append, Ok(())), "journal two append must succeed with Ok(()), got {append:?}");
        }
        let summary2 = recover_runtime_summary(&journal2, run);

        prop_assert_eq!(summary1.is_ok(), summary2.is_ok());
        if let (Ok(h1), Ok(h2)) = (summary1, summary2) {
            let s1 = h1.summary();
            let s2 = h2.summary();
            prop_assert_eq!(s1.run, s2.run);
            prop_assert_eq!(s1.steps_started, s2.steps_started);
            prop_assert_eq!(s1.steps_succeeded, s2.steps_succeeded);
            prop_assert_eq!(s1.terminal, s2.terminal);
            prop_assert_eq!(s1.slots_written, s2.slots_written);
        }
    }
}

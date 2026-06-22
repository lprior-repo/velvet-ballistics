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
mod replay_core_tests {
    use crate::recovery::{
        ActionReplayTracker, RecoveryError,
        replay::{extract_terminal, is_terminal_event, replay_events},
    };
    use crate::{DIGEST_BYTES, EventSeq, JournalEvent};
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

    fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    fn make_run_finished(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result: SlotIdx::new(0),
            attempt: 1,
        }
    }

    fn make_run_cancelled(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
            reason: None,
        }
    }

    fn make_run_failed(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
        }
    }

    #[test]
    fn is_terminal_event_identifies_run_finished() {
        let event = make_run_finished(RunId::new(1), 0);
        assert!(is_terminal_event(&event), "RunFinished should be terminal");
    }

    #[test]
    fn is_terminal_event_identifies_run_cancelled() {
        let event = make_run_cancelled(RunId::new(1), 0);
        assert!(is_terminal_event(&event), "RunCancelled should be terminal");
    }

    #[test]
    fn is_terminal_event_identifies_run_failed_event() {
        let event = make_run_failed(RunId::new(1), 0);
        assert!(
            is_terminal_event(&event),
            "RunFailedEvent should be terminal"
        );
    }

    #[test]
    fn is_terminal_event_rejects_non_terminal_events() {
        let event = make_run_accepted(RunId::new(1), 0);
        assert!(
            !is_terminal_event(&event),
            "RunAccepted should not be terminal"
        );

        let event = make_step_started(RunId::new(1), 0, 0);
        assert!(
            !is_terminal_event(&event),
            "StepStarted should not be terminal"
        );
    }

    #[test]
    fn extract_terminal_returns_terminal_when_present() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 0),
            make_step_started(run, 1, 0),
            make_run_finished(run, 2),
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some(), "should find terminal event");
        assert!(is_terminal_event(terminal.unwrap()));
    }

    #[test]
    fn extract_terminal_returns_none_when_no_terminal() {
        let events = vec![make_step_started(RunId::new(1), 0, 0)];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_none(), "should return None when no terminal");
    }

    #[test]
    fn extract_terminal_returns_last_terminal_with_highest_attempt() {
        let run = RunId::new(2);
        let events = vec![
            make_run_accepted(run, 0),
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
                attempt: 1,
                reason: None,
            },
            JournalEvent::RunRetried {
                run,
                seq: EventSeq::new(2),
                timestamp: chrono::Utc::now(),
            },
            make_step_started(run, 2, 0),
            make_run_finished(run, 3),
        ];
        let terminal = extract_terminal(&events);
        assert!(terminal.is_some(), "should find terminal in latest attempt");
        let found = terminal.unwrap();
        assert!(
            matches!(found, JournalEvent::RunFinished { .. }),
            "should return terminal from latest attempt"
        );
    }

    #[test]
    fn replay_events_detects_non_contiguous_sequences() {
        let run = RunId::new(3);
        let events = vec![
            make_run_accepted(run, 0),
            // Missing seq 1
            make_step_started(run, 2, 0),
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "should reject non-contiguous sequences, got {:?}",
            result
        );
    }

    #[test]
    fn replay_events_detects_step_ordering_violation() {
        let run = RunId::new(4);
        let events = vec![
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(5),
                attempt: 1,
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(3),
                attempt: 1,
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(
            matches!(result, Err(RecoveryError::ReplayDivergence { .. })),
            "should reject step going backward, got {:?}",
            result
        );
    }

    #[test]
    fn replay_events_blocks_duplicate_action_completion() {
        let run = RunId::new(5);
        let action = ActionId::new(10);
        let step = StepIdx::new(0);
        let events = vec![
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(
            matches!(result, Err(RecoveryError::NonIdempotentActionBlocked { action: a, step: s })
                if a == action && s == step),
            "should block duplicate action completion, got {:?}",
            result
        );
    }

    #[test]
    fn replay_events_blocks_action_completed_after_failed() {
        let run = RunId::new(6);
        let action = ActionId::new(11);
        let step = StepIdx::new(0);
        let events = vec![
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            },
        ];
        let mut tracker = ActionReplayTracker::new();
        let result = replay_events(&events, &mut tracker, &[]);
        assert!(
            matches!(
                result,
                Err(RecoveryError::NonIdempotentActionBlocked { .. })
            ),
            "should block action after failure, got {:?}",
            result
        );
    }
}

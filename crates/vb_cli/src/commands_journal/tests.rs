#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
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
    clippy::io_other_error,
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
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
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
    clippy::new_without_default,
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
    clippy::unnecessary_sort_by,
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

use super::{analyze_resume, analyze_retry, event_status, filter_events};
use crate::args::EventStatus;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent};

// ---- analyze_retry: cancellation must not be retried (vb-riz9e) ----

#[test]
fn analyze_retry_cancelled_run_returns_can_retry_false() {
    let run = RunId::new(1);
    let events = vec![JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    }];
    let analysis = analyze_retry(&events);
    assert!(
        !analysis.can_retry,
        "cancelled runs must not be retryable, got reason: {}",
        analysis.reason
    );
    assert!(
        analysis.reason.contains("cancelled"),
        "reason should mention cancelled: {}",
        analysis.reason
    );
    assert!(
        analysis.failed_at_step.is_none(),
        "cancelled runs must not report a failed step"
    );
}

#[test]
fn analyze_retry_failed_run_returns_can_retry_true() {
    let run = RunId::new(2);
    let events = vec![JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
    }];
    let analysis = analyze_retry(&events);
    assert!(
        analysis.can_retry,
        "failed runs must be retryable, got reason: {}",
        analysis.reason
    );
}

// ---- analyze_resume: no suspension event must not be resumable (vb-ujho9) ----

#[test]
fn analyze_resume_no_suspension_event_returns_can_resume_false() {
    let events: Vec<JournalEvent> = vec![];
    let analysis = analyze_resume(&events);
    assert!(
        !analysis.can_resume,
        "runs with no suspension event must not be resumable, got reason: {}",
        analysis.reason
    );
    assert!(analysis.suspended_at_step.is_none());
    assert!(
        analysis.reason.contains("not suspended"),
        "reason should explain lack of suspension: {}",
        analysis.reason
    );
}

#[test]
fn analyze_resume_with_suspension_event_returns_can_resume_true() {
    let run = RunId::new(1);
    let events = vec![JournalEvent::WaitScheduledEvent {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
        deadline_ms: 1000,
    }];
    let analysis = analyze_resume(&events);
    assert!(
        analysis.can_resume,
        "suspended runs must be resumable, got reason: {}",
        analysis.reason
    );
    assert_eq!(analysis.suspended_at_step, Some(0));
}

// ---- event_status: canonical status mapping for the events --status filter (vb-qwsyi) ----

#[test]
fn event_status_run_finished_maps_to_completed() {
    let run = RunId::new(1);
    let event = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(0),
        result: SlotIdx::new(0),
        attempt: 1,
    };
    assert_eq!(event_status(&event), Some(super::TraceStatus::Completed));
}

#[test]
fn event_status_run_failed_event_maps_to_failed() {
    let run = RunId::new(1);
    let event = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
    };
    assert_eq!(event_status(&event), Some(super::TraceStatus::Failed));
}

#[test]
fn event_status_action_failed_event_maps_to_failed() {
    let run = RunId::new(1);
    let event = JournalEvent::ActionFailedEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        action: vb_core::ActionId::new(0),
        attempt: 1,
    };
    assert_eq!(event_status(&event), Some(super::TraceStatus::Failed));
}

#[test]
fn event_status_run_cancelled_maps_to_cancelled() {
    let run = RunId::new(1);
    let event = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    assert_eq!(event_status(&event), Some(super::TraceStatus::Cancelled));
}

#[test]
fn event_status_run_killed_maps_to_cancelled() {
    let run = RunId::new(1);
    let event = JournalEvent::RunKilled {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    assert_eq!(event_status(&event), Some(super::TraceStatus::Cancelled));
}

#[test]
fn event_status_ask_scheduled_maps_to_waiting_answer() {
    let run = RunId::new(1);
    let event = JournalEvent::AskScheduledEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        attempt: 1,
        deadline_ms: 1000,
    };
    assert_eq!(
        event_status(&event),
        Some(super::TraceStatus::WaitingAnswer)
    );
}

#[test]
fn event_status_run_accepted_maps_to_pending() {
    let run = RunId::new(1);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
    };
    assert_eq!(event_status(&event), Some(super::TraceStatus::Pending));
}

// ---- filter_events: events --status / --limit logic (vb-qwsyi) ----

fn synth_run() -> RunId {
    RunId::new(42)
}

fn synth_events() -> Vec<JournalEvent> {
    let run = synth_run();
    // 2 pending (RunAccepted, RunAdmission)
    // 2 active (StepStarted, WaitScheduled)
    // 3 completed (StepSucceeded, ActionCompleted, RunFinished)
    // 1 failed (ActionFailed)
    // 1 waiting_answer (AskScheduled)
    // 1 cancelled (RunCancelled)
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(0),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(0),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(0),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(2),
            attempt: 1,
            deadline_ms: 1000,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(3),
            attempt: 1,
            deadline_ms: 1000,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(9),
            attempt: 1,
            reason: None,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(10),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ]
}

#[test]
fn filter_events_no_filters_returns_all() {
    let events = synth_events();
    let original_len = events.len();
    let filtered = filter_events(events, None, None);
    assert_eq!(
        filtered.len(),
        original_len,
        "no filters must preserve the full event list"
    );
}

#[test]
fn filter_events_status_failed_returns_only_failed() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Failed), None);
    assert_eq!(filtered.len(), 1, "only the ActionFailedEvent is failed");
    let first = filtered.first();
    assert!(
        matches!(first, Some(JournalEvent::ActionFailedEvent { .. })),
        "first filtered event must be ActionFailedEvent, got {first:?}"
    );
}

#[test]
fn filter_events_status_completed_returns_only_completed() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Completed), None);
    assert_eq!(
        filtered.len(),
        3,
        "StepSucceeded + ActionCompleted + RunFinished are completed"
    );
    for event in &filtered {
        let kind_ok = matches!(
            event,
            JournalEvent::StepSucceeded { .. }
                | JournalEvent::ActionCompletedEvent { .. }
                | JournalEvent::ActionCompletedEnvelope { .. }
                | JournalEvent::SlotWrittenEvent { .. }
                | JournalEvent::AskAnsweredEvent { .. }
                | JournalEvent::RunFinished { .. }
                | JournalEvent::RunAnswered { .. }
        );
        assert!(
            kind_ok,
            "non-completed event leaked into completed filter: {event:?}"
        );
    }
}

#[test]
fn filter_events_status_cancelled_returns_only_cancelled() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Cancelled), None);
    assert_eq!(filtered.len(), 1, "only RunCancelled is cancelled");
}

#[test]
fn filter_events_status_pending_returns_only_pending() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Pending), None);
    assert_eq!(filtered.len(), 2, "RunAccepted + RunAdmission are pending");
}

#[test]
fn filter_events_status_active_returns_only_active() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Active), None);
    assert_eq!(
        filtered.len(),
        3,
        "StepStarted + ActionScheduled + WaitScheduled are active"
    );
}

#[test]
fn filter_events_status_waiting_answer_returns_only_ask() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::WaitingAnswer), None);
    assert_eq!(
        filtered.len(),
        1,
        "only AskScheduledEvent is waiting_answer"
    );
}

#[test]
fn filter_events_status_with_no_matches_returns_empty() {
    let events = vec![JournalEvent::RunFinished {
        run: synth_run(),
        seq: EventSeq::new(0),
        result: SlotIdx::new(0),
        attempt: 1,
    }];
    let filtered = filter_events(events, Some(EventStatus::Failed), None);
    assert!(
        filtered.is_empty(),
        "filtering a completed run for failed events must be empty"
    );
}

#[test]
fn filter_events_limit_truncates_after_status_filter() {
    // synth_events has 3 completed events. Limit to 2 should keep first 2.
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Completed), Some(2));
    assert_eq!(filtered.len(), 2, "limit must truncate AFTER status filter");
    // Original order preserved: StepSucceeded first, ActionCompleted second.
    assert!(matches!(filtered[0], JournalEvent::StepSucceeded { .. }));
    assert!(matches!(
        filtered[1],
        JournalEvent::ActionCompletedEvent { .. }
    ));
}

#[test]
fn filter_events_limit_zero_returns_empty() {
    let events = synth_events();
    let filtered = filter_events(events, None, Some(0));
    assert!(filtered.is_empty(), "limit=0 must yield an empty result");
}

#[test]
fn filter_events_negative_limit_returns_empty() {
    // Negative limits are user-input hazards. They must NOT silently expand
    // to usize::MAX. Treated as 0 to fail closed.
    let events = synth_events();
    let filtered = filter_events(events, None, Some(-1));
    assert!(
        filtered.is_empty(),
        "negative limit must be rejected as zero, not silently expand"
    );
}

#[test]
fn filter_events_limit_larger_than_filtered_returns_all_filtered() {
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Failed), Some(10));
    assert_eq!(
        filtered.len(),
        1,
        "limit >= matched count returns matched count"
    );
}

#[test]
fn filter_events_preserves_journal_order() {
    let events = synth_events();
    let original_seqs: Vec<u64> = events.iter().filter_map(seq_of).collect();
    let filtered = filter_events(events, None, None);
    let filtered_seqs: Vec<u64> = filtered.iter().filter_map(seq_of).collect();
    assert_eq!(
        original_seqs, filtered_seqs,
        "journal order must be preserved"
    );
}

fn seq_of(event: &JournalEvent) -> Option<u64> {
    match event {
        JournalEvent::RunAccepted { seq, .. }
        | JournalEvent::RunAdmission { seq, .. }
        | JournalEvent::StepStarted { seq, .. }
        | JournalEvent::StepSucceeded { seq, .. }
        | JournalEvent::ActionScheduled { seq, .. }
        | JournalEvent::ActionCompletedEvent { seq, .. }
        | JournalEvent::ActionScheduledTicket { seq, .. }
        | JournalEvent::ActionCompletedEnvelope { seq, .. }
        | JournalEvent::ActionFailedEvent { seq, .. }
        | JournalEvent::SlotWrittenEvent { seq, .. }
        | JournalEvent::WaitScheduledEvent { seq, .. }
        | JournalEvent::AskScheduledEvent { seq, .. }
        | JournalEvent::AskAnsweredEvent { seq, .. }
        | JournalEvent::RetryScheduledEvent { seq, .. }
        | JournalEvent::RunCancelled { seq, .. }
        | JournalEvent::RunKilled { seq, .. }
        | JournalEvent::RunFinished { seq, .. }
        | JournalEvent::RunFailedEvent { seq, .. } => Some(seq.get()),
        _ => None,
    }
}

#[test]
fn filter_events_status_and_limit_combined() {
    // Real bug scenario: --status failed --limit 10 on a journal with
    // mixed events must return at most 10 failed events, in order.
    let events = synth_events();
    let filtered = filter_events(events, Some(EventStatus::Failed), Some(10));
    assert_eq!(filtered.len(), 1, "synth has 1 failed event");
}

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

use crate::DurableActionOutcome;
use crate::EventSeq;
use crate::JournalEvent;
use crate::recovery::replay::summary::*;
use crate::recovery::{
    RecoveredRunAdmission, RecoveredSlotEntry, RecoveredStepState, RecoveryError,
    RecoveryRuntimeSummary, RecoveryTerminalState,
};
use vb_core::SlotValue;
use vb_core::action::compute_action_idempotency_key;
use vb_core::replay::{ReplayError, SuspensionKind};
use vb_core::{
    ActionId, ActionTicket, CapabilitySet, FiniteF64, ListId, MockMarker, ObjectId, RunId,
    RuntimePolicy, SeqNo, SlotIdx, StepIdx, Taint, WorkflowDigest,
};

fn fresh_summary() -> RecoveryRuntimeSummary {
    RecoveryRuntimeSummary {
        run: RunId::new(1),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(0),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    }
}

fn assert_counters(
    summary: &RecoveryRuntimeSummary,
    steps_started: u64,
    steps_succeeded: u64,
    actions_scheduled: u64,
    actions_resolved: u64,
    suspensions: u64,
    slots_written: u64,
) {
    assert_eq!(summary.steps_started, steps_started, "steps_started");
    assert_eq!(summary.steps_succeeded, steps_succeeded, "steps_succeeded");
    assert_eq!(
        summary.actions_scheduled, actions_scheduled,
        "actions_scheduled"
    );
    assert_eq!(
        summary.actions_resolved, actions_resolved,
        "actions_resolved"
    );
    assert_eq!(summary.suspensions, suspensions, "suspensions");
    assert_eq!(summary.slots_written, slots_written, "slots_written");
}

#[test]
fn ask_answered_event_is_no_op() {
    let mut summary = fresh_summary();
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    apply_summary_event(&mut summary, &event);
    assert_counters(&summary, 0, 0, 0, 0, 0, 0);
}

#[test]
fn action_failed_event_increments_actions_resolved_only() {
    let mut summary = fresh_summary();
    let event = JournalEvent::ActionFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(0),
        attempt: 1,
    };
    apply_summary_event(&mut summary, &event);
    assert_counters(&summary, 0, 0, 0, 1, 0, 0);
}

#[test]
fn slot_written_event_increments_slots_written_only() {
    let mut summary = fresh_summary();
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    apply_summary_event(&mut summary, &event);
    assert_counters(&summary, 0, 0, 0, 0, 0, 1);
}

#[test]
fn wait_scheduled_event_increments_suspensions() {
    let mut summary = fresh_summary();
    let event = JournalEvent::WaitScheduledEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
        deadline_ms: 30000,
    };
    apply_summary_event(&mut summary, &event);
    assert_counters(&summary, 0, 0, 0, 0, 1, 0);
}

#[test]
fn summary_events_cover_workflow_admission_retry_and_terminals() {
    let mut summary = fresh_summary();
    let run = RunId::new(1);
    let workflow = digest(7);
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(1),
            workflow,
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(2),
            artifact_digest: digest(8),
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            output: SlotIdx::new(2),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(8),
            result: SlotIdx::new(2),
            attempt: 1,
        },
    ];

    events
        .iter()
        .for_each(|event| apply_summary_event(&mut summary, event));

    assert_eq!(summary.workflow, Some(workflow));
    assert_counters(&summary, 1, 1, 1, 1, 1, 0);
    assert_eq!(
        summary.terminal,
        Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(2),
        })
    );

    apply_summary_event(
        &mut summary,
        &JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(9),
            attempt: 1,
            reason: None,
        },
    );
    assert_eq!(summary.terminal, Some(RecoveryTerminalState::Cancelled));

    apply_summary_event(
        &mut summary,
        &JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(10),
            attempt: 1,
        },
    );
    assert_eq!(summary.terminal, Some(RecoveryTerminalState::Failed));
}

#[test]
fn recover_run_admission_returns_latest_metadata_or_none() {
    let run = RunId::new(12);
    let first = digest(1);
    let latest = digest(2);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest(9),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: first,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Relaxed,
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(2),
            artifact_digest: latest,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Journaled,
        },
    ];

    let recovered = recover_run_admission_from_events(&events);
    assert!(matches!(
        recovered,
        Some(RecoveredRunAdmission {
            artifact_digest,
            run_id,
            policy: RuntimePolicy::Journaled,
            ..
        }) if artifact_digest == latest && run_id == run
    ));
    assert_eq!(recover_run_admission_from_events(&events[0..1]), None);
}

#[test]
fn replayed_object_slots_are_explicitly_unsupported() {
    let slots = recovered_single_slot(SlotValue::Object(ObjectId::new(7)));

    assert_eq!(slots, RecoveredSlots::unsupported());
}

#[test]
fn replayed_list_slots_are_explicitly_unsupported() {
    let slots = recovered_single_slot(SlotValue::List(ListId::new(8)));

    assert_eq!(slots, RecoveredSlots::unsupported());
}

#[test]
fn replayed_scalar_slots_remain_supported() {
    let slots = recovered_single_slot(SlotValue::I64(7));

    assert!(slots.fully_supported);
    assert_eq!(slots.entries.len(), 1);
}

#[test]
fn replayed_all_scalar_slot_variants_remain_supported() {
    [
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::F64(finite_f64(1.25)),
        SlotValue::Symbol(vb_core::SymbolId::new(4)),
    ]
    .into_iter()
    .for_each(|value| {
        let slots = recovered_single_slot(value);
        assert!(slots.fully_supported, "scalar slot must be supported");
        assert_eq!(slots.entries.len(), 1);
    });
}

#[test]
fn summarize_recovery_events_empty_returns_exact_no_recovery_data() {
    let result = summarize_recovery_events(&[]);

    assert!(matches!(
        result,
        Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0)
    ));
}

#[test]
fn frame_seed_empty_events_returns_exact_no_recovery_data() {
    let result = recover_runtime_frame_seed_from_events(&[]);

    assert!(matches!(
        result,
        Err(RecoveryError::NoRecoveryData { run }) if run == RunId::new(0)
    ));
}

#[test]
fn frame_seed_builder_without_workflow_delegates_to_event_recovery() {
    let events = vec![JournalEvent::StepStarted {
        run: RunId::new(21),
        seq: EventSeq::new(0),
        step: StepIdx::new(5),
        attempt: 1,
    }];

    let direct = recover_runtime_frame_seed_from_events(&events);
    let built = RecoveryFrameSeedBuilder::new().build(&events);

    assert!(matches!((direct, built), (Ok(a), Ok(b)) if a == b));
}

#[test]
fn workflow_digest_rejection_reports_exact_mismatch_and_accepts_match() {
    let expected = digest(11);
    let found = digest(12);
    let events = [JournalEvent::RunAccepted {
        run: RunId::new(31),
        seq: EventSeq::new(0),
        workflow: found,
    }];

    assert!(matches!(
        reject_workflow_digest_mismatch(&events, expected),
        Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
            if e == expected && f == found
    ));
    assert_eq!(
        reject_workflow_digest_mismatch(&events, found).ok(),
        Some(())
    );
}

#[test]
fn workflow_digest_rejection_fails_closed_without_run_accepted() {
    let expected = digest(11);
    let run = RunId::new(31);
    let events = [
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
    ];

    assert!(matches!(
        reject_workflow_digest_mismatch(&events, expected),
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO && detail.contains("RunAccepted evidence missing")
    ));
    assert!(matches!(
        reject_workflow_digest_mismatch(&[], expected),
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO && detail.contains("RunAccepted evidence missing")
    ));
}

#[test]
fn frame_seed_slot_dimension_overflow_reports_exact_variant() {
    let run = RunId::new(41);
    let events = [JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        output: SlotIdx::MAX,
    }];

    // vb-xb38b: StepSucceeded.output now contributes to the slot dimension
    // (previously it was silently dropped), so SlotIdx::MAX produces the
    // typed FrameDimensionOverflow error rather than overflowing u16.
    let result = recover_runtime_frame_seed_from_events(&events);
    assert!(
        matches!(result, Err(RecoveryError::FrameDimensionOverflow { run: found }) if found == run),
        "Expected FrameDimensionOverflow for SlotIdx::MAX output, got {result:?}"
    );
}

#[test]
fn event_slot_values_cover_valid_corrupt_and_missing_frame_paths() {
    let run = RunId::new(51);
    let valid_bytes = encoded_slot_value(SlotValue::Bool(true));
    let events = vec![
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: SlotIdx::new(0),
            value: Some(valid_bytes),
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(1),
            slot: SlotIdx::new(1),
            value: Some(vec![255, 0, 255]),
            extra: None,
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(2),
            value: None,
            extra: None,
            attempt: 1,
        },
    ];

    let seed = recover_runtime_frame_seed_from_events(&events);

    let recovered = seed.expect("should recover successfully");
    assert!(
        recovered
            .slots
            .iter()
            .any(|entry| entry.slot == SlotIdx::new(0)
                && entry.value == SlotValue::Bool(true)
                && entry.taint == Taint::Secret),
        "Expected slot 0 with Bool(true) and Secret taint (SR-013: legacy fallback fails closed)"
    );
    assert!(
        recovered.unsupported.slot_values,
        "Expected slot_values unsupported"
    );
}

#[test]
fn unresolved_action_recovers_as_supported_pending_action_seed() {
    let run = RunId::new(61);
    let events = [JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(3),
        action: ActionId::new(9),
        attempt: 1,
    }];

    let seed = recover_runtime_frame_seed_from_events(&events);

    // snapshot writing checks unsupported traits directly
    assert!(matches!(seed, Ok(recovered) if !recovered.unsupported.slot_values));
}

#[test]
fn replay_step_not_found_maps_to_exact_recovery_error() {
    assert_replay_divergence(
        ReplayError::StepNotFound {
            step: StepIdx::new(9),
        },
        StepIdx::new(9),
        "replay step not found in compiled workflow",
    );
}

#[test]
fn replay_non_deterministic_maps_to_exact_recovery_error() {
    assert_replay_divergence(
        ReplayError::NonDeterministicStep {
            step: StepIdx::new(4),
            kind: SuspensionKind::AskPending,
        },
        StepIdx::new(4),
        "replay blocked by non-deterministic Ask step",
    );
}

#[test]
fn replay_non_deterministic_mapping_uses_all_typed_kind_names() {
    let cases = [
        (
            SuspensionKind::ActionPending,
            "replay blocked by non-deterministic Do step",
        ),
        (
            SuspensionKind::AskPending,
            "replay blocked by non-deterministic Ask step",
        ),
        (
            SuspensionKind::WaitUntil,
            "replay blocked by non-deterministic WaitUntil step",
        ),
        (
            SuspensionKind::WaitEvent,
            "replay blocked by non-deterministic WaitEvent step",
        ),
    ];

    cases.into_iter().for_each(|(kind, detail)| {
        assert_replay_divergence(
            ReplayError::NonDeterministicStep {
                step: StepIdx::new(4),
                kind,
            },
            StepIdx::new(4),
            detail,
        );
    });
}

#[test]
fn replay_slot_not_available_maps_to_exact_recovery_error() {
    assert_replay_divergence(
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(3),
        },
        StepIdx::ZERO,
        "replay required unavailable slot SlotIdx(3)",
    );
}

#[test]
fn replay_expression_error_maps_to_exact_recovery_error() {
    assert_replay_divergence(
        ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(6),
        },
        StepIdx::new(6),
        "replay expression evaluation failed",
    );
}

#[test]
fn replay_internal_error_maps_to_exact_recovery_error() {
    assert_replay_divergence(
        ReplayError::Internal {
            reason: "arena handle recovery unsupported",
        },
        StepIdx::ZERO,
        "arena handle recovery unsupported",
    );
}

#[test]
fn recovery_summary_multiple_runs_reports_exact_divergence_detail() {
    let events = two_run_events();

    assert!(matches!(
        summarize_recovery_events(&events),
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "recovery summary received events for multiple runs"
    ));
}

#[test]
fn frame_seed_multiple_runs_reports_exact_divergence_detail() {
    let events = two_run_events();

    assert!(matches!(
        recover_runtime_frame_seed_from_events(&events),
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "frame seed recovery received events for multiple runs"
    ));
}

fn recovered_single_slot(value: SlotValue) -> RecoveredSlots {
    RecoveredSlots::from_replayed(vec![RecoveredSlotEntry {
        slot: SlotIdx::new(0),
        value,
        taint: Taint::Secret,
    }])
}

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn encoded_slot_value(value: SlotValue) -> Vec<u8> {
    match postcard::to_allocvec(&value) {
        Ok(bytes) => bytes,
        Err(error) => panic!("slot value encoding failed: {error}"),
    }
}

fn finite_f64(value: f64) -> FiniteF64 {
    match FiniteF64::new(value) {
        Ok(finite) => finite,
        Err(error) => panic!("finite test value rejected: {error}"),
    }
}

fn two_run_events() -> [JournalEvent; 2] {
    [
        JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: RunId::new(2),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ]
}

fn assert_replay_divergence(error: ReplayError, step: StepIdx, detail: &str) {
    assert!(matches!(
        replay_error_to_recovery(error),
        RecoveryError::ReplayDivergence { step: s, detail: d }
            if s == step && d == detail
    ));
}

// ══ pending_actions_from_events unit tests (vb-av1y0 / P0-5b2) ════════════

/// B1: Happy path — 5 scheduled, 3 completed → 2 pending.
#[test]
fn pending_actions_from_events_returns_collected_actions() {
    let run = RunId::new(100);
    let events: Vec<JournalEvent> = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(3),
            action: ActionId::new(4),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(4),
            action: ActionId::new(5),
            attempt: 1,
        },
    ];

    let result = pending_actions_from_events(&events);

    assert_eq!(result.len(), 2);
    let set: std::collections::HashSet<_> = result.into_iter().collect();
    assert!(set.contains(&crate::recovery::RecoveredPendingAction {
        step: StepIdx::new(3),
        action: ActionId::new(4),
    }));
    assert!(set.contains(&crate::recovery::RecoveredPendingAction {
        step: StepIdx::new(4),
        action: ActionId::new(5),
    }));
}

/// B2: Empty input → empty output.
#[test]
fn pending_actions_from_events_empty_input() {
    let events: Vec<JournalEvent> = vec![];
    let result = pending_actions_from_events(&events);
    assert!(result.is_empty());
}

/// B4: Only terminal events → empty output (no scheduled actions).
#[test]
fn pending_actions_from_events_only_terminal_events() {
    let run = RunId::new(200);
    let events: Vec<JournalEvent> = vec![
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: SlotIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
            reason: None,
        },
    ];

    let result = pending_actions_from_events(&events);
    assert!(result.is_empty());
}

/// B4: Orphan completed (no matching scheduled) → empty, no panic.
#[test]
fn pending_actions_from_events_orphan_completed_event() {
    let run = RunId::new(201);
    let events = vec![JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        action: ActionId::new(99),
        attempt: 1,
    }];

    let result = pending_actions_from_events(&events);
    assert!(result.is_empty());
}

/// B5: All scheduled, no completed → all pending.
#[test]
fn pending_actions_from_events_all_scheduled_no_completed() {
    let run = RunId::new(202);
    let events: Vec<JournalEvent> = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        },
    ];

    let result = pending_actions_from_events(&events);
    assert_eq!(result.len(), 3);
}

/// B6: One scheduled, one completed → empty.
#[test]
fn pending_actions_from_events_all_completed_no_pending() {
    let run = RunId::new(203);
    let events = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
    ];

    let result = pending_actions_from_events(&events);
    assert!(result.is_empty());
}

/// B7: Precondition — empty slice returns empty.
#[test]
fn pending_actions_from_events_empty_slice_precondition() {
    let events: Vec<JournalEvent> = vec![];
    let result = pending_actions_from_events(&events);
    assert!(result.is_empty());
}

/// B8: Contract — return length = scheduled - completed.
#[test]
fn pending_actions_from_events_length_equals_scheduled_minus_completed() {
    // Property: for any sequence of ActionScheduled/ActionCompleted events,
    // the result length equals the set-difference count.
    let events: Vec<JournalEvent> = vec![
        JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
    ];

    let result = pending_actions_from_events(&events);
    // 2 scheduled - 1 completed = 1 pending
    assert_eq!(result.len(), 1);
}

/// B9: Invariant — pure function, deterministic output.
#[test]
fn pending_actions_from_events_is_pure_deterministic() {
    let run = RunId::new(300);
    let events: Vec<JournalEvent> = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
    ];

    let r1 = pending_actions_from_events(&events);
    let r2 = pending_actions_from_events(&events);

    // Both calls produce the same elements (same HashSet, same iteration)
    let s1: std::collections::HashSet<_> = r1.into_iter().collect();
    let s2: std::collections::HashSet<_> = r2.into_iter().collect();
    assert_eq!(s1, s2, "function must be deterministic (pure)");
}

/// B10: Ticket variants — ActionScheduledTicket and ActionCompletedEnvelope.
#[test]
fn pending_actions_from_events_handles_ticket_variants() {
    let run = RunId::new(400);

    let ticket1 = ActionTicket {
        run,
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(10),
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, SeqNo::new(0), ActionId::new(10)),
        capacity: 3,
        mock: MockMarker::default(),
    };
    let ticket2 = ActionTicket {
        run,
        step: StepIdx::new(1),
        seq: SeqNo::new(1),
        action: ActionId::new(11),
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, SeqNo::new(1), ActionId::new(11)),
        capacity: 3,
        mock: MockMarker::default(),
    };

    let events: Vec<JournalEvent> = vec![
        JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(0),
            ticket: ticket1.clone(),
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
        },
        JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(1),
            ticket: ticket2.clone(),
            input: SlotIdx::new(1),
            output: SlotIdx::new(2),
        },
        JournalEvent::ActionCompletedEnvelope {
            run,
            seq: EventSeq::new(2),
            ticket: ticket1,
            output: SlotIdx::new(1),
            outcome: DurableActionOutcome::Ready,
            value: vec![],
            encoded_len: 0,
            taint: Taint::Clean,
            value_digest: [0u8; 32],
        },
    ];

    let result = pending_actions_from_events(&events);
    assert_eq!(result.len(), 1);
    let set: std::collections::HashSet<_> = result.into_iter().collect();
    assert!(set.contains(&crate::recovery::RecoveredPendingAction {
        step: StepIdx::new(1),
        action: ActionId::new(11),
    }));
}

/// SR-007: `ActionFailedEvent` must remove the action from the pending set,
/// matching the accumulator semantics in `record_action_failed`. A failed
/// action should never appear in the resume list because it has been
/// resolved and re-execution would trigger non-idempotent side effects.
#[test]
fn pending_actions_from_events_removes_failed_action() {
    let run = RunId::new(500);
    let events: Vec<JournalEvent> = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            action: ActionId::new(2),
            attempt: 1,
        },
    ];

    let result = pending_actions_from_events(&events);
    assert_eq!(result.len(), 2);
    let set: std::collections::HashSet<_> = result.into_iter().collect();
    assert!(!set.contains(&crate::recovery::RecoveredPendingAction {
        step: StepIdx::new(1),
        action: ActionId::new(2),
    }));
    assert!(set.contains(&crate::recovery::RecoveredPendingAction {
        step: StepIdx::new(0),
        action: ActionId::new(1),
    }));
    assert!(set.contains(&crate::recovery::RecoveredPendingAction {
        step: StepIdx::new(2),
        action: ActionId::new(3),
    }));
}

/// SR-007: Orphan `ActionFailedEvent` (no matching scheduled) is a no-op.
#[test]
fn pending_actions_from_events_orphan_failed_event_is_noop() {
    let run = RunId::new(501);
    let events = vec![JournalEvent::ActionFailedEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(7),
        action: ActionId::new(99),
        attempt: 1,
    }];

    let result = pending_actions_from_events(&events);
    assert!(result.is_empty());
}

// ══ legacy_slot_taint unit tests (SR-013 / P0) ════════════════════════════

fn legacy_slot_taint(value: SlotValue) -> Taint {
    crate::recovery::replay::summary::slots::taint::recovered_slot_taint(
        SlotIdx::new(0),
        value,
        None,
    )
    .expect("legacy taint never errors")
    .taint
}

fn legacy_frame_extra_slot_taint(value: SlotValue) -> Taint {
    let extra = crate::events::SlotWriteExtra::Legacy(vec![0xAB, 0xCD, 0xEF, 0x42]);
    crate::recovery::replay::summary::slots::taint::recovered_slot_taint(
        SlotIdx::new(0),
        value,
        Some(&extra),
    )
    .expect("legacy frame extra taint never errors")
    .taint
}

/// qi37-1.1 red recovery contract: legacy fallback without `SlotWriteExtra`
/// classifies the value to reflect how much secret information it could
/// leak. Bool(false) → Clean (false predicates do not leak secrets),
/// Bool(true) and Null → DerivedFromSecret (positive / absence predicates
/// can derive from secrets), I64/F64 → Secret (they carry the data itself).
#[test]
fn legacy_slot_taint_classifies_bool_false_as_clean() {
    assert_eq!(legacy_slot_taint(SlotValue::Bool(false)), Taint::Clean);
}

/// qi37-1.1 red recovery contract: legacy fallback taint is value-typed.
/// Bool(false) is the only Clean case; Bool(true) and Null are
/// DerivedFromSecret; I64/F64 carry the data and are Secret.
#[test]
fn legacy_slot_taint_classifies_values_by_type() {
    assert_eq!(
        legacy_slot_taint(SlotValue::Bool(false)),
        Taint::Clean,
        "Bool(false) must be Clean"
    );
    assert_eq!(
        legacy_slot_taint(SlotValue::Bool(true)),
        Taint::DerivedFromSecret,
        "Bool(true) must be DerivedFromSecret"
    );
    assert_eq!(
        legacy_slot_taint(SlotValue::Null),
        Taint::DerivedFromSecret,
        "Null must be DerivedFromSecret"
    );
    assert_eq!(
        legacy_slot_taint(SlotValue::I64(0)),
        Taint::Secret,
        "I64 must be Secret"
    );
    assert_eq!(
        legacy_slot_taint(SlotValue::I64(42)),
        Taint::Secret,
        "I64 must be Secret"
    );
    assert_eq!(
        legacy_slot_taint(SlotValue::F64(vb_core::FiniteF64::new(0.0).expect("finite"))),
        Taint::Secret,
        "F64 must be Secret"
    );
}

/// vb-7ol6y: legacy frame-extra payloads (no versioned envelope prefix)
/// are not taint metadata — legacy runtime used this slot for collect
/// pagination state and other non-taint payloads. They must classify as
/// `Taint::Clean`, not `Taint::Secret`.
#[test]
fn legacy_frame_extra_slot_taint_classifies_as_clean() {
    assert_eq!(
        legacy_frame_extra_slot_taint(SlotValue::Bool(false)),
        Taint::Clean
    );
    assert_eq!(
        legacy_frame_extra_slot_taint(SlotValue::I64(7)),
        Taint::Clean
    );
}

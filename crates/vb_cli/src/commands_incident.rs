#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Incident report computation for CLI output.
//!
//! Delegates domain analysis to vb_storage::journal::incident.
//! This CLI module builds the CLI-specific IncidentReport with JSON values.

use vb_storage::events::JournalEvent;
use vb_storage::journal::incident::{
    IncidentCheckpoint, IncidentEventCounts, SideEffectCertainty, SideEffectDisposition,
    SideEffectEvidence, analyze_incident_events, build_repair_hints,
};

/// Structured incident report for CLI output.
pub struct IncidentReport {
    /// The run id string (as provided by the caller).
    pub run_id: String,
    /// Failure code, e.g. "RunFailed" or "RunCancelled". Empty if no failure.
    pub failure_code: String,
    /// Whether a failure event was found.
    pub failure_found: bool,
    /// Step at which the failure occurred, if known.
    pub failed_at_step: Option<u16>,
    /// Last journal sequence observed while building the incident report.
    pub last_sequence: Option<u64>,
    /// Last durable checkpoint-like journal event observed.
    pub last_checkpoint: serde_json::Value,
    /// Per-kind event counts from the incident journal slice.
    pub event_counts: serde_json::Value,
    /// Side effects collected from action completed/failed events.
    pub side_effects: Vec<serde_json::Value>,
    /// Durable action side-effect evidence, including scheduled/resolved state.
    pub side_effect_evidence: Vec<serde_json::Value>,
    /// Durable failed-action evidence.
    pub failed_action_evidence: Vec<serde_json::Value>,
    /// Actions durably scheduled but not resolved by the incident tail.
    pub pending_scheduled_actions: Vec<serde_json::Value>,
    /// Repair hints based on failure type.
    pub repair_hints: Vec<serde_json::Value>,
}

/// Build an incident report from a run's event stream.
pub fn build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport {
    let analysis = analyze_incident_events(events);
    let hints = build_repair_hints(
        &analysis.failure_code,
        &analysis.side_effects,
        analysis.failed_at_step,
    );

    IncidentReport {
        run_id: run_id.to_string(),
        failure_code: analysis.failure_code,
        failure_found: analysis.failure_found,
        failed_at_step: analysis.failed_at_step,
        last_sequence: analysis.last_sequence.map(|seq| seq.get()),
        last_checkpoint: checkpoint_json(analysis.last_checkpoint),
        event_counts: counts_json(&analysis.counts),
        side_effects: analysis
            .side_effects
            .into_iter()
            .map(side_effect_json)
            .collect(),
        side_effect_evidence: analysis
            .side_effect_evidence
            .into_iter()
            .map(side_effect_evidence_json)
            .collect(),
        failed_action_evidence: analysis
            .failed_action_evidence
            .into_iter()
            .map(side_effect_evidence_json)
            .collect(),
        pending_scheduled_actions: analysis
            .pending_scheduled_actions
            .into_iter()
            .map(side_effect_evidence_json)
            .collect(),
        repair_hints: hints.into_iter().map(serde_json::Value::String).collect(),
    }
}

fn side_effect_json(se: vb_storage::journal::incident::SideEffect) -> serde_json::Value {
    serde_json::json!({
        "step": se.step,
        "action": se.action,
        "certainty": match se.certainty {
            SideEffectCertainty::Confirmed => "confirmed",
            SideEffectCertainty::Failed => "failed",
        }
    })
}

fn side_effect_evidence_json(evidence: SideEffectEvidence) -> serde_json::Value {
    serde_json::json!({
        "seq": evidence.seq.get(),
        "step": evidence.step,
        "action": evidence.action,
        "attempt": evidence.attempt,
        "disposition": disposition_name(evidence.disposition)
    })
}

fn disposition_name(disposition: SideEffectDisposition) -> &'static str {
    match disposition {
        SideEffectDisposition::Scheduled => "scheduled",
        SideEffectDisposition::Completed => "completed",
        SideEffectDisposition::Failed => "failed",
    }
}

fn checkpoint_json(checkpoint: Option<IncidentCheckpoint>) -> serde_json::Value {
    match checkpoint {
        Some(value) => serde_json::json!({
            "available": true,
            "seq": value.seq.get(),
            "kind": format!("{:?}", value.kind),
            "kind_id": value.kind.id(),
            "step": value.step,
            "action": value.action,
            "slot": value.slot,
            "attempt": value.attempt
        }),
        None => serde_json::json!({"available": false}),
    }
}

fn counts_json(counts: &IncidentEventCounts) -> serde_json::Value {
    serde_json::json!({
        "total": counts.total,
        "run_accepted": counts.run_accepted,
        "run_admission": counts.run_admission,
        "steps_started": counts.steps_started,
        "steps_succeeded": counts.steps_succeeded,
        "actions_scheduled": counts.actions_scheduled,
        "actions_completed": counts.actions_completed,
        "actions_failed": counts.actions_failed,
        "slot_writes": counts.slot_writes,
        "waits_scheduled": counts.waits_scheduled,
        "asks_scheduled": counts.asks_scheduled,
        "asks_answered": counts.asks_answered,
        "retries_scheduled": counts.retries_scheduled,
        "run_cancelled": counts.run_cancelled,
        "run_killed": counts.run_killed,
        "run_finished": counts.run_finished,
        "run_failed": counts.run_failed,
        "run_resumed": counts.run_resumed,
        "run_retried": counts.run_retried,
        "run_answered": counts.run_answered
    })
}

#[cfg(test)]
mod tests {
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

    use super::*;
    use vb_core::{ActionId, RunId, StepIdx};
    use vb_storage::{EventSeq, JournalEvent};

    /// Helper: create a minimal StepStarted event.
    fn step_event(step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    /// Helper: create a minimal ActionCompletedEvent.
    fn action_completed(step: u16, action: u16) -> JournalEvent {
        JournalEvent::ActionCompletedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    /// Helper: create a minimal ActionFailedEvent.
    fn action_failed(step: u16, action: u16) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    /// Helper: create a minimal ActionScheduled event at a specific sequence.
    fn action_scheduled_at(seq: u64, step: u16, action: u16) -> JournalEvent {
        JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    /// Helper: create a RunFailedEvent.
    fn run_failed() -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
        }
    }

    /// Helper: create a RunCancelled event.
    fn run_cancelled() -> JournalEvent {
        JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(10),
            attempt: 1,
            reason: None,
        }
    }

    // ---- T-001: Empty events ----
    #[test]
    fn t_001_empty_events() {
        let report = build_incident_report("run-1", &[]);
        assert!(!report.failure_found);
        assert_eq!(report.failure_code, "");
        assert!(report.failed_at_step.is_none());
        assert!(report.side_effects.is_empty());
    }

    // ---- T-002: RunFailedEvent ----
    #[test]
    fn t_002_run_failed_event() {
        let events = vec![step_event(1), run_failed()];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunFailed");
        assert_eq!(report.failed_at_step, Some(1));
    }

    // ---- T-003: RunCancelled ----
    #[test]
    fn t_003_run_cancelled() {
        let events = vec![step_event(1), step_event(2), run_cancelled()];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunCancelled");
        assert_eq!(report.failed_at_step, Some(2));
    }

    // ---- T-004: ActionCompletedEvent side effects ----
    #[test]
    fn t_004_action_completed_side_effects() {
        let events = vec![action_completed(1, 100)];
        let report = build_incident_report("run-1", &events);
        assert!(!report.failure_found);
        assert_eq!(report.side_effects.len(), 1);
        assert_eq!(report.side_effects[0]["step"], 1);
        assert_eq!(report.side_effects[0]["action"], 100);
        assert_eq!(report.side_effects[0]["certainty"], "confirmed");
    }

    // ---- T-005: ActionFailedEvent side effects ----
    #[test]
    fn t_005_action_failed_side_effects() {
        let events = vec![action_failed(2, 200)];
        let report = build_incident_report("run-1", &events);
        assert!(!report.failure_found);
        assert_eq!(report.side_effects.len(), 1);
        assert_eq!(report.side_effects[0]["certainty"], "failed");
    }

    // ---- T-006: Multiple events ----
    #[test]
    fn t_006_multiple_events() {
        let events = vec![
            step_event(1),
            action_completed(1, 10),
            action_failed(1, 20),
            step_event(2),
            action_completed(2, 30),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunFailed");
        assert_eq!(report.failed_at_step, Some(2));
        assert_eq!(report.side_effects.len(), 3);
    }

    // ---- T-007: Multiple StepStarted tracking ----
    #[test]
    fn t_007_multiple_step_started_tracking() {
        let events = vec![
            step_event(1),
            step_event(3),
            step_event(5),
            step_event(7),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        // failed_at_step should be the last step_started (7)
        assert_eq!(report.failed_at_step, Some(7));
    }

    // ---- T-008: Mixed events full report ----
    #[test]
    fn t_008_mixed_events_full_report() {
        let events = vec![
            step_event(1),
            action_completed(1, 10),
            step_event(2),
            action_failed(2, 20),
            step_event(3),
            action_completed(3, 30),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);
        assert!(report.failure_found);
        assert_eq!(report.failure_code, "RunFailed");
        assert_eq!(report.failed_at_step, Some(3));
        assert_eq!(report.side_effects.len(), 3);
        assert!(!report.repair_hints.is_empty());
    }

    #[test]
    fn t_009_durable_incident_fields_are_projected() {
        let events = vec![
            step_event(4),
            action_scheduled_at(2, 4, 70),
            action_failed(4, 70),
            run_failed(),
        ];
        let report = build_incident_report("run-1", &events);

        assert_eq!(report.last_sequence, Some(10));
        assert_eq!(report.last_checkpoint["kind"], "RunFailed");
        assert_eq!(report.last_checkpoint["kind_id"], 23);
        assert_eq!(report.event_counts["total"], 4);
        assert_eq!(report.event_counts["actions_scheduled"], 1);
        assert_eq!(report.event_counts["actions_failed"], 1);
        assert_eq!(report.side_effect_evidence.len(), 2);
        assert_eq!(report.side_effect_evidence[0]["disposition"], "scheduled");
        assert_eq!(report.failed_action_evidence.len(), 1);
        assert_eq!(report.failed_action_evidence[0]["disposition"], "failed");
        assert!(report.pending_scheduled_actions.is_empty());
    }

    // Later build_repair_hints-only tests removed: build_repair_hints logic is now tested
    // in vb_storage::journal::incident (domain tests). CLI tests cover the
    // full build_incident_report pipeline which includes repair hints.
}

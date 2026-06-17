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
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Incident analysis and lifecycle state derivation for workflow runs.
//!
//! Domain logic for analyzing journal events.

use crate::events::JournalEvent;
#[allow(unused_imports)]
use vb_core::{ActionId, RunId, StepIdx, workflow::LifecycleState};

/// Side effect recorded from an action event.
#[derive(Debug, Clone)]
pub struct SideEffect {
    pub step: u16,
    pub action: u16,
    pub certainty: SideEffectCertainty,
}

/// Whether an action succeeded or failed.
#[derive(Debug, Clone)]
pub enum SideEffectCertainty {
    Confirmed,
    Failed,
}

/// Incident analysis result from scanning journal events.
#[derive(Debug, Clone)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_code: String,
    pub failed_at_step: Option<u16>,
    pub side_effects: Vec<SideEffect>,
}

/// Build incident analysis from a run's event stream.
pub fn analyze_incident_events(events: &[JournalEvent]) -> IncidentAnalysis {
    let mut failure_found = false;
    let mut failure_code = String::new();
    let mut failed_at_step: Option<u16> = None;
    let mut side_effects: Vec<SideEffect> = Vec::new();
    let mut last_step_started: Option<u16> = None;

    for event in events {
        match event {
            JournalEvent::StepStarted { step, .. } => {
                last_step_started = Some(step.get());
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Confirmed,
                });
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                side_effects.push(SideEffect {
                    step: step.get(),
                    action: action.get(),
                    certainty: SideEffectCertainty::Failed,
                });
            }
            JournalEvent::RunFailedEvent { .. } => {
                failure_found = true;
                failure_code = "RunFailed".to_string();
                failed_at_step = last_step_started;
            }
            JournalEvent::RunCancelled { .. } => {
                failure_found = true;
                failure_code = "RunCancelled".to_string();
                failed_at_step = last_step_started;
            }
            _ => {}
        }
    }

    IncidentAnalysis {
        failure_found,
        failure_code,
        failed_at_step,
        side_effects,
    }
}

/// Build repair hints based on the failure code, side effects, and failed step.
pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[SideEffect],
    failed_at_step: Option<u16>,
) -> Vec<String> {
    let mut hints: Vec<String> = Vec::new();

    match failure_code {
        "RunFailed" => {
            hints.push("investigate step output and engine logs for the failed step".to_string());
            if !side_effects.is_empty() {
                hints.push(
                    "review side effects that completed before failure for compensating actions"
                        .to_string(),
                );
            }
            if let Some(step) = failed_at_step {
                hints.push(format!(
                    "consider retry from step {step} using the retry command"
                ));
            }
        }
        "RunCancelled" => {
            hints.push("run was cancelled; check if cancellation was intentional".to_string());
            if !side_effects.is_empty() {
                hints.push("review completed side effects for partial cleanup needs".to_string());
            }
        }
        _ => {}
    }

    hints
}

/// Maps a lifecycle state to a human-readable status string for the inspect command.
///
/// Terminal states map to their name; Active/WaitingAnswer map to "running".
#[must_use]
pub fn lifecycle_state_to_inspect_status(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Cancelled => "cancelled",
        LifecycleState::Completed => "finished",
        LifecycleState::Failed => "failed",
        LifecycleState::Pending | LifecycleState::Active | LifecycleState::WaitingAnswer => {
            "running"
        }
        _ => "running",
    }
}

/// Derives the final lifecycle state from a sequence of journal events.
///
/// The last event in the sequence determines the final state:
/// - `RunCancelled` → Cancelled
/// - `RunResumed` → Active
/// - `RunRetried` → Active
/// - `RunAnswered` → Completed
/// - `RunFinished` → Completed
/// - `RunFailedEvent` → Failed
///
/// If no events exist, defaults to Pending.
#[allow(unreachable_patterns)]
pub fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState {
    events
        .last()
        .map(|e| match e {
            JournalEvent::RunCancelled { .. } => LifecycleState::Cancelled,
            JournalEvent::RunResumed { .. } => LifecycleState::Active,
            JournalEvent::RunRetried { .. } => LifecycleState::Active,
            JournalEvent::RunAnswered { .. } => LifecycleState::Completed,
            JournalEvent::RunFinished { .. } => LifecycleState::Completed,
            JournalEvent::RunFailedEvent { .. } => LifecycleState::Failed,
            JournalEvent::RunAccepted { .. } => LifecycleState::Active,
            JournalEvent::RunAdmission { .. } => LifecycleState::Active,
            JournalEvent::StepStarted { .. } => LifecycleState::Active,
            JournalEvent::StepSucceeded { .. } => LifecycleState::Active,
            JournalEvent::ActionScheduled { .. } => LifecycleState::Active,
            JournalEvent::SlotWrittenEvent { .. } => LifecycleState::Active,
            JournalEvent::ActionCompletedEvent { .. } => LifecycleState::Active,
            JournalEvent::ActionFailedEvent { .. } => LifecycleState::Failed,
            JournalEvent::WaitScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::AskScheduledEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::AskAnsweredEvent { .. } => LifecycleState::WaitingAnswer,
            JournalEvent::RetryScheduledEvent { .. } => LifecycleState::Active,
            _ => LifecycleState::Active,
        })
        .unwrap_or(LifecycleState::Pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSeq;
    use crate::JournalEvent;

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
        let analysis = analyze_incident_events(&[]);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.failure_code, "");
        assert!(analysis.failed_at_step.is_none());
        assert!(analysis.side_effects.is_empty());
    }

    // ---- T-002: RunFailedEvent ----
    #[test]
    fn t_002_run_failed_event() {
        let events = vec![step_event(1), run_failed()];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunFailed");
        assert_eq!(analysis.failed_at_step, Some(1));
    }

    // ---- T-003: RunCancelled ----
    #[test]
    fn t_003_run_cancelled() {
        let events = vec![step_event(1), step_event(2), run_cancelled()];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunCancelled");
        assert_eq!(analysis.failed_at_step, Some(2));
    }

    // ---- T-004: ActionCompletedEvent side effects ----
    #[test]
    fn t_004_action_completed_side_effects() {
        let events = vec![action_completed(1, 100)];
        let analysis = analyze_incident_events(&events);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.side_effects.len(), 1);
        assert_eq!(analysis.side_effects[0].step, 1);
        assert_eq!(analysis.side_effects[0].action, 100);
        assert!(matches!(
            analysis.side_effects[0].certainty,
            SideEffectCertainty::Confirmed
        ));
    }

    // ---- T-005: ActionFailedEvent side effects ----
    #[test]
    fn t_005_action_failed_side_effects() {
        let events = vec![action_failed(2, 200)];
        let analysis = analyze_incident_events(&events);
        assert!(!analysis.failure_found);
        assert_eq!(analysis.side_effects.len(), 1);
        assert!(matches!(
            analysis.side_effects[0].certainty,
            SideEffectCertainty::Failed
        ));
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
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunFailed");
        assert_eq!(analysis.failed_at_step, Some(2));
        assert_eq!(analysis.side_effects.len(), 3);
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
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failed_at_step, Some(7));
    }

    // ---- T-008: Mixed events ----
    #[test]
    fn t_008_mixed_events() {
        let events = vec![
            step_event(1),
            action_completed(1, 10),
            step_event(2),
            action_failed(2, 20),
            step_event(3),
            action_completed(3, 30),
            run_failed(),
        ];
        let analysis = analyze_incident_events(&events);
        assert!(analysis.failure_found);
        assert_eq!(analysis.failure_code, "RunFailed");
        assert_eq!(analysis.failed_at_step, Some(3));
        assert_eq!(analysis.side_effects.len(), 3);
    }

    // ---- T-009: RunFailed repair hints (1 hint) ----
    #[test]
    fn t_009_run_failed_1_hint() {
        let hints = build_repair_hints("RunFailed", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0],
            "investigate step output and engine logs for the failed step"
        );
    }

    // ---- T-010: RunFailed repair hints (3 hints) ----
    #[test]
    fn t_010_run_failed_3_hints() {
        let side_effects = vec![SideEffect {
            step: 1,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("RunFailed", &side_effects, Some(3));
        assert_eq!(hints.len(), 3);
        assert_eq!(
            hints[0],
            "investigate step output and engine logs for the failed step"
        );
        assert_eq!(
            hints[1],
            "review side effects that completed before failure for compensating actions"
        );
        assert_eq!(
            hints[2],
            "consider retry from step 3 using the retry command"
        );
    }

    // ---- T-011: RunCancelled repair hints (1 hint) ----
    #[test]
    fn t_011_run_cancelled_1_hint() {
        let hints = build_repair_hints("RunCancelled", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0],
            "run was cancelled; check if cancellation was intentional"
        );
    }

    // ---- T-012: RunCancelled repair hints (2 hints) ----
    #[test]
    fn t_012_run_cancelled_2_hints() {
        let side_effects = vec![SideEffect {
            step: 2,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("RunCancelled", &side_effects, None);
        assert_eq!(hints.len(), 2);
        assert_eq!(
            hints[0],
            "run was cancelled; check if cancellation was intentional"
        );
        assert_eq!(
            hints[1],
            "review completed side effects for partial cleanup needs"
        );
    }

    // ---- T-013: Unknown failure code (0 hints) ----
    #[test]
    fn t_013_unknown_failure_code() {
        let hints = build_repair_hints("UnknownError", &[], None);
        assert!(hints.is_empty());
    }
}

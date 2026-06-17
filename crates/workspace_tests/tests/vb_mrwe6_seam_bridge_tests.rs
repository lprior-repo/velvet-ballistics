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
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::manual_contains,
    clippy::if_same_then_else,
    clippy::multiple_bound_locations,
    clippy::identity_op,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
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

use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6DuplicateRetryDecision, Mrwe6EventClass,
    Mrwe6IntentKind, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision, Mrwe6SeamError,
    mrwe6_action_index_intent, mrwe6_committed_resolution_from_facts,
    mrwe6_duplicate_retry_decision, mrwe6_duplicate_retry_decision_from_facts, mrwe6_event_class,
    mrwe6_event_intent_matches_class, mrwe6_idempotent_duplicate_retry_from_facts,
    mrwe6_pending_inventory_from_facts, mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
    mrwe6_required_intent_kind_for_class, mrwe6_resolution_commit_decision,
    mrwe6_resolution_commit_decision_from_facts, mrwe6_valid_queued_relevant_intent,
    mrwe6_valid_scheduled_atom, mrwe6_validated_atom, mrwe6_validated_atom_for_event,
};
use vb_storage::{EventSeq, JournalEvent};

fn run() -> RunId {
    RunId::new(7)
}

fn step() -> StepIdx {
    StepIdx::new(3)
}

fn action() -> ActionId {
    ActionId::new(11)
}

fn scheduled(seq: u64) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run: run(),
        seq: EventSeq::new(seq),
        step: step(),
        action: action(),
        attempt: 1,
    }
}

fn completed(seq: u64, action_id: ActionId) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run: run(),
        seq: EventSeq::new(seq),
        step: step(),
        action: action_id,
        attempt: 1,
    }
}

#[test]
fn vb_mrwe6_bridge_scheduled_event_maps_to_put_pending_intent() {
    let event = scheduled(1);

    assert!(matches!(
        mrwe6_event_class(&event),
        Mrwe6EventClass::Scheduled
    ));
    assert!(matches!(
        mrwe6_action_index_intent(&event),
        Mrwe6ActionIndexIntent::Put { action: a, run: r, step: s }
            if a == action() && r == run() && s == step()
    ));
    assert!(matches!(
        mrwe6_required_intent_kind_for_class(mrwe6_event_class(&event)),
        Mrwe6IntentKind::PutPending
    ));
    assert!(mrwe6_event_intent_matches_class(&event));
}

#[test]
fn vb_mrwe6_bridge_resolution_event_maps_to_remove_pending_intent() {
    let event = completed(2, action());

    assert!(matches!(
        mrwe6_event_class(&event),
        Mrwe6EventClass::Resolution
    ));
    assert!(matches!(
        mrwe6_action_index_intent(&event),
        Mrwe6ActionIndexIntent::Delete { action: a, run: r, step: s }
            if a == action() && r == run() && s == step()
    ));
    assert!(matches!(
        mrwe6_required_intent_kind_for_class(mrwe6_event_class(&event)),
        Mrwe6IntentKind::RemovePending
    ));
    assert!(mrwe6_event_intent_matches_class(&event));
}

#[test]
fn vb_mrwe6_bridge_duplicate_classifier_separates_equal_from_divergent() {
    let existing = scheduled(3);
    let equal_retry = scheduled(3);
    let divergent_retry = completed(3, action());

    assert!(matches!(
        mrwe6_duplicate_retry_decision(&existing, &equal_retry, true),
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
    ));
    assert!(matches!(
        mrwe6_duplicate_retry_decision(&existing, &equal_retry, false),
        Mrwe6DuplicateRetryDecision::MissingExpectedIndexState
    ));
    assert!(matches!(
        mrwe6_duplicate_retry_decision(&existing, &divergent_retry, true),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    ));
}

#[test]
fn vb_mrwe6_bridge_duplicate_classifier_rejects_equal_resolution_retry_for_marker_states() {
    let resolution = completed(13, action());

    assert_eq!(
        mrwe6_duplicate_retry_decision(&resolution, &resolution, false),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision(&resolution, &resolution, true),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
}

#[test]
fn vb_mrwe6_bridge_duplicate_classifier_rejects_equal_unrelated_retry_for_marker_states() {
    let unrelated = JournalEvent::RunKilled {
        run: run(),
        seq: EventSeq::new(14),
        attempt: 1,
    };

    assert_eq!(
        mrwe6_duplicate_retry_decision(&unrelated, &unrelated, false),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision(&unrelated, &unrelated, true),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
}

#[test]
fn vb_mrwe6_bridge_completion_classifier_removes_only_same_key_on_success() {
    let resolution = completed(4, action());
    let other_action = ActionId::new(12);

    assert!(matches!(
        mrwe6_resolution_commit_decision(&resolution, action(), run(), step(), true),
        Ok(Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved)
    ));
    assert!(matches!(
        mrwe6_resolution_commit_decision(&resolution, action(), run(), step(), false),
        Ok(Mrwe6ResolutionCommitDecision::CommitFailedMarkerRetained)
    ));
    assert!(matches!(
        mrwe6_resolution_commit_decision(&resolution, other_action, run(), step(), true),
        Ok(Mrwe6ResolutionCommitDecision::MismatchedResolutionRejected)
    ));
}

#[test]
fn vb_mrwe6_bridge_recovery_classifier_separates_inventory_defect_and_fallback() {
    let schedule = scheduled(5);
    let resolution = completed(6, action());
    let mismatched_resolution = completed(7, ActionId::new(12));

    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, None, true, false),
        Ok(Mrwe6RecoveryOutcome::PendingInventory)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, Some(&resolution), true, false),
        Ok(Mrwe6RecoveryOutcome::ResolvedNoPending)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, Some(&mismatched_resolution), true, false),
        Ok(Mrwe6RecoveryOutcome::ParityDefect)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, None, false, true),
        Ok(Mrwe6RecoveryOutcome::LegacyFallback)
    ));
    assert!(matches!(
        mrwe6_recovery_outcome(&schedule, None, false, false),
        Ok(Mrwe6RecoveryOutcome::ParityDefect)
    ));
}

#[test]
fn vb_mrwe6_primitive_atom_constructor_rejects_invalid_state() {
    let valid_schedule =
        mrwe6_validated_atom(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending);
    let invalid_schedule = mrwe6_validated_atom(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::None);

    assert!(matches!(
        valid_schedule.map(|atom| atom.atom_kind()),
        Ok(Mrwe6AtomKind::EventAndPutPending)
    ));
    assert!(matches!(
        invalid_schedule,
        Err(Mrwe6SeamError::ClassIntentMismatch)
    ));
    assert!(matches!(
        mrwe6_validated_atom_for_event(&completed(8, action())).map(|atom| atom.atom_kind()),
        Ok(Mrwe6AtomKind::EventAndRemovePending)
    ));
}

#[test]
fn vb_mrwe6_primitive_decision_functions_match_event_wrappers() {
    let event = scheduled(9);
    let completion = completed(10, action());

    assert!(matches!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Scheduled, true),
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
    ));
    assert!(matches!(
        mrwe6_resolution_commit_decision_from_facts(true, true, true),
        Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved
    ));
    assert!(matches!(
        mrwe6_recovery_outcome_from_facts(true, false, false, true, false),
        Mrwe6RecoveryOutcome::PendingInventory
    ));
    assert_eq!(
        mrwe6_duplicate_retry_decision(&event, &event, true),
        mrwe6_duplicate_retry_decision_from_facts(true, mrwe6_event_class(&event), true)
    );
    assert!(matches!(
        mrwe6_resolution_commit_decision(&completion, action(), run(), step(), true),
        Ok(decision)
            if decision == mrwe6_resolution_commit_decision_from_facts(true, true, true)
    ));
}

#[test]
fn vb_mrwe6_invalid_scheduled_atom_is_rejected_with_diagnostic() {
    assert!(matches!(
        mrwe6_valid_scheduled_atom(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::None),
        Err(Mrwe6SeamError::ClassIntentMismatch)
    ));
    assert!(matches!(
        mrwe6_valid_scheduled_atom(Mrwe6EventClass::Unrelated, Mrwe6IntentKind::None),
        Err(Mrwe6SeamError::ScheduledAtomMissingPutPending)
    ));
}

#[test]
fn vb_mrwe6_invalid_queued_relevant_intent_is_rejected_with_diagnostic() {
    assert!(matches!(
        mrwe6_valid_queued_relevant_intent(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::PutPending),
        Ok(_)
    ));
    assert!(matches!(
        mrwe6_valid_queued_relevant_intent(Mrwe6EventClass::Unrelated, Mrwe6IntentKind::None),
        Err(Mrwe6SeamError::QueuedRelevantEventMissingIntent)
    ));
}

#[test]
fn vb_mrwe6_invalid_duplicate_success_is_rejected_with_diagnostic() {
    assert!(matches!(
        mrwe6_idempotent_duplicate_retry_from_facts(false, Mrwe6EventClass::Scheduled, true),
        Err(Mrwe6SeamError::DuplicateRetryNotIdempotent)
    ));
}

#[test]
fn vb_mrwe6_invalid_resolution_success_is_rejected_with_diagnostic() {
    assert!(matches!(
        mrwe6_committed_resolution_from_facts(true, true, false),
        Err(Mrwe6SeamError::ResolutionDidNotRemovePending)
    ));
}

#[test]
fn vb_mrwe6_invalid_recovery_inventory_is_rejected_with_diagnostic() {
    assert!(matches!(
        mrwe6_pending_inventory_from_facts(true, false, false, false, false),
        Err(Mrwe6SeamError::RecoveryOutcomeNotPendingInventory)
    ));
}

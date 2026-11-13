use crate::execution_lanes::GroupError;

#[test]
fn ordered_reverse() -> Result<(), GroupError> {
    ordered::ordered_reverse()
}

#[test]
fn foreach_reuse() -> Result<(), GroupError> {
    ordered::foreach_reuse()
}

#[test]
fn null_wins_after_failure() -> Result<(), GroupError> {
    null::null_wins_after_failure()
}

#[test]
fn winner_requires_drain() -> Result<(), GroupError> {
    lifecycle::winner_requires_drain()
}

#[test]
fn wrong_receipts_are_rejected_and_group_is_unchanged() -> Result<(), GroupError> {
    receipts::wrong_receipts_are_rejected_and_group_is_unchanged()
}

#[test]
fn dropping_prepared_events_is_atomic() -> Result<(), GroupError> {
    lifecycle::dropping_prepared_events_is_atomic()
}

#[test]
fn all_branches_fail_and_terminal_failure_in_ordered_group() -> Result<(), GroupError> {
    lifecycle::all_branches_fail_and_terminal_failure_in_ordered_group()
}

#[test]
fn abort_preserves_winner_and_blocks_partial_completion_in_ordered_group() -> Result<(), GroupError>
{
    lifecycle::abort_preserves_winner_and_blocks_partial_completion_in_ordered_group()
}

#[test]
fn events_for_wrong_generation_are_rejected() -> Result<(), GroupError> {
    bounds::events_for_wrong_generation_are_rejected()
}

#[test]
fn constructor_errors_and_revision_overflow() -> Result<(), GroupError> {
    bounds::constructor_errors_and_revision_overflow()
}

#[test]
fn admission_event_identity_and_stale_receipt() -> Result<(), GroupError> {
    events::admission_event_identity_and_stale_receipt()
}

#[path = "group_test_ordered.rs"]
mod ordered;

#[path = "group_test_support.rs"]
mod support;
#[path = "group_test_winner.rs"]
mod winner;

#[path = "group_test_lifecycle.rs"]
mod lifecycle;

#[path = "group_test_receipt_dimensions.rs"]
mod receipt_dimensions;

#[path = "group_test_abort_receipts.rs"]
mod abort_receipts;

#[path = "group_test_null.rs"]
mod null;

#[path = "group_test_events.rs"]
mod events;

#[path = "group_test_bounds.rs"]
mod bounds;

#[path = "group_test_receipts.rs"]
mod receipts;

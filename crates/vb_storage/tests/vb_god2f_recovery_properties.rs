#![forbid(unsafe_code)]

//! HVR-PO-STORAGE-003: generated executable recovery properties.

use proptest::prelude::*;
use proptest::strategy::Strategy;
use vb_core::{ActionId, ActionTicket, MockMarker, RunId, SeqNo, SlotIdx, StepIdx, Taint};
use vb_storage::recovery::{
    ActionReplayTracker, DigestCheck, RecoveryError, UnsupportedRecoveryState,
};

fn digest_level_strategy() -> impl Strategy<Value = DigestCheck> {
    prop_oneof![
        Just(DigestCheck::WorkflowSourceOnly),
        Just(DigestCheck::WorkflowAndIr),
        Just(DigestCheck::Full),
    ]
}

fn taint_strategy() -> impl Strategy<Value = Taint> {
    prop_oneof![
        Just(Taint::Clean),
        Just(Taint::DerivedFromSecret),
        Just(Taint::Secret)
    ]
}

fn ticket_strategy() -> impl Strategy<Value = ActionTicket> {
    (
        1u64..=64,
        0u16..=16,
        0u64..=128,
        0u16..=16,
        1u16..=16,
        any::<u128>(),
    )
        .prop_map(|(run, step, seq, action, attempt, key)| ActionTicket {
            run: RunId::new(run),
            step: StepIdx::new(step),
            seq: SeqNo::new(seq),
            action: ActionId::new(action),
            attempt,
            idempotency_key: key,
            capacity: u16::MAX,
            mock: MockMarker::HttpGet,
        })
}

fn expected_rank(level: DigestCheck) -> u8 {
    match level {
        DigestCheck::WorkflowSourceOnly => 1,
        DigestCheck::WorkflowAndIr => 2,
        DigestCheck::Full => 3,
        _ => 0,
    }
}

fn alternate_slot(slot: SlotIdx) -> SlotIdx {
    match slot.get().checked_add(1) {
        Some(value) => SlotIdx::new(value),
        None => SlotIdx::ZERO,
    }
}

proptest! {
    #[test]
    fn vb_god2f_recovery_digest_and_unsupported_state_properties(
        left in digest_level_strategy(),
        right in digest_level_strategy(),
        a_slot_values in any::<bool>(),
        a_slot_taint in any::<bool>(),
        a_action_payloads in any::<bool>(),
        b_slot_values in any::<bool>(),
        b_slot_taint in any::<bool>(),
        b_action_payloads in any::<bool>(),
    ) {
        prop_assert_eq!(left.hierarchy_rank(), expected_rank(left));
        prop_assert_eq!(left.checks_workflow_source(), expected_rank(left) >= expected_rank(DigestCheck::WorkflowSourceOnly));
        prop_assert_eq!(left.checks_compiled_ir(), expected_rank(left) >= expected_rank(DigestCheck::WorkflowAndIr));
        prop_assert_eq!(left.checks_full(), expected_rank(left) >= expected_rank(DigestCheck::Full));
        prop_assert_eq!(left.is_strictly_weaker_than(right), expected_rank(left) < expected_rank(right));

        let a = UnsupportedRecoveryState {
            slot_values: a_slot_values,
            slot_taint: a_slot_taint,
            action_payloads: a_action_payloads,
        };
        let b = UnsupportedRecoveryState {
            slot_values: b_slot_values,
            slot_taint: b_slot_taint,
            action_payloads: b_action_payloads,
        };
        let union = a.union(b);
        prop_assert_eq!(union.slot_values, a_slot_values || b_slot_values);
        prop_assert_eq!(union.slot_taint, a_slot_taint || b_slot_taint);
        prop_assert_eq!(union.action_payloads, a_action_payloads || b_action_payloads);
        prop_assert!(a.union_matches_flags(b, union));
        prop_assert!(UnsupportedRecoveryState::SUPPORTED.is_fully_supported());
    }

    #[test]
    fn vb_god2f_recovery_action_replay_properties(
        ticket in ticket_strategy(),
        output_raw in any::<u16>(),
        encoded_len in any::<u32>(),
        taint in taint_strategy(),
        digest in any::<[u8; 32]>(),
        mark_failed_first in any::<bool>(),
    ) {
        let output = SlotIdx::new(output_raw);
        let mut tracker = ActionReplayTracker::new();
        let first = tracker.mark_completed_envelope(ticket, output, encoded_len, taint, digest);
        prop_assert!(first.is_ok(), "first completion envelope must apply, got {first:?}");
        prop_assert!(tracker.has_completed(ticket.action, ticket.step));

        let duplicate = tracker.mark_completed_envelope(ticket, output, encoded_len, taint, digest);
        prop_assert!(duplicate.is_ok(), "identical completion envelope is a duplicate, got {duplicate:?}");

        let divergent = tracker.mark_completed_envelope(
            ticket,
            alternate_slot(output),
            encoded_len,
            taint,
            digest,
        );
        prop_assert!(
            matches!(divergent, Err(RecoveryError::ReplayDivergence { .. })),
            "divergent duplicate envelope must fail closed, got {divergent:?}"
        );

        let mut resolved = ActionReplayTracker::new();
        if mark_failed_first {
            resolved.mark_failed(ticket.action, ticket.step);
            prop_assert!(resolved.has_failed(ticket.action, ticket.step));
        } else {
            resolved.mark_completed(ticket.action, ticket.step);
            prop_assert!(resolved.has_completed(ticket.action, ticket.step));
        }
        prop_assert!(resolved.is_resolved(ticket.action, ticket.step));
        let blocked = resolved.mark_completed_envelope(ticket, output, encoded_len, taint, digest);
        prop_assert!(
            matches!(blocked, Err(RecoveryError::NonIdempotentActionBlocked { .. })),
            "resolved action completion must be blocked, got {blocked:?}"
        );
    }
}

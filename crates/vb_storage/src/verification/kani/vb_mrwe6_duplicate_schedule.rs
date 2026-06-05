#![cfg(kani)]

//! Kani harness for obl-vb-mrwe-6-duplicate-kani-014.

use crate::events::JournalEvent;
use crate::journal::append::{
    VerificationActionIndexIntent, VerificationDuplicateRetryDecision,
    verification_action_index_intent, verification_duplicate_retry_decision,
    verification_event_and_index_keys_exist,
};
use crate::types::EventSeq;
use vb_core::{ActionId, RunId, StepIdx};

fn schedule(
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
    attempt: u16,
) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq,
        step,
        action,
        attempt,
    }
}

#[kani::proof]
fn duplicate_schedule_retry_is_idempotent_or_conflict() {
    let run = RunId::new(kani::any::<u64>());
    let seq = EventSeq::new(kani::any::<u64>());
    let step = StepIdx::new(kani::any::<u16>());
    let action = ActionId::new(kani::any::<u16>());
    let attempt = kani::any::<u16>();
    let divergent = kani::any::<bool>();
    let retry_attempt = if divergent {
        attempt.wrapping_add(1)
    } else {
        attempt
    };

    let existing = schedule(run, seq, step, action, attempt);
    let retry = schedule(run, seq, step, action, retry_attempt);
    let equal_payload = existing == retry;
    let existing_intent = verification_action_index_intent(&existing);
    let retry_intent = verification_action_index_intent(&retry);
    let marker_present = kani::any::<bool>();
    let decision = verification_duplicate_retry_decision(&existing, &retry, marker_present);

    assert!(matches!(
        existing_intent,
        VerificationActionIndexIntent::Put {
            action: a,
            run: r,
            step: s,
        } if a == action && r == run && s == step
    ));
    assert_eq!(existing_intent, retry_intent);
    assert!(matches!(
        verification_event_and_index_keys_exist(&existing),
        Ok(true)
    ));
    assert!(matches!(
        verification_event_and_index_keys_exist(&retry),
        Ok(true)
    ));

    if equal_payload && marker_present {
        assert_eq!(
            decision,
            VerificationDuplicateRetryDecision::IdempotentEqualRetry
        );
    }
    if equal_payload && !marker_present {
        assert_eq!(
            decision,
            VerificationDuplicateRetryDecision::MissingExpectedIndexState
        );
    }
    if !equal_payload {
        assert_eq!(
            decision,
            VerificationDuplicateRetryDecision::DivergentDuplicateConflict
        );
    }
    assert_eq!(equal_payload, !divergent || retry_attempt == attempt);

    core::mem::forget(existing);
    core::mem::forget(retry);
}

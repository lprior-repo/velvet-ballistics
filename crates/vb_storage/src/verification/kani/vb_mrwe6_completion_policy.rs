#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-completion-kani.

use crate::events::JournalEvent;
use crate::journal::append::{
    VerificationActionIndexIntent, VerificationResolutionCommitDecision,
    verification_action_index_intent, verification_event_and_index_keys_exist,
    verification_resolution_commit_decision, verification_resolution_marker_present_after_commit,
};
use crate::types::EventSeq;
use vb_core::{ActionId, RunId, StepIdx};

#[derive(Clone, Copy)]
enum ResolutionKind {
    Completed,
    Failed,
}

fn resolution(
    kind: ResolutionKind,
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
) -> JournalEvent {
    match kind {
        ResolutionKind::Completed => JournalEvent::ActionCompletedEvent {
            run,
            seq,
            step,
            action,
            attempt: kani::any::<u16>(),
        },
        ResolutionKind::Failed => JournalEvent::ActionFailedEvent {
            run,
            seq,
            step,
            action,
            attempt: kani::any::<u16>(),
        },
    }
}

#[kani::proof]
fn vb_mrwe6_completion_policy_all_cases() {
    let kind = if kani::any::<bool>() {
        ResolutionKind::Completed
    } else {
        ResolutionKind::Failed
    };
    let run = RunId::new(kani::any::<u64>());
    let seq = EventSeq::new(kani::any::<u64>());
    let step = StepIdx::new(kani::any::<u16>());
    let action = ActionId::new(kani::any::<u16>());
    let mismatched_key = kani::any::<bool>();
    let resolved_action = if mismatched_key {
        ActionId::new(action.get().wrapping_add(1))
    } else {
        action
    };
    let event = resolution(kind, run, seq, step, resolved_action);
    let intent = verification_action_index_intent(&event);

    kani::assert(matches!(
        intent, VerificationActionIndexIntent::Delete {
            action: a,
            run: r,
            step: s,
        } if a == resolved_action && r == run && s == step
    ));
    kani::assert(matches!(
        verification_event_and_index_keys_exist(&event),
        Ok(true)
    ));

    let commit_success = kani::any::<bool>();
    let same_key = !mismatched_key;
    let decision =
        verification_resolution_commit_decision(&event, action, run, step, commit_success);
    let marker_present_after_commit =
        match verification_resolution_marker_present_after_commit(&event, same_key, commit_success)
        {
            Ok(present) => present,
            Err(_) => true,
        };
    if commit_success && same_key {
        kani::assert(matches!(
            decision,
            Ok(VerificationResolutionCommitDecision::CommittedAndMarkerRemoved)
        ));
        kani::assert(!marker_present_after_commit, "kani harness assertion");
    }
    if !commit_success && same_key {
        kani::assert(matches!(
            decision,
            Ok(VerificationResolutionCommitDecision::CommitFailedMarkerRetained)
        ));
        kani::assert(marker_present_after_commit, "kani harness assertion");
    }
    if mismatched_key {
        kani::assert(matches!(
            decision,
            Ok(VerificationResolutionCommitDecision::MismatchedResolutionRejected)
        ));
        kani::assert(marker_present_after_commit, "kani harness assertion");
    }

    core::mem::forget(event);
}

#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-recovery-kani.
//!
//! The harness is bound to production-adjacent verification seams in
//! `journal::append`: the same `ActionIndexIntent::for_event` classifier and
//! `index_action_key` construction used by append/stage/remove paths feed the
//! recovery outcome seam. The harness generates marker/resolution/legacy
//! permutations and proves non-legacy parity mismatches cannot classify as
//! ordinary pending inventory.

use crate::events::JournalEvent;
use crate::journal::append::{
    VerificationActionIndexIntent, VerificationRecoveryOutcome, verification_action_index_intent,
    verification_recovery_outcome,
};
use crate::types::EventSeq;
use vb_core::{ActionId, RunId, StepIdx};

#[derive(Clone, Copy)]
enum ResolutionShape {
    None,
    SameKey,
    MismatchedKey,
}

fn generated_resolution_shape() -> ResolutionShape {
    if kani::any::<bool>() {
        ResolutionShape::None
    } else if kani::any::<bool>() {
        ResolutionShape::SameKey
    } else {
        ResolutionShape::MismatchedKey
    }
}

fn schedule(run: RunId, seq: EventSeq, step: StepIdx, action: ActionId) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq,
        step,
        action,
        attempt: kani::any::<u16>(),
    }
}

fn completion(run: RunId, seq: EventSeq, step: StepIdx, action: ActionId) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq,
        step,
        action,
        attempt: kani::any::<u16>(),
    }
}

#[kani::proof]
fn vb_mrwe6_recovery_inventory_all_cases() {
    let run = RunId::new(kani::any::<u64>());
    let seq = EventSeq::new(kani::any::<u64>());
    let resolution_seq = EventSeq::new(seq.get().wrapping_add(1));
    let action = ActionId::new(kani::any::<u16>());
    let step = StepIdx::new(kani::any::<u16>());
    let scheduled = schedule(run, seq, step, action);
    let marker_present = kani::any::<bool>();
    let legacy_profile = kani::any::<bool>();
    let resolution_shape = generated_resolution_shape();
    let mismatched_action = ActionId::new(action.get().wrapping_add(1));
    let resolution_event = match resolution_shape {
        ResolutionShape::None => None,
        ResolutionShape::SameKey => Some(completion(run, resolution_seq, step, action)),
        ResolutionShape::MismatchedKey => {
            Some(completion(run, resolution_seq, step, mismatched_action))
        }
    };

    kani::assert(matches!(
        verification_action_index_intent(&scheduled), VerificationActionIndexIntent::Put {
            action: classified_action,
            run: classified_run,
            step: classified_step,
        } if classified_action == action && classified_run == run && classified_step == step
    ));
    if let Some(resolution) = &resolution_event {
        kani::assert(matches!(
            verification_action_index_intent(resolution), VerificationActionIndexIntent::Delete { .. }
        ));
    }

    let outcome = verification_recovery_outcome(
        &scheduled,
        resolution_event.as_ref(),
        marker_present,
        legacy_profile,
    );

    match resolution_shape {
        ResolutionShape::None if marker_present => {
            kani::assert(matches!(
                outcome, Ok(VerificationRecoveryOutcome::PendingInventory)
            ));
        }
        ResolutionShape::None if legacy_profile => {
            kani::assert(matches!(
                outcome, Ok(VerificationRecoveryOutcome::LegacyFallback)
            ));
        }
        ResolutionShape::None => {
            kani::assert(matches!(
                outcome, Ok(VerificationRecoveryOutcome::ParityDefect)
            ));
        }
        ResolutionShape::SameKey => {
            kani::assert(matches!(
                outcome, Ok(VerificationRecoveryOutcome::ResolvedNoPending)
            ));
        }
        ResolutionShape::MismatchedKey => {
            kani::assert(matches!(
                outcome, Ok(VerificationRecoveryOutcome::ParityDefect)
            ));
        }
    }

    if !legacy_profile && !marker_present && matches!(resolution_shape, ResolutionShape::None) {
        kani::assert(!matches!(
            outcome, Ok(VerificationRecoveryOutcome::PendingInventory)
        ));
    }
    if matches!(resolution_shape, ResolutionShape::MismatchedKey) {
        kani::assert(!matches!(
            outcome, Ok(VerificationRecoveryOutcome::PendingInventory)
        ));
        kani::assert(!matches!(
            outcome, Ok(VerificationRecoveryOutcome::LegacyFallback)
        ));
    }

    core::mem::forget(scheduled);
    core::mem::forget(resolution_event);
}

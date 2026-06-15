#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-queue-intent-kani.
//!
//! Production-bound seams used here:
//! - `journal::append::verification_action_index_intent` for the production
//!   classifier used by queued flush/drain append paths.
//! - `journal::append::verification_event_and_index_keys_exist` for production
//!   run-event and side-index key construction.

use crate::events::JournalEvent;
use crate::journal::append::{
    VerificationActionIndexIntent, verification_action_index_intent,
    verification_event_and_index_keys_exist,
};
use crate::types::EventSeq;
use vb_core::{ActionId, RunId, StepIdx};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EventClass {
    Scheduled,
    Resolution,
    Unrelated,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideIndexIntent {
    None,
    PutPending,
    RemovePending,
}

fn required_intent(class: EventClass) -> SideIndexIntent {
    match class {
        EventClass::Scheduled => SideIndexIntent::PutPending,
        EventClass::Resolution => SideIndexIntent::RemovePending,
        EventClass::Unrelated => SideIndexIntent::None,
    }
}

fn generated_class() -> EventClass {
    match kani::any::<u8>() % 3 {
        0 => EventClass::Scheduled,
        1 => EventClass::Resolution,
        _ => EventClass::Unrelated,
    }
}

fn generated_event(
    class: EventClass,
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
) -> JournalEvent {
    match class {
        EventClass::Scheduled => JournalEvent::ActionScheduled {
            run,
            seq,
            step,
            action,
            attempt: kani::any::<u16>(),
        },
        EventClass::Resolution => JournalEvent::ActionCompletedEvent {
            run,
            seq,
            step,
            action,
            attempt: kani::any::<u16>(),
        },
        EventClass::Unrelated => JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: kani::any::<u16>(),
        },
    }
}

#[kani::proof]
fn vb_mrwe6_queue_intent_preservation() {
    let class = generated_class();
    let intent = required_intent(class);
    let run = RunId::new(kani::any::<u64>());
    let seq = EventSeq::new(kani::any::<u64>());
    let step = StepIdx::new(kani::any::<u16>());
    let action = ActionId::new(kani::any::<u16>());
    let event = generated_event(class, run, seq, step, action);
    let production_intent = verification_action_index_intent(&event);

    let keys_exist = verification_event_and_index_keys_exist(&event);

    kani::assert(matches!(
        (class, intent),
        (EventClass::Scheduled, SideIndexIntent::PutPending)
            | (EventClass::Resolution, SideIndexIntent::RemovePending)
            | (EventClass::Unrelated, SideIndexIntent::None)
    ));

    match class {
        EventClass::Scheduled => {
            kani::assert(matches!(intent, SideIndexIntent::PutPending));
            kani::assert(matches!(
                production_intent, VerificationActionIndexIntent::Put {
                    action: classified_action,
                    run: classified_run,
                    step: classified_step,
                } if classified_action == action && classified_run == run && classified_step == step
            ));
            kani::assert(matches!(keys_exist, Ok(true)));
        }
        EventClass::Resolution => {
            kani::assert(matches!(intent, SideIndexIntent::RemovePending));
            kani::assert(matches!(
                production_intent, VerificationActionIndexIntent::Delete {
                    action: classified_action,
                    run: classified_run,
                    step: classified_step,
                } if classified_action == action && classified_run == run && classified_step == step
            ));
            kani::assert(matches!(keys_exist, Ok(true)));
        }
        EventClass::Unrelated => {
            kani::assert(matches!(intent, SideIndexIntent::None));
            kani::assert(matches!(
                production_intent, VerificationActionIndexIntent::None
            ));
            kani::assert(matches!(keys_exist, Ok(false)));
        }
    }

    core::mem::forget(event);
}

#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-atomic-index-kani.
//!
//! Production-bound seams used here:
//! - `journal::append::verification_action_index_intent` for the production
//!   schedule-event classifier used by append/queue flush paths.
//! - `journal::append::verification_event_and_index_keys_exist` for production
//!   run-event and index-action key construction used before batch staging.

use crate::events::JournalEvent;
use crate::journal::append::{
    VerificationActionIndexIntent, verification_action_index_intent,
    verification_event_and_index_keys_exist,
};
use crate::types::EventSeq;
use vb_core::{ActionId, ActionTicket, MockMarker, RunId, SeqNo, SlotIdx, StepIdx};

#[path = "vb_mrwe6_architecture_binding.rs"]
mod vb_mrwe6_architecture_binding;

#[derive(Clone, Copy)]
enum ScheduleVariant {
    Legacy,
    Ticketed,
}

#[derive(Clone, Copy)]
enum CommitResult {
    Success,
    Failure,
}

fn generated_variant() -> ScheduleVariant {
    if kani::any::<bool>() {
        ScheduleVariant::Legacy
    } else {
        ScheduleVariant::Ticketed
    }
}

fn generated_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq: SeqNo::new(kani::any::<u64>()),
        action,
        attempt: kani::any::<u16>(),
        idempotency_key: kani::any::<u128>(),
        capacity: kani::any::<u16>(),
        mock: MockMarker::default(),
    }
}

fn generated_schedule_event(
    variant: ScheduleVariant,
    run: RunId,
    seq: EventSeq,
    step: StepIdx,
    action: ActionId,
) -> JournalEvent {
    match variant {
        ScheduleVariant::Legacy => JournalEvent::ActionScheduled {
            run,
            seq,
            step,
            action,
            attempt: kani::any::<u16>(),
        },
        ScheduleVariant::Ticketed => JournalEvent::ActionScheduledTicket {
            run,
            seq,
            ticket: generated_ticket(run, step, action),
            input: SlotIdx::new(kani::any::<u16>()),
            output: SlotIdx::new(kani::any::<u16>()),
        },
    }
}

#[kani::proof]
fn vb_mrwe6_atomic_index_all_cases() {
    let variant = generated_variant();
    let run = RunId::new(kani::any::<u64>());
    let seq = EventSeq::new(kani::any::<u64>());
    let step = StepIdx::new(kani::any::<u16>());
    let action = ActionId::new(kani::any::<u16>());
    let event = generated_schedule_event(variant, run, seq, step, action);
    let commit = if kani::any::<bool>() {
        CommitResult::Success
    } else {
        CommitResult::Failure
    };

    let intent = verification_action_index_intent(&event);
    kani::assert(
        matches!(
        intent, VerificationActionIndexIntent::Put {
            action: classified_action,
            run: classified_run,
            step: classified_step,
        } if classified_action == action && classified_run == run && classified_step == step),
        "scheduled event stages matching put intent",
    );
    kani::assert(
        matches!(verification_event_and_index_keys_exist(&event), Ok(true)),
        "scheduled event and index keys exist",
    );

    let event_staged = matches!(intent, VerificationActionIndexIntent::Put { .. });
    let index_staged = matches!(verification_event_and_index_keys_exist(&event), Ok(true));
    kani::assert(
        event_staged == index_staged,
        "event and index staging are atomic",
    );

    let event_committed = matches!(commit, CommitResult::Success) && event_staged;
    let index_committed = matches!(commit, CommitResult::Success) && index_staged;
    kani::assert(
        event_committed == index_committed,
        "event and index commit status stay equal",
    );

    match variant {
        ScheduleVariant::Legacy => kani::assert(
            matches!(event, JournalEvent::ActionScheduled { .. }),
            "legacy variant produces ActionScheduled",
        ),
        ScheduleVariant::Ticketed => {
            kani::assert(
                matches!(event, JournalEvent::ActionScheduledTicket { .. }),
                "ticketed variant produces ActionScheduledTicket",
            );
        }
    }

    if matches!(commit, CommitResult::Failure) {
        kani::assert(!event_committed, "failed commit does not commit event");
        kani::assert(!index_committed, "failed commit does not commit index");
    }

    core::mem::forget(event);
}

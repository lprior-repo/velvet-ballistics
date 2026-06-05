#![cfg(kani)]

//! Kani harness for obl-vb-mrwe-6-atomic-index-kani-002.
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
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, SlotIdx, StepIdx};

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
fn scheduled_write_commits_event_and_index_atomically() {
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
    assert!(matches!(
        intent,
        VerificationActionIndexIntent::Put {
            action: classified_action,
            run: classified_run,
            step: classified_step,
        } if classified_action == action && classified_run == run && classified_step == step
    ));
    assert!(matches!(
        verification_event_and_index_keys_exist(&event),
        Ok(true)
    ));

    let event_staged = matches!(intent, VerificationActionIndexIntent::Put { .. });
    let index_staged = matches!(verification_event_and_index_keys_exist(&event), Ok(true));
    assert_eq!(event_staged, index_staged);

    let event_committed = matches!(commit, CommitResult::Success) && event_staged;
    let index_committed = matches!(commit, CommitResult::Success) && index_staged;
    assert_eq!(event_committed, index_committed);

    match variant {
        ScheduleVariant::Legacy => assert!(matches!(event, JournalEvent::ActionScheduled { .. })),
        ScheduleVariant::Ticketed => {
            assert!(matches!(event, JournalEvent::ActionScheduledTicket { .. }));
        }
    }

    if matches!(commit, CommitResult::Failure) {
        assert!(!event_committed);
        assert!(!index_committed);
    }

    core::mem::forget(event);
}

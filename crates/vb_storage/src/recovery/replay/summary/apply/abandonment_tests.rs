use super::summarize_recovery_events;
use crate::recovery::replay::summary::recover_raw_runtime_frame_seed_from_events;
use crate::recovery::types::RecoveryError;
use crate::{EventSeq, JournalEvent};
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};

fn ticket() -> ActionTicket {
    let run = RunId::new(81_501);
    let action = ActionId::new(7);
    let seq = SeqNo::new(1);
    ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
        fencing_token: 0,
    }
}

fn scheduled(ticket: ActionTicket, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run: ticket.run,
        seq,
        ticket,
        input: SlotIdx::ZERO,
        output: SlotIdx::new(1),
        action_abi_digest: WorkflowDigest::from_bytes([4; 32]),
    }
}

fn abandoned(ticket: ActionTicket, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionAbandoned {
        run: ticket.run,
        seq,
        ticket,
    }
}

#[test]
fn summary_and_frame_block_resolution_or_reschedule_after_abandonment() {
    let ticket = ticket();
    let seq = EventSeq::new(2);
    for next in [
        scheduled(ticket, seq),
        JournalEvent::ActionScheduled {
            run: ticket.run,
            seq,
            step: ticket.step,
            action: ticket.action,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run: ticket.run,
            seq,
            step: ticket.step,
            action: ticket.action,
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run: ticket.run,
            seq,
            step: ticket.step,
            action: ticket.action,
            attempt: 99,
        },
    ] {
        let events = [
            scheduled(ticket, EventSeq::ZERO),
            abandoned(ticket, EventSeq::new(1)),
            next,
        ];
        for result in [
            summarize_recovery_events(&events).map(|_| ()),
            recover_raw_runtime_frame_seed_from_events(&events).map(|_| ()),
        ] {
            assert!(matches!(
                result,
                Err(RecoveryError::NonIdempotentActionBlocked { action, step })
                    if action == ticket.action && step == ticket.step
            ));
        }
    }
}

#[test]
fn repeated_abandonment_preserves_existing_summary_counters() -> Result<(), RecoveryError> {
    let ticket = ticket();
    let events = [
        scheduled(ticket, EventSeq::ZERO),
        abandoned(ticket, EventSeq::new(1)),
        abandoned(ticket, EventSeq::new(2)),
    ];
    let summary = summarize_recovery_events(&events)?.summary();
    let frame = recover_raw_runtime_frame_seed_from_events(&events)?;
    assert_eq!(summary.actions_scheduled, 1);
    assert_eq!(summary.actions_resolved, 2);
    assert_eq!(frame.summary.actions_scheduled, summary.actions_scheduled);
    assert_eq!(frame.summary.actions_resolved, summary.actions_resolved);
    Ok(())
}

#[test]
fn legacy_abandonment_without_full_schedule_remains_accepted() -> Result<(), RecoveryError> {
    let ticket = ticket();
    let events = [
        abandoned(ticket, EventSeq::ZERO),
        abandoned(ticket, EventSeq::new(1)),
    ];
    let summary = summarize_recovery_events(&events)?.summary();
    assert_eq!(summary.actions_scheduled, 0);
    assert_eq!(summary.actions_resolved, 2);
    Ok(())
}

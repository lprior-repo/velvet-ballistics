#![forbid(unsafe_code)]

use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};

use crate::recovery::replay::summary::recover_raw_runtime_frame_seed_from_events;
use crate::recovery::types::{RecoveryError, RecoveryFrameSeed};
use crate::{EventSeq, JournalEvent};

fn scheduled_ticket(attempt: u16) -> ActionTicket {
    scheduled_ticket_for(ActionId::new(7), StepIdx::ZERO, SeqNo::new(1), attempt)
}

fn scheduled_ticket_for(action: ActionId, step: StepIdx, seq: SeqNo, attempt: u16) -> ActionTicket {
    let run = RunId::new(41);
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
        fencing_token: 0,
    }
}

fn step_started() -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(41),
        seq: EventSeq::ZERO,
        step: StepIdx::ZERO,
        attempt: 1,
    }
}

fn scheduled(attempt: u16, seq: EventSeq) -> JournalEvent {
    scheduled_event_for(scheduled_ticket(attempt), seq)
}

fn scheduled_event_for(ticket: ActionTicket, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run: ticket.run,
        seq,
        ticket,
        input: SlotIdx::ZERO,
        output: SlotIdx::new(1),
        action_abi_digest: WorkflowDigest::from_bytes([4; 32]),
    }
}

fn failed(attempt: u16, seq: EventSeq) -> JournalEvent {
    failed_for(ActionId::new(7), StepIdx::ZERO, attempt, seq)
}

fn failed_for(action: ActionId, step: StepIdx, attempt: u16, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(41),
        seq,
        step,
        action,
        attempt,
    }
}

#[test]
fn frame_seed_rejects_failure_attempt_that_differs_from_full_schedule() -> Result<(), String> {
    for (scheduled_attempt, failed_attempt) in [(1, 2), (2, 1)] {
        let events = [
            step_started(),
            scheduled(scheduled_attempt, EventSeq::new(1)),
            failed(failed_attempt, EventSeq::new(2)),
        ];
        let error = recover_raw_runtime_frame_seed_from_events(&events)
            .err()
            .ok_or("mismatched failure attempt unexpectedly recovered")?;
        assert_eq!(
            error,
            RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: String::from("action failure attempt does not match schedule ticket"),
            }
        );
    }
    Ok(())
}

#[test]
fn frame_seed_accepts_failure_attempt_matching_full_schedule() -> Result<(), String> {
    for attempt in [1, 2] {
        let events = [
            step_started(),
            scheduled(attempt, EventSeq::new(1)),
            failed(attempt, EventSeq::new(2)),
        ];
        let seed = recover_raw_runtime_frame_seed_from_events(&events)
            .map_err(|error| error.to_string())?;
        assert!(seed.pending_actions.is_empty());
        assert_eq!(seed.summary.actions_scheduled, 1);
        assert_eq!(seed.summary.actions_resolved, 1);
    }
    Ok(())
}

#[test]
fn frame_seed_preserves_failure_without_full_schedule() -> Result<(), String> {
    let legacy_schedule = JournalEvent::ActionScheduled {
        run: RunId::new(41),
        seq: EventSeq::new(1),
        step: StepIdx::ZERO,
        action: ActionId::new(7),
        attempt: 1,
    };
    let cases = [
        vec![step_started(), failed(2, EventSeq::new(1))],
        vec![step_started(), legacy_schedule, failed(2, EventSeq::new(2))],
    ];
    for mut events in cases {
        let seed = recover_raw_runtime_frame_seed_from_events(&events)
            .map_err(|error| error.to_string())?;
        assert!(seed.pending_actions.is_empty());
        assert_eq!(seed.summary.actions_resolved, 1);

        let next_seq = u64::try_from(events.len()).map_err(|error| error.to_string())?;
        events.push(scheduled(1, EventSeq::new(next_seq)));
        let error = recover_raw_runtime_frame_seed_from_events(&events)
            .err()
            .ok_or("legacy resolved action unexpectedly accepted a schedule")?;
        assert_eq!(
            error,
            RecoveryError::NonIdempotentActionBlocked {
                action: ActionId::new(7),
                step: StepIdx::ZERO,
            }
        );
    }
    Ok(())
}

#[test]
fn frame_seed_still_blocks_schedule_after_matching_failure() -> Result<(), String> {
    let events = [
        step_started(),
        scheduled(1, EventSeq::new(1)),
        failed(1, EventSeq::new(2)),
        scheduled(2, EventSeq::new(3)),
    ];
    let error = recover_raw_runtime_frame_seed_from_events(&events)
        .err()
        .ok_or("resolved action unexpectedly accepted a replacement schedule")?;
    assert_eq!(
        error,
        RecoveryError::NonIdempotentActionBlocked {
            action: ActionId::new(7),
            step: StepIdx::ZERO,
        }
    );
    Ok(())
}

fn competing_schedules() -> [JournalEvent; 3] {
    [
        scheduled_event_for(
            scheduled_ticket_for(ActionId::new(7), StepIdx::new(3), SeqNo::new(1), 1),
            EventSeq::ZERO,
        ),
        scheduled_event_for(
            scheduled_ticket_for(ActionId::new(7), StepIdx::new(4), SeqNo::new(2), 2),
            EventSeq::new(1),
        ),
        scheduled_event_for(
            scheduled_ticket_for(ActionId::new(8), StepIdx::new(3), SeqNo::new(3), 2),
            EventSeq::new(2),
        ),
    ]
}

fn pending_pairs(seed: &RecoveryFrameSeed) -> Vec<(ActionId, StepIdx)> {
    let mut pairs: Vec<_> = seed
        .pending_actions
        .iter()
        .map(|entry| (entry.action, entry.step))
        .collect();
    pairs.sort_unstable_by_key(|(action, step)| (action.get(), step.get()));
    pairs
}

#[test]
fn frame_seed_rejects_nonzero_step_mismatch_with_competing_schedules() -> Result<(), String> {
    let mut events = competing_schedules().to_vec();
    let seed =
        recover_raw_runtime_frame_seed_from_events(&events).map_err(|error| error.to_string())?;
    assert_eq!(seed.summary.actions_scheduled, 3);
    assert_eq!(seed.summary.actions_resolved, 0);
    assert_eq!(
        pending_pairs(&seed),
        vec![
            (ActionId::new(7), StepIdx::new(3)),
            (ActionId::new(7), StepIdx::new(4)),
            (ActionId::new(8), StepIdx::new(3)),
        ]
    );

    events.push(failed_for(
        ActionId::new(7),
        StepIdx::new(3),
        2,
        EventSeq::new(3),
    ));
    let error = recover_raw_runtime_frame_seed_from_events(&events)
        .err()
        .ok_or("competing schedule masked a nonzero-step failure mismatch")?;
    assert_eq!(
        error,
        RecoveryError::ReplayDivergence {
            step: StepIdx::new(3),
            detail: String::from("action failure attempt does not match schedule ticket"),
        }
    );
    Ok(())
}

#[test]
fn frame_seed_matching_failure_preserves_competing_pending_pairs() -> Result<(), String> {
    let mut events = competing_schedules().to_vec();
    events.push(failed_for(
        ActionId::new(7),
        StepIdx::new(3),
        1,
        EventSeq::new(3),
    ));
    let seed =
        recover_raw_runtime_frame_seed_from_events(&events).map_err(|error| error.to_string())?;
    assert_eq!(seed.summary.actions_scheduled, 3);
    assert_eq!(seed.summary.actions_resolved, 1);
    assert_eq!(
        pending_pairs(&seed),
        vec![
            (ActionId::new(7), StepIdx::new(4)),
            (ActionId::new(8), StepIdx::new(3)),
        ]
    );
    Ok(())
}

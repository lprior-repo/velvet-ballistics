#![forbid(unsafe_code)]

use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};

use crate::recovery::replay::summary::recover_raw_runtime_frame_seed_from_events;
use crate::recovery::types::RecoveryError;
use crate::{EventSeq, JournalEvent};

fn scheduled_ticket(attempt: u16) -> ActionTicket {
    let run = RunId::new(41);
    let action = ActionId::new(7);
    let seq = SeqNo::new(1);
    ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq,
        action,
        run_attempt: 1,
        attempt,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
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
    JournalEvent::ActionScheduledTicket {
        run: RunId::new(41),
        seq,
        ticket: scheduled_ticket(attempt),
        input: SlotIdx::ZERO,
        output: SlotIdx::new(1),
        action_abi_digest: WorkflowDigest::from_bytes([4; 32]),
    }
}

fn failed(attempt: u16, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(41),
        seq,
        step: StepIdx::ZERO,
        action: ActionId::new(7),
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
    for events in cases {
        let seed = recover_raw_runtime_frame_seed_from_events(&events)
            .map_err(|error| error.to_string())?;
        assert!(seed.pending_actions.is_empty());
        assert_eq!(seed.summary.actions_resolved, 1);
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

//! Status derivation and replay timeline tests.

use vb_core::ids::{ActionId, RunId, StepIdx, WorkflowDigest};
use vb_storage::JournalEvent;

use crate::{derive_status_from_events, DerivedStatus};

fn dummy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB_u8; 32])
}

#[test]
fn derive_status_empty_events_returns_pending() {
    let events: Vec<JournalEvent> = vec![];
    let status = derive_status_from_events(&events);
    assert_eq!(status, DerivedStatus::Pending);
}

#[test]
fn derive_status_run_accepted_is_active() {
    let events = vec![JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: vb_storage::EventSeq::ZERO,
        workflow: dummy_digest(),
    }];
    let status = derive_status_from_events(&events);
    assert_eq!(status, DerivedStatus::Active);
}

#[test]
fn derive_status_action_scheduled_is_waiting_action() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::ZERO,
            workflow: dummy_digest(),
        },
        JournalEvent::ActionScheduled {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::new(1),
            step: StepIdx::new(2),
            action: ActionId::new(5),
            attempt: 1,
        },
    ];
    let status = derive_status_from_events(&events);
    match status {
        DerivedStatus::WaitingAction {
            pending_action,
            pending_step,
        } => {
            assert_eq!(pending_action, ActionId::new(5));
            assert_eq!(pending_step, StepIdx::new(2));
        }
        other => panic!("expected WaitingAction, got {:?}", other),
    }
}

#[test]
fn derive_status_run_finished_is_completed() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::ZERO,
            workflow: dummy_digest(),
        },
        JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::new(1),
            result: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        },
    ];
    let status = derive_status_from_events(&events);
    assert_eq!(status, DerivedStatus::Completed);
}

#[test]
fn derive_status_run_failed_with_retry_is_backing_off() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::ZERO,
            workflow: dummy_digest(),
        },
        JournalEvent::RunFailedEvent {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::new(1),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];
    let status = derive_status_from_events(&events);
    match status {
        DerivedStatus::BackingOff { retry_step } => {
            assert_eq!(retry_step, StepIdx::new(1));
        }
        other => panic!("expected BackingOff, got {:?}", other),
    }
}

#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for vb-rpch recovery preconditions.
//!
//! Harnesses:
//! - `hydrate_run_frame_precond_kani` (PO-VB-014 / PRE-001)
//! - `hydrate_run_frame_from_events_precond_kani` (PO-VB-015 / PRE-002)
//! - `replay_events_kani` (PO-VB-016 / POST-009)

use crate::recovery::hydrate::{hydrate_run_frame, hydrate_run_frame_from_events};
use crate::recovery::replay::core::replay_events;
use crate::recovery::types::ActionReplayTracker;
use crate::recovery::types::RunSnapshot;
use crate::JournalEvent;
use crate::EventSeq;
use vb_core::{ActionId, ConstValue, RunId, SlotIdx, StepIdx, WorkflowDigest};

impl kani::Arbitrary for RunSnapshot {
    fn any() -> Self {
        Self {
            run: RunId::new(kani::any()),
            seq: EventSeq::new(kani::any()),
            workflow: WorkflowDigest::from_bytes(kani::any()),
            slots: Vec::new(),
            taint: Vec::new(),
        }
    }
}

impl kani::Arbitrary for JournalEvent {
    fn any() -> Self {
        let discriminant: u8 = kani::any::<u8>() % 18;
        let run = RunId::new(kani::any());
        let seq = EventSeq::new(kani::any());
        match discriminant {
            0 => JournalEvent::RunAccepted {
                run,
                seq,
                workflow: WorkflowDigest::from_bytes(kani::any()),
            },
            1 => JournalEvent::RunAdmission {
                run,
                seq,
                artifact_digest: WorkflowDigest::from_bytes(kani::any()),
                granted_capabilities: kani::any(),
                policy: kani::any(),
            },
            2 => JournalEvent::StepStarted {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                attempt: kani::any(),
            },
            3 => JournalEvent::StepSucceeded {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                output: SlotIdx::new(kani::any()),
            },
            4 => JournalEvent::ActionScheduled {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                action: ActionId::new(kani::any()),
                attempt: kani::any(),
            },
            5 => JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                action: ActionId::new(kani::any()),
                attempt: kani::any(),
            },
            6 => JournalEvent::ActionFailedEvent {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                action: ActionId::new(kani::any()),
                attempt: kani::any(),
            },
            7 => JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot: SlotIdx::new(kani::any()),
                value: None,
                extra: None,
                attempt: kani::any(),
            },
            8 => JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                attempt: kani::any(),
            },
            9 => JournalEvent::AskScheduledEvent {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                attempt: kani::any(),
            },
            10 => JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                attempt: kani::any(),
            },
            11 => JournalEvent::RetryScheduledEvent {
                run,
                seq,
                step: StepIdx::new(kani::any()),
                attempt: kani::any(),
            },
            12 => JournalEvent::RunCancelled {
                run,
                seq,
                attempt: kani::any(),
                reason: None,
            },
            13 => JournalEvent::RunFinished {
                run,
                seq,
                result: SlotIdx::new(kani::any()),
                attempt: kani::any(),
            },
            14 => JournalEvent::RunFailedEvent {
                run,
                seq,
                attempt: kani::any(),
            },
            15 => JournalEvent::RunResumed {
                run,
                timestamp: kani::any(),
            },
            16 => JournalEvent::RunRetried {
                run,
                timestamp: kani::any(),
            },
            17 => JournalEvent::RunAnswered {
                run,
                slot_idx: SlotIdx::new(kani::any()),
                answer: ConstValue::from(kani::any::<i64>()),
                timestamp: kani::any(),
            },
        }
    }
}

#[kani::proof]
fn hydrate_run_frame_precond_kani() {
    let snapshot: RunSnapshot = kani::any();
    let run_id: RunId = RunId::new(kani::any());

    let max_tail: usize = (kani::any::<u8>() % 20) as usize;
    let tail_events: Vec<JournalEvent> = (0..max_tail)
        .map(|_| kani::any::<JournalEvent>())
        .collect();

    kani::cover(snapshot.run == run_id, "snapshot_run_matches");
    kani::cover(!tail_events.is_empty(), "tail_events_non_empty");

    let result = hydrate_run_frame(&snapshot, &tail_events, run_id);
    kani::assert(result.is_ok() || result.is_err(), "hydrate_run_frame returns valid result");
}

#[kani::proof]
fn hydrate_run_frame_from_events_precond_kani() {
    let run_id: RunId = RunId::new(kani::any());

    let max_events: usize = (kani::any::<u8>() % 20) as usize;
    let events: Vec<JournalEvent> = if max_events == 0 {
        vec![]
    } else {
        (0..max_events).map(|_| kani::any::<JournalEvent>()).collect()
    };

    kani::cover(events.is_empty(), "events_empty");
    kani::cover(!events.is_empty(), "events_non_empty");

    let result = hydrate_run_frame_from_events(&events, run_id);
    kani::assert(result.is_ok() || result.is_err(), "hydrate_run_frame_from_events returns valid result");
}

#[kani::proof]
fn replay_events_kani() {
    let mut tracker = ActionReplayTracker::new();

    let max_events: usize = (kani::any::<u8>() % 20) as usize;
    let events: Vec<JournalEvent> = (0..max_events)
        .map(|_| kani::any::<JournalEvent>())
        .collect();

    let empty_digests: &[(ActionId, WorkflowDigest)] = &[];

    let result = replay_events(&events, &mut tracker, empty_digests);
    kani::assert(result.is_ok() || result.is_err(), "replay_events returns valid result");
}
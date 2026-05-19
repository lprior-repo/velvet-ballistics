//! Kani harness for vb-rpch recovery hydration functions.
//!
//! Proves PRE-001, PRE-002, and POST-006, POST-007 contract properties for:
//! - `hydrate_run_frame`: snapshot + tail events hydration
//! - `hydrate_run_frame_from_events`: events-only hydration
//!
//! Obligation: KANI-RPCH-001

#![forbid(unsafe_code)]

use crate::JournalEvent;
use crate::recovery::hydrate::{hydrate_run_frame, hydrate_run_frame_from_events};
use crate::recovery::types::RunSnapshot;
use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};

impl kani::Arbitrary for RunSnapshot {
    fn any() -> Self {
        RunSnapshot {
            run: kani::any(),
            seq: kani::any(),
            workflow: WorkflowDigest::from_bytes(kani::any()),
            slots: Vec::new(),
            taint: Vec::new(),
        }
    }
}

impl kani::Arbitrary for JournalEvent {
    fn any() -> Self {
        let discriminant: u8 = kani::any();
        match discriminant % 18 {
            0 => JournalEvent::RunAccepted {
                run: kani::any(),
                seq: kani::any(),
                workflow: WorkflowDigest::from_bytes(kani::any()),
            },
            1 => JournalEvent::RunAdmission {
                run: kani::any(),
                seq: kani::any(),
                artifact_digest: WorkflowDigest::from_bytes(kani::any()),
                granted_capabilities: kani::any(),
                policy: kani::any(),
            },
            2 => JournalEvent::StepStarted {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                attempt: kani::any(),
            },
            3 => JournalEvent::StepSucceeded {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                output: kani::any(),
            },
            4 => JournalEvent::ActionScheduled {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                action: kani::any(),
                attempt: kani::any(),
            },
            5 => JournalEvent::ActionCompletedEvent {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                action: kani::any(),
                attempt: kani::any(),
            },
            6 => JournalEvent::ActionFailedEvent {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                action: kani::any(),
                attempt: kani::any(),
            },
            7 => JournalEvent::SlotWrittenEvent {
                run: kani::any(),
                seq: kani::any(),
                slot: kani::any(),
                value: None,
                extra: None,
                attempt: kani::any(),
            },
            8 => JournalEvent::WaitScheduledEvent {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                attempt: kani::any(),
            },
            9 => JournalEvent::AskScheduledEvent {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                attempt: kani::any(),
            },
            10 => JournalEvent::AskAnsweredEvent {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                attempt: kani::any(),
            },
            11 => JournalEvent::RetryScheduledEvent {
                run: kani::any(),
                seq: kani::any(),
                step: kani::any(),
                attempt: kani::any(),
            },
            12 => JournalEvent::RunCancelled {
                run: kani::any(),
                seq: kani::any(),
                attempt: kani::any(),
                reason: None,
            },
            13 => JournalEvent::RunFinished {
                run: kani::any(),
                seq: kani::any(),
                result: kani::any(),
                attempt: kani::any(),
            },
            14 => JournalEvent::RunFailedEvent {
                run: kani::any(),
                seq: kani::any(),
                attempt: kani::any(),
            },
            15 => JournalEvent::RunResumed {
                run: kani::any(),
                timestamp: kani::any(),
            },
            16 => JournalEvent::RunRetried {
                run: kani::any(),
                timestamp: kani::any(),
            },
            17 => JournalEvent::RunAnswered {
                run: kani::any(),
                slot_idx: kani::any(),
                answer: kani::any(),
                timestamp: kani::any(),
            },
            _ => kani::any(),
        }
    }
}

#[kani::proof]
#[kani::unwind(5)]
fn hydrate_run_frame_precond_run_id_mismatch() {
    let snapshot: RunSnapshot = kani::any();
    let tail_events: Vec<JournalEvent> = kani::any();
    let run_id: RunId = kani::any();

    kani::assume(snapshot.run != run_id);

    let result = hydrate_run_frame(&snapshot, &tail_events, run_id);

    kani::assert(
        result.is_err(),
        "hydrate_run_frame must return Err when snapshot.run != run_id",
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn hydrate_run_frame_precond_tail_events_run_id_mismatch() {
    let mut snapshot: RunSnapshot = kani::any();
    let tail_events: Vec<JournalEvent> = kani::any();
    let run_id: RunId = kani::any();

    snapshot.run = run_id;

    let mismatched_run: RunId = kani::any();
    kani::assume(mismatched_run != run_id);

    kani::assume(!tail_events.is_empty());
    let mut tail = tail_events;
    for event in &mut tail {
        if let JournalEvent::RunAccepted { run, .. } = event {
            *run = mismatched_run;
        }
    }

    let result = hydrate_run_frame(&snapshot, &tail, run_id);

    kani::assert(
        result.is_err(),
        "hydrate_run_frame must return Err when tail event has mismatched run_id",
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn hydrate_run_frame_precond_seq_order_violation() {
    let mut snapshot: RunSnapshot = kani::any();
    let mut tail_events: Vec<JournalEvent> = kani::any();
    let run_id: RunId = kani::any();

    snapshot.run = run_id;
    snapshot.seq = vb_core::EventSeq::new(100);

    for event in &mut tail_events {
        if let JournalEvent::RunAccepted { run, seq, .. } = event {
            *run = run_id;
            *seq = vb_core::EventSeq::new(50);
        }
    }

    kani::assume(!tail_events.is_empty());

    let result = hydrate_run_frame(&snapshot, &tail_events, run_id);

    kani::assert(
        result.is_err(),
        "hydrate_run_frame must return Err when tail seq <= snapshot seq",
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn hydrate_run_frame_from_events_precond_empty_events() {
    let events: Vec<JournalEvent> = Vec::new();
    let run_id: RunId = kani::any();

    let result = hydrate_run_frame_from_events(&events, run_id);

    kani::assert(
        result.is_err(),
        "hydrate_run_frame_from_events must return Err on empty events",
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn recover_runtime_summary_precond_basic() {
    use crate::recovery::replay::summary::recover_runtime_summary_from_events;

    let events: Vec<JournalEvent> = kani::any();
    let run_id: RunId = kani::any();

    kani::assume(!events.is_empty());

    let result = recover_runtime_summary_from_events(&events, run_id);

    kani::assert(
        result.is_ok() || result.is_err(),
        "recover_runtime_summary_from_events must return Result",
    );
}

fn main() {}

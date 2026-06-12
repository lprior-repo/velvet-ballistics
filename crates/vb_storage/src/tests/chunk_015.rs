#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;


#[test]
fn append_strict_handles_concurrent_runs_interleaved() {
    // Given a journal with interleaved events from run A and run B
    // When events_for_run is called for run A
    // Then only run A events are returned in order
    let (_guard, journal) = open_journal();
    let run_a = RunId::new(100);
    let run_b = RunId::new(200);

    let a0 = JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let b0 = JournalEvent::RunAccepted {
        run: run_b,
        seq: EventSeq::new(0),
        workflow: test_digest(2),
    };
    let a1 = JournalEvent::StepStarted {
        run: run_a,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let b1 = JournalEvent::StepStarted {
        run: run_b,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let a2 = JournalEvent::RunFinished {
        run: run_a,
        seq: EventSeq::new(2),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    journal
        .append_journaled(&a0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&b0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&a1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&b1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&a2)
        .expect("journal.append_journaled must succeed");

    let events_a = journal
        .events_for_run(run_a)
        .expect("events_for_run A should succeed");
    assert_eq!(events_a.len(), 3);
    assert_eq!(events_a[0], a0);
    assert_eq!(events_a[1], a1);
    assert_eq!(events_a[2], a2);

    let events_b = journal
        .events_for_run(run_b)
        .expect("events_for_run B should succeed");
    assert_eq!(events_b.len(), 2);
    assert_eq!(events_b[0], b0);
    assert_eq!(events_b[1], b1);
}


#[test]
fn append_journaled_succeeds_without_flush() {
    // Given an open journal
    // When append_journaled is called
    // Then the event is readable immediately
    let (_guard, journal) = open_journal();
    let run = RunId::new(30);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    journal
        .append_journaled(&event)
        .expect("journal.append_journaled must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], event);
}


#[test]
fn run_header_record_roundtrip_with_large_timestamp() {
    // Given a run header with a large accepted_at_ms value
    // When put and retrieved
    // Then the timestamp survives exactly
    let (_guard, journal) = open_journal();
    let record = RunHeaderRecord {
        run: RunId::new(1),
        workflow_id: WorkflowId::new(2),
        compiled_digest: test_digest(5),
        status: 0,
        accepted_at_ms: u64::MAX / 2,
    };
    journal
        .put_run_header(&record)
        .expect("journal.put_run_header must succeed");

    let retrieved = journal
        .run_header(RunId::new(1))
        .expect("lookup should succeed");
    assert_eq!(retrieved, Some(record));
}


#[test]
fn snapshot_record_roundtrip_with_nonempty_slots() {
    // Given a snapshot with non-empty slot data
    // When stored and retrieved
    // Then the slot bytes survive exactly
    let (_guard, journal) = open_journal();
    let snapshot = RunSnapshot {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: test_digest(7),
        slots: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE],
        taint: Vec::new(),
    };
    journal
        .put_snapshot(&snapshot)
        .expect("journal.put_snapshot must succeed");

    let retrieved = journal
        .snapshot(RunId::new(1), EventSeq::new(0))
        .expect("lookup should succeed");
    assert_eq!(retrieved, Some(snapshot));
}


#[test]
fn compiled_ir_returns_none_when_different_digest_queried() {
    // Given an open journal with a compiled IR stored at digest [1;32]
    // When a different digest [2;32] is queried
    // Then it returns None
    let (_guard, journal) = open_journal();
    let record = crate::accepted_compiled_ir_record_for_test(vec![1, 2, 3]);
    journal
        .put_compiled_ir(&record)
        .expect("journal.put_compiled_ir must succeed");

    let result = journal
        .compiled_ir(test_digest(2))
        .expect("lookup should succeed");
    assert_eq!(result, None);
}


#[test]
fn workflow_source_returns_none_for_different_digest() {
    // Given an open journal with one workflow source stored
    // When a different digest is queried
    // Then it returns None
    let (_guard, journal) = open_journal();
    let source = vec![1];
    let stored_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord {
        digest: stored_digest,
        source,
    };
    journal
        .put_workflow_source(&record)
        .expect("journal.put_workflow_source must succeed");

    let result = journal
        .workflow_source(test_digest(11))
        .expect("lookup should succeed");
    assert_eq!(result, None);
}

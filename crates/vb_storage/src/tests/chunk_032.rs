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
fn workflow_index_stores_and_queries_by_workflow_id() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let workflow = WorkflowId::new(42);
    let run = RunId::new(7002);
    journal
        .put_workflow_index(workflow, run)
        .expect("put_workflow_index must succeed");
    let key = index_workflow_key(workflow, run).expect("key must succeed");
    let value = journal
        .index_workflow
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(
        value.is_some(),
        "workflow index entry must exist after put at key {:?}, got {:?}",
        key,
        value
    );
}

#[test]
fn action_index_stores_and_queries_by_action_id() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let action = ActionId::new(7);
    let run = RunId::new(7003);
    let step = StepIdx::new(2);
    journal
        .put_action_index(action, run, step)
        .expect("put_action_index must succeed");
    let key = index_action_key(action, run, step).expect("key must succeed");
    let value = journal
        .index_action
        .get(key.as_slice())
        .expect("get must succeed");
    assert!(
        value.is_some(),
        "action index entry must exist after put at key {:?}, got {:?}",
        key,
        value
    );
}

#[test]
fn status_index_multiple_runs_same_state() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let state = IndexStatusState::Other(5);
    let run_1 = RunId::new(7010);
    let run_2 = RunId::new(7011);
    let run_3 = RunId::new(7012);
    journal
        .put_status_index(state, 100, run_1)
        .expect("put_status_index 1 must succeed");
    journal
        .put_status_index(state, 200, run_2)
        .expect("put_status_index 2 must succeed");
    journal
        .put_status_index(state, 300, run_3)
        .expect("put_status_index 3 must succeed");
    let key_1 = index_status_key(state, 100, run_1).expect("key 1 must succeed");
    let key_2 = index_status_key(state, 200, run_2).expect("key 2 must succeed");
    let key_3 = index_status_key(state, 300, run_3).expect("key 3 must succeed");
    assert!(
        journal
            .index_status
            .get(key_1.as_slice())
            .expect("get 1")
            .is_some(),
        "status index entry 1 must exist at key {:?}",
        key_1
    );
    assert!(
        journal
            .index_status
            .get(key_2.as_slice())
            .expect("get 2")
            .is_some(),
        "status index entry 2 must exist at key {:?}",
        key_2
    );
    assert!(
        journal
            .index_status
            .get(key_3.as_slice())
            .expect("get 3")
            .is_some(),
        "status index entry 3 must exist at key {:?}",
        key_3
    );
}

#[test]
fn workflow_index_multiple_runs_same_workflow() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let workflow = WorkflowId::new(99);
    let run_1 = RunId::new(7020);
    let run_2 = RunId::new(7021);
    let run_3 = RunId::new(7022);
    journal
        .put_workflow_index(workflow, run_1)
        .expect("put 1 must succeed");
    journal
        .put_workflow_index(workflow, run_2)
        .expect("put 2 must succeed");
    journal
        .put_workflow_index(workflow, run_3)
        .expect("put 3 must succeed");
    let key_1 = index_workflow_key(workflow, run_1).expect("key 1 must succeed");
    let key_2 = index_workflow_key(workflow, run_2).expect("key 2 must succeed");
    let key_3 = index_workflow_key(workflow, run_3).expect("key 3 must succeed");
    assert!(
        journal
            .index_workflow
            .get(key_1.as_slice())
            .expect("get 1")
            .is_some(),
        "workflow index entry 1 must exist at key {:?}",
        key_1
    );
    assert!(
        journal
            .index_workflow
            .get(key_2.as_slice())
            .expect("get 2")
            .is_some(),
        "workflow index entry 2 must exist at key {:?}",
        key_2
    );
    assert!(
        journal
            .index_workflow
            .get(key_3.as_slice())
            .expect("get 3")
            .is_some(),
        "workflow index entry 3 must exist at key {:?}",
        key_3
    );
}

// --- Record builder (tests 36-40) ---

#[test]
fn builder_initial_len_is_zero() {
    let builder = BatchBuilder::new();
    assert_eq!(builder.len(), 0, "new builder must have len 0");
    assert!(builder.is_empty(), "new builder must be empty");
}

#[test]
fn builder_append_increments_len() {
    let mut builder = BatchBuilder::new();
    let run = RunId::new(8001);
    builder.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    });
    assert_eq!(builder.len(), 1, "builder must have len 1 after one push");
    assert!(!builder.is_empty());
}

#[test]
fn builder_append_multiple_events_len_matches() {
    let mut builder = BatchBuilder::new();
    let run = RunId::new(8002);
    builder.push(JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    });
    builder.push(JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    });
    builder.push(JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(2),
        result: SlotIdx::new(0),
        attempt: 1,
    });
    assert_eq!(
        builder.len(),
        3,
        "builder must have len 3 after three pushes"
    );
}

#[test]
fn builder_as_slice_returns_appended_events() {
    let mut builder = BatchBuilder::new();
    let run = RunId::new(8003);
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let e1 = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };
    builder.push(e0.clone());
    builder.push(e1.clone());
    let slice = builder.as_slice();
    assert_eq!(slice.len(), 2);
    assert_eq!(
        slice[0], e0,
        "first slice element must match first pushed event"
    );
    assert_eq!(
        slice[1], e1,
        "second slice element must match second pushed event"
    );
}

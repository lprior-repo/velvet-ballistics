#![forbid(unsafe_code)]
use super::*;

#[test]
fn new_batch_is_empty_with_zero_length() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    assert!(batch.is_empty(), "newly constructed batch must be empty");
    assert_eq!(
        batch.len(),
        0,
        "newly constructed batch must report length 0"
    );
}

#[test]
fn new_batch_from_journal_batch_method_is_empty() {
    let (_temp, journal) = temp_journal();
    let batch = journal.batch();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
}

#[test]
fn len_increments_after_each_append_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1);
    let mut batch = JournalWriteBatch::new(&journal);

    batch.append_event(&make_event(run, 0)).expect("append 0");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());

    batch.append_event(&make_event(run, 1)).expect("append 1");
    assert_eq!(batch.len(), 2);

    batch.append_event(&make_event(run, 2)).expect("append 2");
    assert_eq!(batch.len(), 3);
}

#[test]
fn len_increments_after_put_run_header() {
    let (_temp, journal) = temp_journal();
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_run_header(&make_run_header(RunId::new(10)))
        .expect("put header");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());
}

#[test]
fn len_increments_after_put_status_index() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20);
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_status_index(IndexStatusState::Active, 12345, run)
        .expect("put status index");
    assert_eq!(batch.len(), 1);
}

#[test]
fn len_increments_after_put_workflow_index() {
    let (_temp, journal) = temp_journal();
    let wf = WorkflowId::new(5);
    let run = RunId::new(30);
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_workflow_index(wf, run)
        .expect("put workflow index");
    assert_eq!(batch.len(), 1);
}

#[test]
fn len_increments_after_put_action_index() {
    let (_temp, journal) = temp_journal();
    let action = vb_core::ActionId::new(99);
    let run = RunId::new(40);
    let step = StepIdx::new(0);
    let mut batch = JournalWriteBatch::new(&journal);
    batch
        .put_action_index(action, run, step)
        .expect("put action index");
    assert_eq!(batch.len(), 1);
}

#[test]
fn empty_batch_commit_succeeds() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    let result = batch.commit();
    assert!(
        result.is_ok(),
        "committing an empty batch should succeed, got {:?}",
        result
    );
}
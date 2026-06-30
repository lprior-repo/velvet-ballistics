#![forbid(unsafe_code)]
use super::*;

#[test]
fn cross_batch_duplicate_is_rejected_with_duplicate_event() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(300);
    let event = make_event(run, 0);

    let mut b1 = JournalWriteBatch::new(&journal);
    b1.append_event(&event).expect("first append");
    b1.commit().expect("first commit");

    let mut b2 = JournalWriteBatch::new(&journal);
    let result = b2.append_event(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "cross-batch duplicate must be DuplicateEvent, got {result:?}"
    );
}

#[test]
fn duplicate_event_aborts_batch() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(301);
    let event = make_event(run, 0);

    let mut b1 = JournalWriteBatch::new(&journal);
    b1.append_event(&event).expect("first append");
    b1.commit().expect("first commit");

    let mut b2 = JournalWriteBatch::new(&journal);
    let result = b2.append_event(&event);
    assert!(matches!(result, Err(JournalError::DuplicateEvent { .. })));
    assert_eq!(b2.len(), 0, "aborted batch must report len 0");
}

#[test]
fn e2e_full_lifecycle_append_to_limit_commit() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(400);
    let mut batch = JournalWriteBatch::new(&journal);

    for i in 0..MAX_BATCH_COUNT {
        batch
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }
    let result = batch.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(matches!(result, Err(JournalError::QueueFull)));

    batch.commit().expect("commit must succeed");

    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(events.len(), MAX_BATCH_COUNT);
}

#[test]
fn e2e_many_events_under_limit_committed_and_replayable() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(401);
    let mut batch = journal.batch();

    let count = 50;
    for i in 0..count {
        batch.append_event(&make_event(run, i)).expect("append");
    }
    assert_eq!(batch.len(), count as usize);
    batch.commit().expect("commit");

    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(events.len(), count as usize);
    assert_eq!(events[0].run_id(), run);
}

#[test]
fn e2e_aborted_batch_commit_returns_typed_batch_aborted_error() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(402);
    let event = make_event(run, 0);

    let mut batch1 = JournalWriteBatch::new(&journal);
    batch1.append_event(&event).expect("append");
    batch1.commit().expect("commit");

    let mut batch2 = JournalWriteBatch::new(&journal);
    let result = batch2.append_event(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { run: _, seq: _ })),
        "duplicate event must produce DuplicateEvent error, got {result:?}"
    );
    let commit_result = batch2.commit();
    assert!(
        matches!(commit_result, Err(JournalError::BatchAborted)),
        "aborted batch commit must return Err(JournalError::BatchAborted), got {commit_result:?}"
    );

    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(
        events.len(),
        1,
        "only one event must persist after aborted batch"
    );
}

#[test]
fn append_strict_batch_atomicity_rolls_back_on_duplicate() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(900);
    let seed = make_event(run, 0);

    let mut seed_batch = JournalWriteBatch::new(&journal);
    seed_batch.append_event(&seed).expect("seed append");
    seed_batch.commit().expect("seed commit");

    let colliding = vec![make_event(run, 0), make_event(run, 1), make_event(run, 2)];
    let result = journal.append_strict_batch(&colliding);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "strict batch with collision must surface DuplicateEvent, got {result:?}"
    );

    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(
        events.len(),
        1,
        "collision must roll back the entire strict batch; expected only seeded seq=0, got {events:?}"
    );
}

#[test]
fn e2e_mixed_accept_reject_batch_produces_correct_result() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(403);
    let mut batch = journal.batch();

    for i in 0..10 {
        batch.append_event(&make_event(run, i)).expect("append");
    }

    for i in 10..MAX_BATCH_COUNT {
        batch
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }

    let result = batch.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(matches!(result, Err(JournalError::QueueFull)));

    batch.commit().expect("commit");
    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(
        events.len(),
        MAX_BATCH_COUNT,
        "exactly MAX_BATCH_COUNT events must be persisted"
    );
}

#[test]
fn batch_len_at_zero_on_fresh_batch() {
    let (_temp, journal) = temp_journal();
    let batch = journal.batch();
    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
}

#[test]
fn batch_len_at_one_after_single_append() {
    let (_temp, journal) = temp_journal();
    let mut batch = journal.batch();
    batch
        .append_event(&make_event(RunId::new(500), 0))
        .expect("append");
    assert_eq!(batch.len(), 1);
    assert!(!batch.is_empty());
}

#[test]
fn batch_is_empty_equals_len_zero_invariant() {
    let (_temp, journal) = temp_journal();
    let mut batch = journal.batch();

    assert_eq!(batch.is_empty(), batch.len() == 0);

    batch
        .append_event(&make_event(RunId::new(501), 0))
        .expect("append");
    assert_eq!(batch.is_empty(), batch.len() == 0);

    batch
        .append_event(&make_event(RunId::new(502), 1))
        .expect("append");
    assert_eq!(batch.is_empty(), batch.len() == 0);
}

#[test]
fn multiple_events_with_different_run_ids_committed_correctly() {
    let (_temp, journal) = temp_journal();
    let run1 = RunId::new(600);
    let run2 = RunId::new(601);
    let mut batch = journal.batch();

    batch
        .append_event(&make_event(run1, 0))
        .expect("append run1");
    batch
        .append_event(&make_event(run1, 1))
        .expect("append run1");
    batch
        .append_event(&make_event(run2, 0))
        .expect("append run2");
    batch
        .append_event(&make_event(run2, 1))
        .expect("append run2");

    batch.commit().expect("commit");

    let events1 = journal.events_for_run(run1).expect("replay run1");
    let events2 = journal.events_for_run(run2).expect("replay run2");
    assert_eq!(events1.len(), 2);
    assert_eq!(events2.len(), 2);
}

#[test]
fn cross_keyspace_batch_commit_preserves_all_operations() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(700);
    let mut batch = journal.batch();

    batch.append_event(&make_event(run, 0)).expect("event");
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0xBB; DIGEST_BYTES]),
        status: 1,
        accepted_at_ms: 5000,
    };
    batch.put_run_header(&header).expect("header");
    batch
        .put_status_index(IndexStatusState::Active, 100, run)
        .expect("status index");
    batch
        .put_workflow_index(WorkflowId::new(1), run)
        .expect("workflow index");
    batch
        .put_action_index(vb_core::ActionId::new(1), run, StepIdx::new(0))
        .expect("action index");

    assert_eq!(batch.len(), 5);
    batch.commit().expect("commit");

    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(events.len(), 1);
}

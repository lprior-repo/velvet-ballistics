#![forbid(unsafe_code)]
use super::*;

#[test]
fn batch_strict_mode_commits_successfully() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(500);
    let event = make_event(run, 0);

    let batch = JournalWriteBatch::new(&journal);
    let mut batch = batch.strict();
    batch.append_event(&event).expect("append should succeed");
    batch.commit().expect("strict commit should succeed");

    let events = journal.events_for_run(run).expect("replay should succeed");
    assert_eq!(events.len(), 1);
}

#[test]
fn empty_strict_batch_commit_succeeds() {
    let (_temp, journal) = temp_journal();
    let batch = JournalWriteBatch::new(&journal);
    let batch = batch.strict();
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    batch
        .commit()
        .expect("empty strict batch commit should succeed");
}

use super::support::{Fixture, TestResult, fixture, submission};
use vb_core::{RuntimePolicy, Taint, WorkflowDigest};
use vb_storage::{FjallJournal, JournalError};

fn assert_no_authority(journal: &FjallJournal, f: &Fixture) -> TestResult {
    assert!(journal.run_header(f.run)?.is_none());
    assert!(
        journal
            .owned_run_inputs(f.run, f.artifact.digest, f.source.digest, &f.workflow)?
            .is_none()
    );
    assert!(journal.compiled_ir(f.artifact.digest)?.is_none());
    assert!(journal.workflow_source(f.source.digest)?.is_none());
    assert!(journal.events_for_run(f.run)?.is_empty());
    assert!(
        !journal.has_workflow_index_entry(vb_storage::keys::index_workflow_key(
            vb_core::WorkflowId::new(0),
            f.run
        )?)?
    );
    assert!(
        !journal.has_status_index_entry(vb_storage::keys::index_status_key(
            vb_storage::IndexStatusState::Submitted,
            123,
            f.run
        )?)?
    );
    Ok(())
}

#[test]
fn failed_actual_submission_stages_no_partial_record_or_admission() -> TestResult {
    let mut f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let max = usize::try_from(vb_storage::constants::MAX_WORKFLOW_SOURCE_BYTES)?;
    let over = max.checked_add(1).ok_or("source size overflow")?;
    f.source.source.resize(over, b' ');
    f.source.digest = WorkflowDigest::from_bytes(blake3::hash(&f.source.source).into());
    let prepared = submission(&f)?;
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        assert!(matches!(
            journal.commit_run_submission(&prepared, RuntimePolicy::Strict),
            Err(JournalError::PayloadTooLarge { .. })
        ));
        assert_no_authority(&journal, &f)?;
    }
    let journal = FjallJournal::open(directory.path(), None)?;
    assert_no_authority(&journal, &f)
}

#[test]
fn duplicate_owned_submission_cannot_replace_the_first_binding() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let different = fixture(9, Taint::Clean, Taint::Secret)?;
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    journal.commit_run_submission(&submission(&f)?, RuntimePolicy::Strict)?;
    let before = journal
        .owned_run_inputs(f.run, f.artifact.digest, f.source.digest, &f.workflow)?
        .ok_or("missing owned input")?;
    let events = journal.events_for_run(f.run)?;
    assert!(matches!(
        journal.commit_run_submission(&submission(&different)?, RuntimePolicy::Strict),
        Err(JournalError::RunAlreadyExists)
    ));
    let after = journal
        .owned_run_inputs(f.run, f.artifact.digest, f.source.digest, &f.workflow)?
        .ok_or("owned input replaced")?;
    assert_eq!(before.input_digest(), after.input_digest());
    assert_eq!(before.checkpoint(), after.checkpoint());
    assert_eq!(journal.events_for_run(f.run)?, events);
    Ok(())
}

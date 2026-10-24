use super::support::{TestResult, open, refused};
use std::sync::Arc;
use vb_core::RunId;
use vb_storage::{FjallJournal, OwnedWriterError as Error, OwnedWriterQuarantine as Reason};

#[test]
fn uncertainty_survives_guard_drop_and_never_frees_capacity() -> TestResult {
    let (_directory, journal) = open()?;
    let owner = journal.try_acquire_owned_writer(RunId::new(7), 1)?;
    assert_eq!(
        owner.quarantine(Reason::IndeterminateCommit)?,
        Reason::IndeterminateCommit
    );
    assert_eq!(
        owner.quarantine(Reason::OwnershipUncertain)?,
        Reason::IndeterminateCommit
    );
    let expected = Error::Quarantined {
        run: RunId::new(7),
        reason: Reason::IndeterminateCommit,
    };
    assert_eq!(journal.validate_owned_writer(&owner), Err(expected));
    drop(owner);
    assert_eq!(refused(&journal, 7, 1)?, expected);
    assert_eq!(refused(&journal, 8, 1)?, Error::Full { capacity: 1 });
    Ok(())
}

#[test]
fn explicit_release_cannot_clear_uncertainty() -> TestResult {
    let (_directory, journal) = open()?;
    let owner = journal.try_acquire_owned_writer(RunId::new(5), 1)?;
    assert_eq!(
        owner.quarantine(Reason::OwnershipUncertain)?,
        Reason::OwnershipUncertain
    );
    let expected = Error::Quarantined {
        run: RunId::new(5),
        reason: Reason::OwnershipUncertain,
    };
    assert_eq!(owner.try_release(), Err(expected));
    assert_eq!(refused(&journal, 5, 1)?, expected);
    assert_eq!(refused(&journal, 9, 1)?, Error::Full { capacity: 1 });
    Ok(())
}

#[test]
fn lease_retains_store_and_only_complete_teardown_allows_fresh_registry() -> TestResult {
    let (directory, journal) = open()?;
    let handle = Arc::clone(&journal);
    let owner = journal.try_acquire_owned_writer(RunId::new(9), 1)?;
    owner.quarantine(Reason::IndeterminateCommit)?;
    drop(journal);
    assert_eq!(
        refused(&handle, 9, 1)?,
        Error::Quarantined {
            run: RunId::new(9),
            reason: Reason::IndeterminateCommit,
        }
    );
    drop(handle);
    // The lease is the last Arc until it is dropped; it preserves the old store.
    drop(owner);
    let reopened = Arc::new(FjallJournal::open(directory.path(), None)?);
    reopened.probe_health()?;
    let fresh = reopened.try_acquire_owned_writer(RunId::new(9), 2)?;
    assert_eq!(fresh.generation(), 1);
    reopened.validate_owned_writer(&fresh)?;
    Ok(())
}

use crate::{FjallJournal, OwnedWriterError as Error, OwnedWriterQuarantine as Reason};
use core::num::NonZeroU64;
use std::sync::{Arc, PoisonError, TryLockError};
use vb_core::RunId;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn open() -> TestResult<(tempfile::TempDir, Arc<FjallJournal>)> {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    Ok((directory, journal))
}

fn refused(journal: &Arc<FjallJournal>, run: u64, capacity: usize) -> TestResult<Error> {
    journal
        .try_acquire_owned_writer(RunId::new(run), capacity)
        .err()
        .ok_or_else(|| "unexpected writer acquired".into())
}

#[test]
fn actual_public_acquisition_is_nonblocking_under_real_gate_contention() -> TestResult {
    let (_directory, journal) = open()?;
    let held = journal
        .owned_writers
        .gate
        .try_lock()
        .map_err(|_| "initial gate unavailable")?;
    assert_eq!(refused(&journal, 1, 1)?, Error::Contended);
    drop(held);
    let owner = journal.try_acquire_owned_writer(RunId::new(1), 1)?;
    assert_eq!(owner.generation(), 1);
    Ok(())
}

#[test]
fn uncertainty_publishes_in_store_even_while_drop_cannot_take_gate() -> TestResult {
    let (_directory, journal) = open()?;
    let owner = journal.try_acquire_owned_writer(RunId::new(1), 1)?;
    let held = journal
        .owned_writers
        .gate
        .try_lock()
        .map_err(|_| "gate unavailable")?;
    assert_eq!(
        owner.quarantine(Reason::IndeterminateCommit)?,
        Reason::IndeterminateCommit
    );
    drop(owner);
    drop(held);
    assert_eq!(
        refused(&journal, 1, 1)?,
        Error::Quarantined {
            run: RunId::new(1),
            reason: Reason::IndeterminateCommit,
        }
    );
    assert_eq!(refused(&journal, 2, 1)?, Error::Full { capacity: 1 });
    Ok(())
}

#[test]
fn healthy_drop_and_release_refusals_retain_the_existing_entry() -> TestResult {
    for explicit in [false, true] {
        let (_directory, journal) = open()?;
        let owner = journal.try_acquire_owned_writer(RunId::new(1), 1)?;
        let held = journal
            .owned_writers
            .gate
            .try_lock()
            .map_err(|_| "gate unavailable")?;
        if explicit {
            assert_eq!(owner.try_release(), Err(Error::Contended));
        } else {
            drop(owner);
        }
        drop(held);
        assert_eq!(refused(&journal, 1, 1)?, Error::Busy { run: RunId::new(1) });
        assert_eq!(refused(&journal, 2, 1)?, Error::Full { capacity: 1 });
    }
    Ok(())
}

#[test]
fn stale_and_double_release_cannot_free_the_replacement_owner() -> TestResult {
    let (_directory, journal) = open()?;
    let first = journal.try_acquire_owned_writer(RunId::new(3), 1)?;
    let original_key = first.key;
    first.try_release()?;
    let expected = Error::StaleOwnership { run: RunId::new(3) };
    assert_eq!(journal.owned_writers.release(original_key), Err(expected));
    let replacement = journal.try_acquire_owned_writer(RunId::new(3), 1)?;
    assert_eq!(replacement.generation(), 2);
    assert_eq!(journal.owned_writers.validate(original_key), Err(expected));
    assert_eq!(journal.owned_writers.release(original_key), Err(expected));
    journal.validate_owned_writer(&replacement)?;
    assert_eq!(refused(&journal, 3, 1)?, Error::Busy { run: RunId::new(3) });
    Ok(())
}

#[test]
fn actual_generation_boundary_exhaustion_remains_permanently_closed() -> TestResult {
    let (_directory, journal) = open()?;
    {
        let mut held = journal
            .owned_writers
            .gate
            .try_lock()
            .map_err(|_| "gate unavailable")?;
        held.last = Some(
            u64::MAX
                .checked_sub(1)
                .and_then(NonZeroU64::new)
                .ok_or("generation fixture unavailable")?,
        );
    }
    let last = journal.try_acquire_owned_writer(RunId::new(1), 1)?;
    assert_eq!(last.generation(), u64::MAX);
    last.try_release()?;
    assert_eq!(refused(&journal, 2, 1)?, Error::GenerationExhausted);
    assert_eq!(refused(&journal, 3, 1)?, Error::GenerationExhausted);
    Ok(())
}

#[test]
fn real_reservation_helper_classifies_capacity_overflow_as_allocation_refusal() {
    let result = super::super::table::reserve_entries(usize::MAX);
    assert_eq!(result.err(), Some(Error::AllocationRefused));
}

#[test]
fn actual_guard_poison_result_uses_the_real_public_api_classifier_without_unwind() -> TestResult {
    let (_directory, journal) = open()?;
    let held = journal
        .owned_writers
        .gate
        .try_lock()
        .map_err(|_| "gate unavailable")?;
    let input = Err(TryLockError::Poisoned(PoisonError::new(held)));
    assert_eq!(super::classify_lock(input).err(), Some(Error::Poisoned));
    // Classification only: constructing this result did not poison std::Mutex.
    let owner = journal.try_acquire_owned_writer(RunId::new(1), 1)?;
    journal.validate_owned_writer(&owner)?;
    Ok(())
}

use super::support::{TestResult, open, refused};
use std::sync::Arc;
use vb_core::RunId;
use vb_storage::{MAX_OWNED_WRITERS, OwnedWriterError as Error};

#[test]
fn shared_arc_owners_refuse_duplicate_and_distinct_run_capacity() -> TestResult {
    let (_directory, journal) = open()?;
    let other_handle = Arc::clone(&journal);
    let first = journal.try_acquire_owned_writer(RunId::new(11), 2)?;
    let second = other_handle.try_acquire_owned_writer(RunId::new(22), 2)?;
    assert_eq!(first.run(), RunId::new(11));
    assert_eq!(first.generation(), 1);
    assert_eq!(second.generation(), 2);
    assert_eq!(
        refused(&other_handle, 11, 2)?,
        Error::Busy {
            run: RunId::new(11)
        }
    );
    assert_eq!(refused(&journal, 33, 2)?, Error::Full { capacity: 2 });
    journal.validate_owned_writer(&first)?;
    other_handle.validate_owned_writer(&second)?;
    Ok(())
}

#[test]
fn a_free_hole_never_hides_a_later_duplicate_owner() -> TestResult {
    let (_directory, journal) = open()?;
    let first = journal.try_acquire_owned_writer(RunId::new(1), 3)?;
    let retained = journal.try_acquire_owned_writer(RunId::new(2), 3)?;
    first.try_release()?;
    assert_eq!(refused(&journal, 2, 3)?, Error::Busy { run: RunId::new(2) });
    let fresh = journal.try_acquire_owned_writer(RunId::new(3), 3)?;
    assert_eq!(fresh.generation(), 3);
    journal.validate_owned_writer(&retained)?;
    journal.validate_owned_writer(&fresh)?;
    Ok(())
}

#[test]
fn healthy_drop_and_explicit_release_keep_fresh_checked_generations() -> TestResult {
    let (_directory, journal) = open()?;
    let first = journal.try_acquire_owned_writer(RunId::new(0), 1)?;
    assert_eq!(first.generation(), 1);
    drop(first);
    let second = journal.try_acquire_owned_writer(RunId::new(0), 1)?;
    assert_eq!(second.generation(), 2);
    second.try_release()?;
    let third = journal.try_acquire_owned_writer(RunId::new(0), 1)?;
    assert_eq!(third.generation(), 3);
    journal.validate_owned_writer(&third)?;
    Ok(())
}

#[test]
fn first_capacity_is_nonzero_representable_and_permanently_fixed() -> TestResult {
    let (_directory, journal) = open()?;
    for requested in [0, 65_536, usize::MAX] {
        assert_eq!(
            refused(&journal, 1, requested)?,
            Error::InvalidCapacity {
                requested,
                maximum: 65_535,
            }
        );
    }
    assert_eq!(MAX_OWNED_WRITERS, 65_535);
    let first = journal.try_acquire_owned_writer(RunId::new(1), 2)?;
    first.try_release()?;
    assert_eq!(
        refused(&journal, 2, 1)?,
        Error::CapacityMismatch {
            bound: 2,
            requested: 1
        }
    );
    assert_eq!(
        refused(&journal, 2, 3)?,
        Error::CapacityMismatch {
            bound: 2,
            requested: 3
        }
    );
    let next = journal.try_acquire_owned_writer(RunId::new(2), 2)?;
    assert_eq!(next.generation(), 2);
    Ok(())
}

#[test]
fn equal_run_and_generation_on_another_store_do_not_transfer_authority() -> TestResult {
    let (_directory_a, journal_a) = open()?;
    let (_directory_b, journal_b) = open()?;
    let a = journal_a.try_acquire_owned_writer(RunId::new(1), 1)?;
    let b = journal_b.try_acquire_owned_writer(RunId::new(1), 1)?;
    assert_eq!(a.generation(), b.generation());
    assert_eq!(journal_b.validate_owned_writer(&a), Err(Error::WrongStore));
    assert_eq!(journal_a.validate_owned_writer(&b), Err(Error::WrongStore));
    journal_a.validate_owned_writer(&a)?;
    journal_b.validate_owned_writer(&b)?;
    Ok(())
}

#[test]
fn hard_capacity_boundary_is_supported_by_the_actual_store() -> TestResult {
    let (_directory, journal) = open()?;
    let owner = journal.try_acquire_owned_writer(RunId::new(1), 65_535)?;
    journal.validate_owned_writer(&owner)?;
    owner.try_release()?;
    assert_eq!(
        refused(&journal, 2, 65_534)?,
        Error::CapacityMismatch {
            bound: 65_535,
            requested: 65_534,
        }
    );
    Ok(())
}

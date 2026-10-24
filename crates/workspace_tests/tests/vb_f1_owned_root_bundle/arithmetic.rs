use super::support::TestResult;
use vb_core::root_bundle::{OwnedRootBundleError, checked_arena_count, checked_edge_work};

#[test]
fn arena_count_sums_and_overflows() -> TestResult<()> {
    // Given empty counts
    let counts = [0u64, 0, 0, 0];
    // When summed
    let result = checked_arena_count(counts)?;
    // Then zero
    assert_eq!(result, 0);

    // Given small counts
    let counts = [1u64, 2, 3, 4];
    // When summed
    let result = checked_arena_count(counts)?;
    // Then 10
    assert_eq!(result, 10);

    // Given max in one slot
    let counts = [u64::MAX, 0, 0, 0];
    // When summed
    let result = checked_arena_count(counts)?;
    // Then max
    assert_eq!(result, u64::MAX);

    // Given max plus one
    let counts = [u64::MAX, 1, 0, 0];
    // When summed
    let result = checked_arena_count(counts);
    // Then ArenaCountOverflow
    match result {
        Err(OwnedRootBundleError::ArenaCountOverflow) => {}
        other => {
            return Err(format!("expected ArenaCountOverflow, got {:?}", other).into());
        }
    }

    Ok(())
}

#[test]
fn edge_work_sums_and_exceeds_quota() -> TestResult<()> {
    // Given zero current and additional
    let result = checked_edge_work(0, 0)?;
    assert_eq!(result, 0);

    // Given near quota
    let result = checked_edge_work(999999, 1)?;
    assert_eq!(result, 1000000);

    // Given at quota plus one
    let result = checked_edge_work(1000000, 1);
    match result {
        Err(OwnedRootBundleError::GraphEdgeWorkExceeded { edges, limit }) => {
            assert_eq!(edges, 1000001);
            assert_eq!(limit, 1000000);
        }
        other => {
            return Err(format!("expected GraphEdgeWorkExceeded, got {:?}", other).into());
        }
    }

    Ok(())
}

#[test]
fn edge_work_overflow_precedes_quota() -> TestResult<()> {
    // Given u64::MAX current
    let result = checked_edge_work(u64::MAX, 1);
    // When adding one
    // Then GraphEdgeWorkOverflow (checked-add fires before quota check)
    match result {
        Err(OwnedRootBundleError::GraphEdgeWorkOverflow) => {}
        other => {
            return Err(format!("expected GraphEdgeWorkOverflow, got {:?}", other).into());
        }
    }

    Ok(())
}

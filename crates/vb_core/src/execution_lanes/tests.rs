use super::*;
use crate::ids::StepIdx;

fn make_group(owner: u16) -> GroupId {
    GroupId::new(StepIdx::new(owner), 1).unwrap()
}

fn make_pool(cap: u16, total: u32) -> LanePool {
    LanePool::new(&make_group(0), total, cap, 100, 10).unwrap()
}

#[test]
fn admit_releases_cycle() {
    let mut pool = make_pool(2, 3);

    let t0 = pool.admit().unwrap();
    assert_eq!(t0.item_ordinal(), 0);

    let t1 = pool.admit().unwrap();
    assert_eq!(t1.item_ordinal(), 1);

    // Pool full.
    assert_eq!(pool.admit(), Err(LaneError::Full));

    // Release t0.
    pool.release(t0).unwrap();
    assert_eq!(pool.live_count(), 1);

    // Next admit reuses t0's slot with new generation.
    let t2 = pool.admit().unwrap();
    assert_eq!(t2.item_ordinal(), 2);
    assert_eq!(t2.physical_slot(), t0.physical_slot());
    assert_eq!(
        t2.lease_generation(),
        t0.lease_generation().checked_add(1).unwrap()
    );
}

#[test]
fn stale_token_rejected() {
    let mut pool = make_pool(1, 2);

    let t0 = pool.admit().unwrap();
    pool.release(t0).unwrap();

    // Reuse slot.
    let t1 = pool.admit().unwrap();

    // Old token is stale (different generation).
    assert_eq!(pool.release(t0), Err(LaneError::Stale));

    // Current token works.
    pool.release(t1).unwrap();
}

#[test]
fn foreign_group_rejected() {
    let group1 = make_group(0);
    let group2 = make_group(1);

    let mut pool1 = LanePool::new(&group1, 1, 1, 100, 10).unwrap();
    let t = pool1.admit().unwrap();

    let mut pool2 = LanePool::new(&group2, 1, 1, 100, 10).unwrap();
    assert_eq!(pool2.release(t), Err(LaneError::Foreign));
}

#[test]
fn duplicate_release_rejected() {
    let mut pool = make_pool(2, 2);
    let t0 = pool.admit().unwrap();
    pool.release(t0).unwrap();
    assert_eq!(pool.release(t0), Err(LaneError::DuplicateRelease));
}

#[test]
fn policy_validation() {
    let group = make_group(0);

    assert_eq!(
        LanePool::new(&group, 10, 0, 100, 10),
        Err(LaneError::ZeroCapacity)
    );

    assert_eq!(
        LanePool::new(&group, 10, 20, 100, 10),
        Err(LaneError::CapacityOverPolicy)
    );

    assert_eq!(
        LanePool::new(&group, 200, 5, 100, 10),
        Err(LaneError::TotalOverPolicy)
    );
}

#[test]
fn empty_total_exhausted() {
    let mut pool = make_pool(2, 0);
    assert_eq!(pool.admit(), Err(LaneError::Exhausted));
}

#[test]
fn generation_overflow() {
    let group = make_group(0);
    let mut pool = LanePool::new(&group, 10, 1, 100, 10).unwrap();

    let t = pool.admit().unwrap();
    pool.release(t).unwrap();

    // Force generation to max via private field access (child module).
    pool.slots.first_mut().unwrap().generation = u64::MAX;

    let before = pool.clone();
    assert_eq!(pool.admit(), Err(LaneError::GenerationOverflow));
    assert_eq!(pool, before);
}

#[test]
fn total_exceeds_u16() {
    let group = make_group(0);
    let pool = LanePool::new(&group, 100000, 2, 100000, 10).unwrap();
    assert_eq!(pool.slots.len(), 2);
}

#[test]
fn round_robin_cursor() {
    let mut pool = make_pool(3, 6);

    let t0 = pool.admit().unwrap();
    let t1 = pool.admit().unwrap();
    let t2 = pool.admit().unwrap();

    // All slots taken.
    assert_eq!(pool.admit(), Err(LaneError::Full));

    pool.release(t0).unwrap();
    pool.release(t1).unwrap();
    pool.release(t2).unwrap();

    // After releasing all, cursor is at 0, so next admit uses slot 0.
    let t3 = pool.admit().unwrap();
    assert_eq!(t3.physical_slot(), 0);
}

#[test]
fn zero_group_generation() {
    assert_eq!(
        GroupId::new(StepIdx::new(0), 0),
        Err(LaneError::InvalidGeneration)
    );
}

#[test]
fn cursor_follows_selected_slot_after_skipping_busy_lanes() {
    let mut pool = make_pool(3, 8);
    let first = pool.admit().unwrap();
    let second = pool.admit().unwrap();
    let third = pool.admit().unwrap();
    pool.release(second).unwrap();
    let reused = pool.admit().unwrap();
    assert_eq!(reused.physical_slot(), second.physical_slot());
    pool.release(third).unwrap();
    pool.release(reused).unwrap();
    let next = pool.admit().unwrap();
    assert_eq!(next.physical_slot(), third.physical_slot());
    pool.release(first).unwrap();
}

#[test]
fn rejected_operations_preserve_all_lease_state() {
    let mut pool = make_pool(1, 3);
    let first = pool.admit().unwrap();
    let before = pool.clone();
    assert_eq!(pool.admit(), Err(LaneError::Full));
    assert_eq!(pool, before);
    let mut other = LanePool::new(&make_group(1), 1, 1, 100, 10).unwrap();
    let foreign = other.admit().unwrap();
    assert_eq!(pool.release(foreign), Err(LaneError::Foreign));
    assert_eq!(pool, before);
    pool.release(first).unwrap();
    let released = pool.clone();
    assert_eq!(pool.release(first), Err(LaneError::DuplicateRelease));
    assert_eq!(pool, released);
    let current = pool.admit().unwrap();
    let before = pool.clone();
    assert_eq!(pool.release(first), Err(LaneError::Stale));
    assert_eq!(pool, before);
    pool.release(current).unwrap();
    let last = pool.admit().unwrap();
    let before = pool.clone();
    assert_eq!(pool.admit(), Err(LaneError::Exhausted));
    assert_eq!(pool, before);
    pool.release(last).unwrap();
}

fn expected_snapshot_three_tuples(fixture: &BoundedSnapshotFixture) -> Vec<BoundedSnapshotTuple> {
    vec![
        (
            RunId::new(93_001),
            vb_core::ids::StepIdx::new(2),
            crate::shard::PendingTimerKind::Ask,
            3,
            fixture.second_deadline,
        ),
        (
            RunId::new(93_002),
            vb_core::ids::StepIdx::new(1),
            crate::shard::PendingTimerKind::Wait,
            4,
            fixture.third_deadline,
        ),
        (
            RunId::new(93_003),
            vb_core::ids::StepIdx::new(1),
            crate::shard::PendingTimerKind::Wait,
            2,
            fixture.first_deadline,
        ),
    ]
}

fn assert_snapshot_three_sorted(
    snapshot: &crate::shard::ShardPendingBoundarySnapshot,
    fixture: &BoundedSnapshotFixture,
) -> Result<(), String> {
    assert_eq!(
        bounded_snapshot_tuples(snapshot),
        expected_snapshot_three_tuples(fixture)
    );
    Ok(())
}

#[test]
fn timer_snapshot_limit_zero_returns_empty_truncated_result() -> Result<(), String> {
    let fixture = bounded_snapshot_fixture()?;
    let zero = fixture.shard.pending_boundary_snapshot(0, 0);
    assert_eq!(zero.pending_timer_count(), 3);
    assert!(zero.pending_timers().is_empty());
    assert!(zero.truncated());
    Ok(())
}

#[test]
fn timer_snapshot_limit_one_selects_first_inserted_then_sorts() -> Result<(), String> {
    let fixture = bounded_snapshot_fixture()?;
    let one = fixture.shard.pending_boundary_snapshot(0, 1);
    assert_eq!(one.pending_timer_count(), 3);
    assert_eq!(
        bounded_snapshot_tuples(&one),
        vec![(
            RunId::new(93_003),
            vb_core::ids::StepIdx::new(1),
            crate::shard::PendingTimerKind::Wait,
            2,
            fixture.first_deadline
        )]
    );
    assert!(one.truncated());
    Ok(())
}

#[test]
fn timer_snapshot_limit_two_selects_first_two_before_sorting() -> Result<(), String> {
    let fixture = bounded_snapshot_fixture()?;
    let two = fixture.shard.pending_boundary_snapshot(0, 2);
    assert_eq!(two.pending_timer_count(), 3);
    assert_eq!(
        bounded_snapshot_tuples(&two),
        vec![
            (
                RunId::new(93_001),
                vb_core::ids::StepIdx::new(2),
                crate::shard::PendingTimerKind::Ask,
                3,
                fixture.second_deadline
            ),
            (
                RunId::new(93_003),
                vb_core::ids::StepIdx::new(1),
                crate::shard::PendingTimerKind::Wait,
                2,
                fixture.first_deadline
            ),
        ]
    );
    assert!(two.truncated());
    Ok(())
}

#[test]
fn timer_snapshot_limit_three_returns_all_sorted_exact_fields() -> Result<(), String> {
    let fixture = bounded_snapshot_fixture()?;
    let three = fixture.shard.pending_boundary_snapshot(0, 3);
    assert_eq!(three.pending_timer_count(), 3);
    assert_snapshot_three_sorted(&three, &fixture)?;
    assert!(!three.truncated());
    Ok(())
}

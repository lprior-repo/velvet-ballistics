use super::support::*;

fn succeed(
    g: &mut GroupControl,
    t: LaneToken,
    value: SlotValue,
    taint: Taint,
) -> Result<(), GroupError> {
    complete(g, t, success(value, taint))
}

fn ordered_expected() -> [ResultCell; 3] {
    [
        cell(SlotValue::I64(10), Taint::Clean),
        cell(SlotValue::I64(20), Taint::Secret),
        cell(SlotValue::I64(30), Taint::DerivedFromSecret),
    ]
}

pub(super) fn ordered_reverse() -> Result<(), GroupError> {
    let mut g = group(GroupMode::TogetherOrdered, 3, 3)?;
    let t0 = admit(&mut g)?;
    let t1 = admit(&mut g)?;
    let t2 = admit(&mut g)?;
    succeed(&mut g, t2, SlotValue::I64(30), Taint::DerivedFromSecret)?;
    succeed(&mut g, t1, SlotValue::I64(20), Taint::Secret)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    succeed(&mut g, t0, SlotValue::I64(10), Taint::Clean)?;
    assert_eq!(g.outcome(), GroupOutcome::Ordered(&ordered_expected()));
    Ok(())
}

fn foreach_expected() -> [ResultCell; 3] {
    [
        cell(SlotValue::I64(1), Taint::Clean),
        cell(SlotValue::I64(2), Taint::Secret),
        cell(SlotValue::I64(3), Taint::Clean),
    ]
}

fn check_admit_error(g: &mut GroupControl, error: GroupError) {
    let snapshot = g.clone();
    let result = g.prepare_admit().err();
    assert_eq!(result, Some(error));
    assert_eq!(*g, snapshot);
}

fn check_stale(g: &mut GroupControl, t: LaneToken) {
    let snapshot = g.clone();
    let completion = g.prepare_completion(t, success(SlotValue::Null, Taint::Clean));
    let result = completion.err();
    let expected = GroupError::Lane(LaneError::Stale);
    assert_eq!(result, Some(expected));
    assert_eq!(*g, snapshot);
}

fn finish_foreach(g: &mut GroupControl, t0: LaneToken, t2: LaneToken) -> Result<(), GroupError> {
    succeed(g, t2, SlotValue::I64(3), Taint::Clean)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    succeed(g, t0, SlotValue::I64(1), Taint::Clean)?;
    let expected = foreach_expected();
    assert_eq!(g.outcome(), GroupOutcome::Ordered(&expected));
    Ok(())
}

pub(super) fn foreach_reuse() -> Result<(), GroupError> {
    let mut g = group(GroupMode::ForEachOrdered, 3, 2)?;
    let t0 = admit(&mut g)?;
    let t1 = admit(&mut g)?;
    check_admit_error(&mut g, GroupError::Lane(LaneError::Full));
    succeed(&mut g, t1, SlotValue::I64(2), Taint::Secret)?;
    let t2 = admit(&mut g)?;
    assert_eq!(t2.physical_slot(), t1.physical_slot());
    assert!(t2.lease_generation() > t1.lease_generation());
    check_admit_error(&mut g, GroupError::Lane(LaneError::Exhausted));
    check_stale(&mut g, t1);
    finish_foreach(&mut g, t0, t2)?;
    Ok(())
}

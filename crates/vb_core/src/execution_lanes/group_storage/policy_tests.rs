fn reject_short_buffer(which: u8) -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(3, 2)?;
    let r = match &mut s.state {
        State::Vacant(r) => r,
        _ => return Err(GroupError::InvalidTransition),
    };
    match which {
        0 => r.slots = Vec::new(),
        1 => r.results = Vec::new(),
        2 => r.lanes = Vec::new(),
        _ => return Err(GroupError::InvalidTransition),
    }
    reject_activation(
        &mut s,
        ActivationAttempt {
            mode: GroupMode::ForEachOrdered,
            group: id(1)?,
            total_items: 3,
            declared_cap: 2,
        },
        GroupError::InsufficientCapacity,
    )?;
    Ok(())
}

#[test]
fn each_reserved_buffer_is_guarded() -> Result<(), GroupError> {
    reject_short_buffer(0)?;
    reject_short_buffer(1)?;
    reject_short_buffer(2)?;
    Ok(())
}

fn ordered_cells() -> [ResultCell; 3] {
    [
        ResultCell::Value {
            value: SlotValue::I64(10),
            taint: Taint::Clean,
        },
        ResultCell::Value {
            value: SlotValue::I64(20),
            taint: Taint::Secret,
        },
        ResultCell::Value {
            value: SlotValue::I64(30),
            taint: Taint::DerivedFromSecret,
        },
    ]
}

fn reject_policy_case(
    mode: GroupMode,
    total: u32,
    cap: u16,
    error: GroupError,
) -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(3, 2)?;
    reject_activation(
        &mut s,
        ActivationAttempt {
            mode: mode,
            group: id(1)?,
            total_items: total,
            declared_cap: cap,
        },
        error,
    )?;
    s.activate(GroupMode::ForEachOrdered, id(1)?, 3, 2)?;
    let c = control(&mut s)?;
    assert_eq!(c.phase(), GroupPhase::Running);
    assert_eq!(c.revision(), 0);
    assert_eq!(c.completed, 0);
    assert_eq!(c.results.as_slice(), &[ResultCell::Pending; 3]);
    Ok(())
}

#[test]
fn policy_lane_bounds_are_atomic() -> Result<(), GroupError> {
    use GroupError::Lane;
    use GroupMode::ForEachOrdered;
    use LaneError::{CapacityOverPolicy, TotalOverPolicy, ZeroCapacity};
    reject_policy_case(ForEachOrdered, 3, 0, Lane(ZeroCapacity))?;
    reject_policy_case(ForEachOrdered, 3, 3, Lane(CapacityOverPolicy))?;
    reject_policy_case(ForEachOrdered, 4, 2, Lane(TotalOverPolicy))?;
    Ok(())
}

#[test]
fn non_foreach_empty_groups_are_atomic() -> Result<(), GroupError> {
    reject_policy_case(GroupMode::TogetherOrdered, 0, 1, GroupError::EmptyGroup)?;
    reject_policy_case(GroupMode::FirstSuccess, 0, 1, GroupError::EmptyGroup)?;
    Ok(())
}

#[test]
fn insufficient_non_foreach_capacity_is_atomic() -> Result<(), GroupError> {
    reject_policy_case(
        GroupMode::TogetherOrdered,
        3,
        2,
        GroupError::InsufficientCapacity,
    )?;
    reject_policy_case(
        GroupMode::FirstSuccess,
        3,
        2,
        GroupError::InsufficientCapacity,
    )?;
    Ok(())
}

fn reject_generation(s: &mut GroupStorage, generation: u64) -> Result<(), GroupError> {
    reject_activation(
        s,
        ActivationAttempt {
            mode: GroupMode::ForEachOrdered,
            group: id(generation)?,
            total_items: 0,
            declared_cap: 1,
        },
        GroupError::Lane(LaneError::InvalidGeneration),
    )
}

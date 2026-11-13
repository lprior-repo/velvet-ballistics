#[test]
fn ordered_reuse_preserves_buffers_and_resets_control() -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(3, 2)?;
    let buffers = buffer_identity(&s);
    s.activate(GroupMode::ForEachOrdered, id(1)?, 3, 2)?;
    assert_eq!(buffer_identity(&s), buffers);
    let old = finish_three(&mut control(&mut s)?)?;
    assert_eq!(buffer_identity(&s), buffers);
    s.retire()?;
    assert_eq!(buffer_identity(&s), buffers);
    assert!(s.active().is_none());
    s.activate(GroupMode::ForEachOrdered, id(2)?, 1, 1)?;
    assert_eq!(buffer_identity(&s), buffers);
    let mut g = control(&mut s)?;
    assert_eq!(g.group, id(2)?);
    assert_eq!(g.revision(), 0);
    assert_eq!(g.completed, 0);
    assert!(g.chosen_winner().is_none());
    assert_eq!(g.results.as_slice(), &[ResultCell::Pending]);
    reject_old_token(&mut g, old)?;
    finish_null(&mut g)?;
    s.retire()?;
    assert_eq!(buffer_identity(&s), buffers);
    Ok(())
}

fn reject_retire(s: &mut GroupStorage) -> Result<(), GroupError> {
    let before = format!("{s:?}");
    let buffers = buffer_identity(s);
    let control_before = s.active().cloned();
    assert_eq!(s.retire(), Err(GroupError::InvalidTransition));
    assert_eq!(format!("{s:?}"), before);
    assert_eq!(buffer_identity(s), buffers);
    assert_eq!(s.active(), control_before.as_ref());
    Ok(())
}

#[test]
fn active_reactivation_is_atomic() -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(3, 2)?;
    s.activate(GroupMode::ForEachOrdered, id(1)?, 3, 2)?;
    let before = control(&mut s)?.clone();
    reject_activation(
        &mut s,
        ActivationAttempt {
            mode: GroupMode::ForEachOrdered,
            group: id(2)?,
            total_items: 1,
            declared_cap: 1,
        },
        GroupError::InvalidTransition,
    )?;
    assert_eq!(s.active(), Some(&before));
    reject_retire(&mut s)?;
    Ok(())
}

fn choose_winner(s: &mut GroupStorage) -> Result<(LaneToken, Winner), GroupError> {
    s.activate(GroupMode::FirstSuccess, id(1)?, 2, 2)?;
    let mut g = control(s)?;
    let loser = admit(&mut g)?;
    let token = admit(&mut g)?;
    assert_eq!(loser.item_ordinal(), 0);
    assert_eq!(token.item_ordinal(), 1);
    complete(&mut g, token, SlotValue::I64(77), Taint::Secret)?;
    let winner = Winner {
        token,
        value: SlotValue::I64(77),
        taint: Taint::Secret,
    };
    assert_eq!(g.chosen_winner(), Some(winner));
    assert_eq!(g.phase(), GroupPhase::Draining(Termination::Winner(winner)));
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    assert_eq!(g.live_count(), 1);
    Ok((loser, winner))
}

#[test]
fn first_success_waits_for_actual_loser_drain() -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(3, 2)?;
    let (loser, winner) = choose_winner(&mut s)?;
    reject_retire(&mut s)?;
    let mut g = control(&mut s)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    let before = GroupControl::clone(&g);
    let prepared = g.prepare_drain(loser)?;
    drop(prepared);
    assert_eq!(*g, before);
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    let result = commit(g.prepare_drain(loser)?)?;
    assert_eq!(
        result,
        GroupTransition::Changed(GroupPhase::Terminal(Termination::Winner(winner)))
    );
    assert_eq!(g.outcome(), GroupOutcome::Winner(winner));
    assert_eq!(g.live_count(), 0);
    s.retire()?;
    assert!(s.active().is_none());
    Ok(())
}

fn empty_generation(s: &mut GroupStorage, generation: u64) -> Result<(), GroupError> {
    s.activate(GroupMode::ForEachOrdered, id(generation)?, 0, 1)?;
    let c = control(s)?;
    assert_eq!(c.group, id(generation)?);
    assert_eq!(c.phase(), GroupPhase::Terminal(Termination::Ordered));
    assert_eq!(c.revision(), 0);
    assert_eq!(c.live_count(), 0);
    s.retire()?;
    assert!(s.active().is_none());
    Ok(())
}

#[test]
fn generations_are_strictly_increasing() -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(0, 1)?;
    let b = buffer_identity(&s);
    empty_generation(&mut s, 1)?;
    assert_eq!(buffer_identity(&s), b);
    empty_generation(&mut s, 2)?;
    assert_eq!(buffer_identity(&s), b);
    reject_generation(&mut s, 1)?;
    reject_generation(&mut s, 2)?;
    empty_generation(&mut s, 3)?;
    assert_eq!(buffer_identity(&s), b);
    Ok(())
}

#[test]
fn maximal_generation_retires_then_exhausts() -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(0, 1)?;
    empty_generation(&mut s, u64::MAX)?;
    reject_activation(
        &mut s,
        ActivationAttempt {
            mode: GroupMode::ForEachOrdered,
            group: id(u64::MAX)?,
            total_items: 0,
            declared_cap: 1,
        },
        GroupError::Lane(LaneError::GenerationOverflow),
    )?;
    Ok(())
}

#[test]
fn zero_reserved_lanes_are_rejected() -> Result<(), GroupError> {
    assert_eq!(
        GroupStorage::reserve(3, 0),
        Err(GroupError::Lane(LaneError::ZeroCapacity))
    );
    Ok(())
}

#[test]
fn dropping_facade_preserves_live_owner() -> Result<(), GroupError> {
    let mut s = GroupStorage::reserve(1, 1)?;
    s.activate(GroupMode::ForEachOrdered, id(1)?, 1, 1)?;
    let token = {
        let mut active = control(&mut s)?;
        admit(&mut active)?
    };
    assert_eq!(s.active().map(GroupControl::live_count), Some(1));
    reject_retire(&mut s)?;
    complete(
        &mut control(&mut s)?,
        token,
        SlotValue::I64(7),
        Taint::Secret,
    )?;
    s.retire()?;
    assert!(s.active().is_none());
    Ok(())
}

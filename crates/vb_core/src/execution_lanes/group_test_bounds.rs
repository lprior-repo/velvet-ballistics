use super::support::*;

fn make_group(
    mode: GroupMode,
    generation: u64,
    total: u32,
    cap: u16,
) -> Result<GroupControl, GroupError> {
    let id = GroupId::new(StepIdx::new(0), generation)?;
    GroupControl::new(mode, id, total, cap, 100, 10)
}

pub(super) fn events_for_wrong_generation_are_rejected() -> Result<(), GroupError> {
    let mut g1 = make_group(GroupMode::ForEachOrdered, 1, 1, 1)?;
    let a = admit(&mut g1)?;
    let mut g2 = make_group(GroupMode::ForEachOrdered, 2, 1, 1)?;
    let b = admit(&mut g2)?;
    let snapshot = g1.clone();
    let r = g1.prepare_completion(b, LaneCompletion::Failed);
    assert_eq!(r.err(), Some(GroupError::Lane(LaneError::Foreign)));
    assert_eq!(g1, snapshot);
    complete(&mut g1, a, success(SlotValue::Null, Taint::Clean))?;
    assert_single_null(&g1)?;
    Ok(())
}

pub(super) fn constructor_errors_and_revision_overflow() -> Result<(), GroupError> {
    constructor_empty()?;
    constructor_policy()?;
    defensive_admit_overflow()?;
    defensive_completion_abort_drain()?;
    Ok(())
}

fn constructor_empty() -> Result<(), GroupError> {
    let empty = make_group(GroupMode::ForEachOrdered, 1, 0, 1)?;
    assert_eq!(empty.outcome(), GroupOutcome::Ordered(&[]));
    let e1 = make_group(GroupMode::TogetherOrdered, 1, 0, 1);
    assert_eq!(e1.err(), Some(GroupError::EmptyGroup));
    let e2 = make_group(GroupMode::FirstSuccess, 1, 0, 1);
    assert_eq!(e2.err(), Some(GroupError::EmptyGroup));
    Ok(())
}

fn constructor_policy() -> Result<(), GroupError> {
    let p1 = make_group(GroupMode::ForEachOrdered, 1, 1, 0);
    assert_eq!(p1.err(), Some(GroupError::Lane(LaneError::ZeroCapacity)));
    let p2 = make_group(GroupMode::ForEachOrdered, 1, 1, 11);
    assert_eq!(
        p2.err(),
        Some(GroupError::Lane(LaneError::CapacityOverPolicy))
    );
    let p3 = make_group(GroupMode::ForEachOrdered, 1, 101, 1);
    assert_eq!(p3.err(), Some(GroupError::Lane(LaneError::TotalOverPolicy)));
    let p4 = make_group(GroupMode::TogetherOrdered, 1, 2, 1);
    assert_eq!(p4.err(), Some(GroupError::InsufficientCapacity));
    let p5 = make_group(GroupMode::FirstSuccess, 1, 2, 1);
    assert_eq!(p5.err(), Some(GroupError::InsufficientCapacity));
    Ok(())
}

fn defensive_admit_overflow() -> Result<(), GroupError> {
    let mut g = make_group(GroupMode::FirstSuccess, 1, 2, 2)?;
    // DEFENSIVE injected state, not public path proof
    g.revision = u64::MAX;
    let snapshot = g.clone();
    let r = g.prepare_admit();
    assert_eq!(r.err(), Some(GroupError::RevisionOverflow));
    assert_eq!(g, snapshot);
    Ok(())
}

fn defensive_completion_abort_drain() -> Result<(), GroupError> {
    let mut g = make_group(GroupMode::FirstSuccess, 1, 2, 2)?;
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;
    // DEFENSIVE injected state, not public path proof
    g.revision = u64::MAX;
    check_completion_abort_max(&mut g, a)?;
    // DEFENSIVE injected state, not public path proof
    g.revision = 2;
    complete(&mut g, a, success(SlotValue::Null, Taint::Clean))?;
    defensive_drain_overflow(&mut g, b)?;
    Ok(())
}

fn check_completion_abort_max(g: &mut GroupControl, token: LaneToken) -> Result<(), GroupError> {
    let snapshot = g.clone();
    let r1 = g.prepare_completion(token, LaneCompletion::Failed);
    assert_eq!(r1.err(), Some(GroupError::RevisionOverflow));
    assert_eq!(*g, snapshot);
    let r2 = g.prepare_abort(AbortReason::Canceled);
    assert_eq!(r2.err(), Some(GroupError::RevisionOverflow));
    assert_eq!(*g, snapshot);
    Ok(())
}

fn defensive_drain_overflow(g: &mut GroupControl, token: LaneToken) -> Result<(), GroupError> {
    // DEFENSIVE injected state, not public path proof
    g.revision = u64::MAX;
    let snapshot = g.clone();
    let r = g.prepare_drain(token);
    assert_eq!(r.err(), Some(GroupError::RevisionOverflow));
    assert_eq!(*g, snapshot);
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    Ok(())
}

#[test]
fn repeated_abort_advances_draining_revision() -> Result<(), GroupError> {
    let mut g = make_group(GroupMode::FirstSuccess, 1, 1, 1)?;
    let token = admit(&mut g)?;
    assert_eq!(g.revision(), 1);
    abort_at(&mut g, AbortReason::Canceled, 2, GroupFailure::Canceled)?;
    abort_at(
        &mut g,
        AbortReason::ParentFailed,
        3,
        GroupFailure::ParentFailed,
    )?;
    abort_at(&mut g, AbortReason::Canceled, 4, GroupFailure::Canceled)?;
    finish_abort_drain(&mut g, token)?;
    Ok(())
}

fn finish_abort_drain(g: &mut GroupControl, token: LaneToken) -> Result<(), GroupError> {
    let p = g.prepare_drain(token)?;
    let ev = p.event();
    assert_eq!(ev.revision, 5);
    assert_eq!(ev.kind, GroupEventKind::Drained(token));
    let expected = GroupPhase::Terminal(Termination::Failed(GroupFailure::Canceled));
    let receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(p.commit(receipt), Ok(GroupTransition::Changed(expected)));
    assert_eq!(g.revision(), 5);
    assert_eq!(g.outcome(), GroupOutcome::Failed(GroupFailure::Canceled));
    assert_eq!(g.live_count(), 0);
    Ok(())
}

fn abort_at(
    g: &mut GroupControl,
    reason: AbortReason,
    revision: u64,
    failure: GroupFailure,
) -> Result<(), GroupError> {
    let p = g.prepare_abort(reason)?;
    let ev = p.event();
    assert_eq!(ev.revision, revision);
    assert_eq!(ev.kind, GroupEventKind::Aborted(reason));
    let expected = GroupPhase::Draining(Termination::Failed(failure));
    let receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(p.commit(receipt), Ok(GroupTransition::Changed(expected)));
    assert_eq!(g.revision(), revision);
    assert_eq!(g.live_count(), 1);
    Ok(())
}
fn assert_single_null(g: &GroupControl) -> Result<(), GroupError> {
    match g.outcome() {
        GroupOutcome::Ordered(cells) => {
            assert_eq!(cells.len(), 1);
            let first = cells.first().ok_or(GroupError::InvalidTransition)?;
            assert_eq!(*first, cell(SlotValue::Null, Taint::Clean));
        }
        _ => return Err(GroupError::InvalidTransition),
    }
    Ok(())
}

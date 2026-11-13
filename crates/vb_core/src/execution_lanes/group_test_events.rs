use super::support::*;

fn check_event(actual: GroupEvent, revision: u64, kind: GroupEventKind) -> Result<(), GroupError> {
    let expected_group = GroupId::new(StepIdx::new(0), 1)?;
    let expected = GroupEvent {
        group: expected_group,
        revision,
        kind,
    };
    assert_eq!(actual, expected);
    Ok(())
}

fn assert_token(token: LaneToken, slot: u16, ordinal: u32) -> Result<(), GroupError> {
    let expected_group = GroupId::new(StepIdx::new(0), 1)?;
    assert_eq!(token.group(), expected_group);
    assert_eq!(token.physical_slot(), slot);
    assert_eq!(token.lease_generation(), 1);
    assert_eq!(token.item_ordinal(), ordinal);
    Ok(())
}

fn admit_literal(
    g: &mut GroupControl,
    revision: u64,
    slot: u16,
    ordinal: u32,
) -> Result<LaneToken, GroupError> {
    let p = g.prepare_admit()?;
    let ev = p.event();
    let token = match ev.kind {
        GroupEventKind::Admission(t) => t,
        _ => return Err(GroupError::InvalidTransition),
    };
    assert_token(token, slot, ordinal)?;
    check_event(ev, revision, GroupEventKind::Admission(token))?;
    let receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(p.commit(receipt), Ok(GroupTransition::Admitted(token)));
    assert_eq!(g.revision(), revision);
    Ok(token)
}

fn drop_admission(g: &mut GroupControl) -> Result<(), GroupError> {
    let snapshot = g.clone();
    let p = g.prepare_admit()?;
    drop(p);
    assert_eq!(*g, snapshot);
    assert_eq!(g.revision(), 0);
    assert_eq!(g.phase(), GroupPhase::Running);
    Ok(())
}

fn literal_completion(g: &mut GroupControl, first: LaneToken) -> Result<(), GroupError> {
    let outcome = success(SlotValue::I64(5), Taint::Secret);
    let p = g.prepare_completion(first, outcome)?;
    let ev = p.event();
    check_event(
        ev,
        3,
        GroupEventKind::Completion {
            token: first,
            outcome,
        },
    )?;
    let receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(
        p.commit(receipt),
        Ok(GroupTransition::Changed(GroupPhase::Running))
    );
    assert_eq!(g.revision(), 3);
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    Ok(())
}

fn literal_abort(g: &mut GroupControl) -> Result<(), GroupError> {
    let p = g.prepare_abort(AbortReason::Canceled)?;
    let ev = p.event();
    check_event(ev, 4, GroupEventKind::Aborted(AbortReason::Canceled))?;
    let expected_phase = GroupPhase::Draining(Termination::Failed(GroupFailure::Canceled));
    let receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(
        p.commit(receipt),
        Ok(GroupTransition::Changed(expected_phase))
    );
    assert_eq!(g.revision(), 4);
    let snapshot = g.clone();
    assert_eq!(g.prepare_admit().err(), Some(GroupError::InvalidTransition));
    assert_eq!(*g, snapshot);
    Ok(())
}

fn drop_drain(g: &mut GroupControl, second: LaneToken) -> Result<(), GroupError> {
    let snapshot = g.clone();
    let p = g.prepare_drain(second)?;
    let ev = p.event();
    check_event(ev, 5, GroupEventKind::Drained(second))?;
    drop(p);
    assert_eq!(*g, snapshot);
    assert_eq!(g.revision(), 4);
    Ok(())
}

fn commit_drain(g: &mut GroupControl, second: LaneToken) -> Result<(), GroupError> {
    let p = g.prepare_drain(second)?;
    let ev = p.event();
    check_event(ev, 5, GroupEventKind::Drained(second))?;
    let expected_phase = GroupPhase::Terminal(Termination::Failed(GroupFailure::Canceled));
    let receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(
        p.commit(receipt),
        Ok(GroupTransition::Changed(expected_phase))
    );
    assert_eq!(g.revision(), 5);
    assert_eq!(g.outcome(), GroupOutcome::Failed(GroupFailure::Canceled));
    assert_eq!(g.live_count(), 0);
    Ok(())
}

#[test]
fn literal_event_sequence() -> Result<(), GroupError> {
    let mut g = group(GroupMode::TogetherOrdered, 2, 2)?;
    assert_eq!(g.revision(), 0);
    assert_eq!(g.phase(), GroupPhase::Running);
    drop_admission(&mut g)?;
    let first = admit_literal(&mut g, 1, 0, 0)?;
    let second = admit_literal(&mut g, 2, 1, 1)?;
    literal_completion(&mut g, first)?;
    literal_abort(&mut g)?;
    drop_drain(&mut g, second)?;
    commit_drain(&mut g, second)?;
    Ok(())
}

pub(super) fn admission_event_identity_and_stale_receipt() -> Result<(), GroupError> {
    let mut g = group(GroupMode::ForEachOrdered, 2, 1)?;
    let p = g.prepare_admit()?;
    let ev = p.event();
    let token = match ev.kind {
        GroupEventKind::Admission(t) => t,
        _ => return Err(GroupError::InvalidTransition),
    };
    let old_receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(p.commit(old_receipt), Ok(GroupTransition::Admitted(token)));
    complete(&mut g, token, success(SlotValue::Null, Taint::Clean))?;
    let snapshot = g.clone();
    let p2 = g.prepare_admit()?;
    let old_receipt = GroupCommitReceipt::from_committed_event(ev);
    assert_eq!(p2.commit(old_receipt), Err(GroupError::InvalidReceipt));
    assert_eq!(g, snapshot);
    admit(&mut g)?;
    Ok(())
}

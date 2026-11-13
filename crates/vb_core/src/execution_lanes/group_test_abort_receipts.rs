use super::support::*;

fn fixture() -> Result<(GroupControl, LaneToken, LaneToken), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 2, 2)?;
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;
    Ok((g, a, b))
}

fn finish_valid(g: &mut GroupControl, a: LaneToken, b: LaneToken) -> Result<(), GroupError> {
    let p = g.prepare_abort(AbortReason::Canceled)?;
    let receipt = GroupCommitReceipt::from_committed_event(p.event());
    let expected = GroupPhase::Draining(Termination::Failed(GroupFailure::Canceled));
    assert_eq!(p.commit(receipt), Ok(GroupTransition::Changed(expected)));
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    drain(g, a)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    drain(g, b)?;
    assert_eq!(g.outcome(), GroupOutcome::Failed(GroupFailure::Canceled));
    assert_eq!(g.live_count(), 0);
    Ok(())
}

#[test]
fn abort_reason_only_receipt() -> Result<(), GroupError> {
    let (mut g, a, b) = fixture()?;
    let snapshot = g.clone();
    let p = g.prepare_abort(AbortReason::Canceled)?;
    let actual = p.event();
    let bad = GroupEvent {
        kind: GroupEventKind::Aborted(AbortReason::ParentFailed),
        ..actual
    };
    assert_eq!(
        p.commit(GroupCommitReceipt::from_committed_event(bad)),
        Err(GroupError::InvalidReceipt)
    );
    assert_eq!(g, snapshot);
    finish_valid(&mut g, a, b)
}

#[test]
fn abort_revision_only_receipt() -> Result<(), GroupError> {
    let (mut g, a, b) = fixture()?;
    let snapshot = g.clone();
    let p = g.prepare_abort(AbortReason::Canceled)?;
    let actual = p.event();
    let bad = GroupEvent {
        revision: actual
            .revision
            .checked_add(1)
            .ok_or(GroupError::RevisionOverflow)?,
        ..actual
    };
    assert_eq!(
        p.commit(GroupCommitReceipt::from_committed_event(bad)),
        Err(GroupError::InvalidReceipt)
    );
    assert_eq!(g, snapshot);
    finish_valid(&mut g, a, b)
}

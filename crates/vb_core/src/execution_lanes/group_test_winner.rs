use super::support::*;

fn fixture() -> Result<(GroupControl, LaneToken, LaneToken, Winner), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 3, 3)?;
    let a = admit(&mut g)?;
    let w = admit(&mut g)?;
    let b = admit(&mut g)?;
    let expected = Winner {
        token: w,
        value: SlotValue::I64(7),
        taint: Taint::Secret,
    };
    complete(&mut g, w, success(SlotValue::I64(7), Taint::Secret))?;
    Ok((g, a, b, expected))
}

fn assert_pending(g: &GroupControl, w: Winner, live: usize) {
    assert_eq!(g.chosen_winner(), Some(w));
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    assert_eq!(g.live_count(), live);
}

fn reject_late(g: &mut GroupControl, t: LaneToken) {
    let snapshot = g.clone();
    let completion = g.prepare_completion(t, success(SlotValue::I64(99), Taint::Clean));
    let err = completion.err();
    assert_eq!(err, Some(GroupError::InvalidTransition));
    assert_eq!(*g, snapshot);
}

#[test]
fn secret_winner_survives_every_drain() -> Result<(), GroupError> {
    let (mut g, a, b, expected) = fixture()?;
    assert_pending(&g, expected, 2);
    reject_late(&mut g, b);
    drain(&mut g, a)?;
    assert_pending(&g, expected, 1);
    drain(&mut g, b)?;
    assert_eq!(g.chosen_winner(), Some(expected));
    assert_eq!(g.outcome(), GroupOutcome::Winner(expected));
    assert_eq!(g.live_count(), 0);
    reject_late(&mut g, b);
    Ok(())
}

fn override_abort(g: &mut GroupControl) -> Result<(), GroupError> {
    let p = g.prepare_abort(AbortReason::ParentFailed)?;
    let receipt = GroupCommitReceipt::from_committed_event(p.event());
    let t = p.commit(receipt)?;
    assert_eq!(
        t,
        GroupTransition::Changed(GroupPhase::Draining(Termination::Failed(
            GroupFailure::ParentFailed
        )))
    );
    Ok(())
}

#[test]
fn secret_winner_audit_survives_abort_and_every_drain() -> Result<(), GroupError> {
    let (mut g, a, b, expected) = fixture()?;
    assert_pending(&g, expected, 2);
    override_abort(&mut g)?;
    assert_pending(&g, expected, 2);
    reject_late(&mut g, b);
    drain(&mut g, a)?;
    assert_pending(&g, expected, 1);
    drain(&mut g, b)?;
    assert_eq!(g.chosen_winner(), Some(expected));
    assert_eq!(
        g.outcome(),
        GroupOutcome::Failed(GroupFailure::ParentFailed)
    );
    assert_eq!(g.live_count(), 0);
    reject_late(&mut g, b);
    Ok(())
}

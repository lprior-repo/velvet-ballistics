use super::support::*;

pub(super) fn wrong_receipts_are_rejected_and_group_is_unchanged() -> Result<(), GroupError> {
    let mut g = group(GroupMode::TogetherOrdered, 2, 2)?;
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;

    reject_wrong_revision(&mut g, a)?;
    reject_failed_outcome(&mut g, a)?;
    reject_wrong_group(&mut g, a)?;

    complete(&mut g, a, success(SlotValue::I64(1), Taint::Clean))?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    complete(&mut g, b, success(SlotValue::Null, Taint::Clean))?;

    let expected = vec![
        cell(SlotValue::I64(1), Taint::Clean),
        cell(SlotValue::Null, Taint::Clean),
    ];
    assert_eq!(g.outcome(), GroupOutcome::Ordered(&expected));

    check_admission_receipt()?;
    check_drain_receipt()?;
    Ok(())
}

fn reject_wrong_revision(g: &mut GroupControl, a: LaneToken) -> Result<(), GroupError> {
    let snap = g.clone();
    let p = g.prepare_completion(a, success(SlotValue::I64(1), Taint::Clean))?;
    let bad = bump(p.event())?;
    assert_eq!(
        p.commit(receipt_from_bad(bad)),
        Err(GroupError::InvalidReceipt)
    );
    assert_eq!(*g, snap);
    Ok(())
}

fn reject_failed_outcome(g: &mut GroupControl, a: LaneToken) -> Result<(), GroupError> {
    let snap = g.clone();
    let p = g.prepare_completion(a, success(SlotValue::I64(1), Taint::Clean))?;
    let ev = p.event();
    let bad = GroupEvent {
        group: ev.group,
        revision: ev.revision,
        kind: GroupEventKind::Completion {
            token: a,
            outcome: LaneCompletion::Failed,
        },
    };
    assert_eq!(
        p.commit(receipt_from_bad(bad)),
        Err(GroupError::InvalidReceipt)
    );
    assert_eq!(*g, snap);
    Ok(())
}

fn reject_wrong_group(g: &mut GroupControl, a: LaneToken) -> Result<(), GroupError> {
    let snap = g.clone();
    let p = g.prepare_completion(a, success(SlotValue::I64(1), Taint::Clean))?;
    let ev = p.event();
    let bad = GroupEvent {
        group: GroupId::new(StepIdx::new(0), 99)?,
        revision: ev.revision,
        kind: ev.kind,
    };
    assert_eq!(
        p.commit(receipt_from_bad(bad)),
        Err(GroupError::InvalidReceipt)
    );
    assert_eq!(*g, snap);
    Ok(())
}

fn reject_drain_receipt(g: &mut GroupControl, loser: LaneToken) -> Result<(), GroupError> {
    let snap = g.clone();
    let p = g.prepare_drain(loser)?;
    let bad = bump(p.event())?;
    assert_eq!(
        p.commit(receipt_from_bad(bad)),
        Err(GroupError::InvalidReceipt)
    );
    assert_eq!(*g, snap);
    Ok(())
}

fn check_admission_receipt() -> Result<(), GroupError> {
    let mut g = group(GroupMode::TogetherOrdered, 1, 1)?;
    let snap = g.clone();
    let p = g.prepare_admit()?;
    let bad = bump(p.event())?;
    let err = p.commit(receipt_from_bad(bad));
    assert!(err.is_err());
    if let Err(e) = err {
        assert_eq!(e, GroupError::InvalidReceipt);
    }
    assert_eq!(g, snap);
    admit(&mut g)?;
    Ok(())
}

fn check_drain_receipt() -> Result<(), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 2, 2)?;
    let loser = admit(&mut g)?;
    let winner = admit(&mut g)?;
    complete(&mut g, winner, success(SlotValue::Null, Taint::Clean))?;
    reject_drain_receipt(&mut g, loser)?;
    drain(&mut g, loser)?;
    let expected = Winner {
        token: winner,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    assert_eq!(g.outcome(), GroupOutcome::Winner(expected));
    Ok(())
}

fn bump(ev: GroupEvent) -> Result<GroupEvent, GroupError> {
    Ok(GroupEvent {
        revision: ev
            .revision
            .checked_add(1)
            .ok_or(GroupError::RevisionOverflow)?,
        ..ev
    })
}

fn receipt_from_bad(ev: GroupEvent) -> GroupCommitReceipt {
    GroupCommitReceipt::from_committed_event(ev)
}

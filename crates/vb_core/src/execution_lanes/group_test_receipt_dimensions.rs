use super::support::*;

fn fixture() -> Result<(GroupControl, LaneToken, LaneToken), GroupError> {
    let mut g = group(GroupMode::TogetherOrdered, 2, 2)?;
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;
    Ok((g, a, b))
}

fn reject_and_continue(
    g: &mut GroupControl,
    a: LaneToken,
    b: LaneToken,
    bad_kind: GroupEventKind,
) -> Result<(), GroupError> {
    let snapshot = g.clone();
    let prepared = g.prepare_completion(a, success(SlotValue::I64(1), Taint::Clean))?;
    let actual = prepared.event();
    let bad = GroupEvent {
        kind: bad_kind,
        ..actual
    };
    let err = prepared.commit(GroupCommitReceipt::from_committed_event(bad));
    if err != Err(GroupError::InvalidReceipt) {
        return Err(GroupError::InvalidReceipt);
    }
    if *g != snapshot {
        return Err(GroupError::InvalidReceipt);
    }
    finish_valid(g, a, b)?;
    Ok(())
}

fn finish_valid(g: &mut GroupControl, a: LaneToken, b: LaneToken) -> Result<(), GroupError> {
    complete(g, a, success(SlotValue::I64(1), Taint::Clean))?;
    if !matches!(g.outcome(), GroupOutcome::Pending) {
        return Err(GroupError::InvalidTransition);
    }
    complete(g, b, success(SlotValue::Null, Taint::Clean))?;
    let expected = [
        cell(SlotValue::I64(1), Taint::Clean),
        cell(SlotValue::Null, Taint::Clean),
    ];
    if g.outcome() != GroupOutcome::Ordered(&expected) {
        return Err(GroupError::InvalidTransition);
    }
    Ok(())
}

#[test]
fn value_only_receipt() -> Result<(), GroupError> {
    let (mut g, a, b) = fixture()?;
    let bad = GroupEventKind::Completion {
        token: a,
        outcome: success(SlotValue::I64(2), Taint::Clean),
    };
    reject_and_continue(&mut g, a, b, bad)
}

#[test]
fn taint_only_receipt() -> Result<(), GroupError> {
    let (mut g, a, b) = fixture()?;
    let bad = GroupEventKind::Completion {
        token: a,
        outcome: success(SlotValue::I64(1), Taint::Secret),
    };
    reject_and_continue(&mut g, a, b, bad)
}

#[test]
fn live_token_only_receipt() -> Result<(), GroupError> {
    let (mut g, a, b) = fixture()?;
    let bad = GroupEventKind::Completion {
        token: b,
        outcome: success(SlotValue::I64(1), Taint::Clean),
    };
    reject_and_continue(&mut g, a, b, bad)
}

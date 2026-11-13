use super::support::*;

fn drop_success(g: &mut GroupControl, t: LaneToken) -> Result<(), GroupError> {
    let snapshot = g.clone();
    let prepared = g.prepare_completion(t, success(SlotValue::Null, Taint::Secret))?;
    drop(prepared);
    assert_eq!(*g, snapshot);
    Ok(())
}

fn assert_clean_terminal(g: &mut GroupControl, t1: LaneToken, t2: LaneToken) {
    let expected = Winner {
        token: t1,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
    };
    assert_eq!(g.outcome(), GroupOutcome::Winner(expected));
    let snapshot = g.clone();
    assert_eq!(
        g.prepare_drain(t2).err(),
        Some(GroupError::InvalidTransition)
    );
    assert_eq!(*g, snapshot);
}

fn finish_clean_winner(
    g: &mut GroupControl,
    t0: LaneToken,
    t1: LaneToken,
    t2: LaneToken,
) -> Result<(), GroupError> {
    prepare_completion_rejected(
        g,
        t2,
        success(SlotValue::I64(99), Taint::Clean),
        GroupError::InvalidTransition,
    );
    drain(g, t0)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    drain(g, t2)?;
    assert_clean_terminal(g, t1, t2);
    Ok(())
}

pub(super) fn winner_requires_drain() -> Result<(), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 3, 3)?;
    let t0 = admit(&mut g)?;
    let t1 = admit(&mut g)?;
    let t2 = admit(&mut g)?;
    drop_success(&mut g, t1)?;
    complete(&mut g, t1, success(SlotValue::I64(7), Taint::Clean))?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    finish_clean_winner(&mut g, t0, t1, t2)
}

pub(super) fn dropping_prepared_events_is_atomic() -> Result<(), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 2, 2)?;
    let snap = g.clone();
    let p = g.prepare_admit()?;
    drop(p);
    assert_eq!(g, snap);
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;
    let snap2 = g.clone();
    let p2 = g.prepare_abort(AbortReason::Canceled)?;
    drop(p2);
    assert_eq!(g, snap2);
    complete(&mut g, a, success(SlotValue::Null, Taint::Clean))?;
    let snap3 = g.clone();
    let p3 = g.prepare_drain(b)?;
    drop(p3);
    assert_eq!(g, snap3);
    Ok(())
}

fn first_success_all_fail() -> Result<(), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 2, 2)?;
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;
    complete(&mut g, a, LaneCompletion::Failed)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    complete(&mut g, b, LaneCompletion::Failed)?;
    assert_eq!(
        g.outcome(),
        GroupOutcome::Failed(GroupFailure::AllBranchesFailed)
    );
    Ok(())
}

fn reject_ordered_work(g: &mut GroupControl, c2: LaneToken) -> Result<(), GroupError> {
    let snap = g.clone();
    let admit_err = g.prepare_admit().err();
    assert_eq!(admit_err, Some(GroupError::InvalidTransition));
    let completion = g.prepare_completion(c2, success(SlotValue::Null, Taint::Clean));
    let comp_err = completion.err();
    assert_eq!(comp_err, Some(GroupError::InvalidTransition));
    assert_eq!(*g, snap);
    Ok(())
}

fn together_ordered_terminal_failure() -> Result<(), GroupError> {
    let mut g2 = group(GroupMode::TogetherOrdered, 3, 3)?;
    let a2 = admit(&mut g2)?;
    let b2 = admit(&mut g2)?;
    let c2 = admit(&mut g2)?;
    complete(&mut g2, a2, success(SlotValue::Null, Taint::Clean))?;
    complete(&mut g2, b2, LaneCompletion::Failed)?;
    assert_eq!(g2.outcome(), GroupOutcome::Pending);
    reject_ordered_work(&mut g2, c2)?;
    drain(&mut g2, c2)?;
    assert_eq!(
        g2.outcome(),
        GroupOutcome::Failed(GroupFailure::BranchFailed { ordinal: 1 })
    );
    Ok(())
}

pub(super) fn all_branches_fail_and_terminal_failure_in_ordered_group() -> Result<(), GroupError> {
    first_success_all_fail()?;
    together_ordered_terminal_failure()?;
    Ok(())
}

fn abort_with_winner() -> Result<(), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 2, 2)?;
    let a = admit(&mut g)?;
    let b = admit(&mut g)?;
    complete(&mut g, a, success(SlotValue::I64(42), Taint::Secret))?;
    let winner = g.chosen_winner().ok_or(GroupError::InvalidTransition)?;
    let p = g.prepare_abort(AbortReason::ParentFailed)?;
    let ev = p.event();
    p.commit(GroupCommitReceipt::from_committed_event(ev))?;
    assert_eq!(g.chosen_winner(), Some(winner));
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    drain(&mut g, b)?;
    assert_eq!(
        g.outcome(),
        GroupOutcome::Failed(GroupFailure::ParentFailed)
    );
    Ok(())
}

fn abort_ordered_before_completion() -> Result<(), GroupError> {
    let mut g2 = group(GroupMode::ForEachOrdered, 2, 2)?;
    let x = admit(&mut g2)?;
    let y = admit(&mut g2)?;
    complete(&mut g2, x, success(SlotValue::Null, Taint::Clean))?;
    let p2 = g2.prepare_abort(AbortReason::Canceled)?;
    let ev2 = p2.event();
    p2.commit(GroupCommitReceipt::from_committed_event(ev2))?;
    assert_eq!(g2.outcome(), GroupOutcome::Pending);
    drain(&mut g2, y)?;
    assert_eq!(g2.outcome(), GroupOutcome::Failed(GroupFailure::Canceled));
    Ok(())
}

pub(super) fn abort_preserves_winner_and_blocks_partial_completion_in_ordered_group()
-> Result<(), GroupError> {
    abort_with_winner()?;
    abort_ordered_before_completion()?;
    Ok(())
}

fn prepare_completion_rejected(
    g: &mut GroupControl,
    token: LaneToken,
    outcome: LaneCompletion,
    expected: GroupError,
) {
    let snapshot2 = g.clone();
    let error = g.prepare_completion(token, outcome).err();
    assert_eq!(error, Some(expected));
    assert_eq!(*g, snapshot2);
}

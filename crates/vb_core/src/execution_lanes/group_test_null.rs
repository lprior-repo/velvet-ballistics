use super::support::*;

pub(super) fn null_wins_after_failure() -> Result<(), GroupError> {
    let mut g = group(GroupMode::FirstSuccess, 2, 2)?;
    let t0 = admit(&mut g)?;
    let t1 = admit(&mut g)?;
    complete(&mut g, t0, LaneCompletion::Failed)?;
    assert_eq!(g.outcome(), GroupOutcome::Pending);
    complete(&mut g, t1, success(SlotValue::Null, Taint::Secret))?;
    let winner = match g.outcome() {
        GroupOutcome::Winner(w) => w,
        _ => return Err(GroupError::InvalidTransition),
    };
    assert_eq!(winner.token, t1);
    assert_eq!(winner.value, SlotValue::Null);
    assert_eq!(winner.taint, Taint::Secret);
    Ok(())
}

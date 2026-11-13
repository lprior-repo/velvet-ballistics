pub(super) use crate::execution_lanes::*;
pub(super) use crate::ids::StepIdx;
pub(super) use crate::value::{SlotValue, Taint};

pub(super) fn group(mode: GroupMode, total: u32, cap: u16) -> Result<GroupControl, GroupError> {
    let id = GroupId::new(StepIdx::new(0), 1)?;
    GroupControl::new(mode, id, total, cap, 100, 10)
}

pub(super) fn admit(g: &mut GroupControl) -> Result<LaneToken, GroupError> {
    let prepared = g.prepare_admit()?;
    let event = prepared.event();
    let receipt = GroupCommitReceipt::from_committed_event(event);
    let transition = prepared.commit(receipt)?;
    match transition {
        GroupTransition::Admitted(t) => Ok(t),
        _ => Err(GroupError::InvalidTransition),
    }
}

pub(super) fn complete(
    g: &mut GroupControl,
    token: LaneToken,
    completion: LaneCompletion,
) -> Result<(), GroupError> {
    let prepared = g.prepare_completion(token, completion)?;
    let event = prepared.event();
    let receipt = GroupCommitReceipt::from_committed_event(event);
    prepared.commit(receipt)?;
    Ok(())
}

pub(super) fn drain(g: &mut GroupControl, token: LaneToken) -> Result<(), GroupError> {
    let prepared = g.prepare_drain(token)?;
    let event = prepared.event();
    let receipt = GroupCommitReceipt::from_committed_event(event);
    prepared.commit(receipt)?;
    Ok(())
}

pub(super) fn success(value: SlotValue, taint: Taint) -> LaneCompletion {
    LaneCompletion::Succeeded { value, taint }
}

pub(super) fn cell(value: SlotValue, taint: Taint) -> ResultCell {
    ResultCell::Value { value, taint }
}

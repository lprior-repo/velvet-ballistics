use crate::ids::{BranchCount, FanoutLimit, StepIdx};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow};

use super::super::ReplayError;

fn invariant(reason: &'static str) -> ReplayError {
    ReplayError::Internal { reason }
}

pub(super) fn check_for_each_capacity(
    plan: &CompiledWorkflow,
    owner: StepIdx,
    join: StepIdx,
    count: usize,
    limit: FanoutLimit,
    at_once: BranchCount,
) -> Result<(), ReplayError> {
    let group = plan
        .node(owner)
        .ok_or_else(|| invariant("concurrent group owner is out of bounds"))?;
    if !matches!(
        &group.kind,
        CompiledNodeKind::ForEachConcurrentStart {
            join: group_join,
            limit: group_limit,
            at_once: group_at_once,
            ..
        } if *group_join == join && *group_limit == limit && *group_at_once == at_once
    ) {
        return Err(invariant("for_each replay group identity mismatch"));
    }
    if !matches!(
        plan.node(join).map(|node| &node.kind),
        Some(CompiledNodeKind::ConcurrentGroupJoin { owner: join_owner })
            if *join_owner == owner
    ) {
        return Err(invariant("concurrent group join owner mismatch"));
    }
    let capacity = usize::from(at_once.get()).min(count).min(limit.as_usize());
    let requirements = plan
        .concurrent_requirements()
        .ok_or_else(|| invariant("concurrent plan missing checked requirements"))?;
    if capacity.saturating_add(1) > usize::from(requirements.frames())
        || requirements.groups() == 0
        || capacity > usize::from(plan.resource_contract().max_fanout)
    {
        return Err(invariant("concurrent replay capacity exceeds checked bounds"));
    }
    Ok(())
}

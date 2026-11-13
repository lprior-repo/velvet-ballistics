use crate::ids::StepIdx;
use crate::workflow::concurrent_lane_walk::scratch;
use crate::workflow::concurrent_pair::at;
use crate::workflow::{WorkflowError, WorkflowParts};

#[path = "together_policy_targets.rs"]
mod targets;

struct Walk {
    outer: StepIdx,
    join: StepIdx,
    queue: Vec<StepIdx>,
    seen: Vec<bool>,
}

fn init(parts: &WorkflowParts, outer: StepIdx, join: StepIdx) -> Result<Walk, WorkflowError> {
    let queue = scratch(parts.nodes.len())?;
    let seen = scratch(parts.nodes.len())?;
    let mut seen = seen;
    seen.resize(parts.nodes.len(), false);
    Ok(Walk {
        outer,
        join,
        queue,
        seen,
    })
}

fn enqueue(w: &mut Walk, target: StepIdx) -> Result<(), WorkflowError> {
    if target <= w.outer || target >= w.join {
        return Ok(());
    }
    let idx = target.as_usize();
    let flag = w
        .seen
        .get_mut(idx)
        .ok_or(WorkflowError::StepOutOfBounds { step: target })?;
    if !*flag {
        *flag = true;
        w.queue.push(target);
    }
    Ok(())
}

pub(super) fn find(
    parts: &WorkflowParts,
    outer: StepIdx,
    entry: StepIdx,
    join: StepIdx,
) -> Result<Option<StepIdx>, WorkflowError> {
    if entry.as_usize() >= parts.nodes.len() || join.as_usize() >= parts.nodes.len() {
        return Ok(None);
    }
    let mut w = init(parts, outer, join)?;
    enqueue(&mut w, entry)?;
    for index in 0..parts.nodes.len() {
        let Some(step) = w.queue.get(index).copied() else {
            break;
        };
        let node = at(parts, step)?;
        if super::is_together(&node.kind) {
            return Ok(Some(node.id));
        }
        for target in targets::of(node)? {
            enqueue(&mut w, target)?;
        }
    }
    Ok(None)
}

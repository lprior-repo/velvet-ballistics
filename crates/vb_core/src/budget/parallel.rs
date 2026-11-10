#![forbid(unsafe_code)]

use super::BudgetTraversalError;
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};

pub(super) fn compute_peak<'a>(
    nodes: impl IntoIterator<Item = &'a CompiledNode>,
) -> Result<u64, BudgetTraversalError> {
    let mut owners = Vec::new();
    for node in nodes {
        if let CompiledNodeKind::TogetherStart { branches, join } = &node.kind {
            let count = u64::try_from(branches.len())
                .map_err(|_| BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            owners.push((node.id, *join, count));
        }
    }
    owners.sort_unstable_by_key(|(start, _, _)| *start);
    let mut active: Vec<(StepIdx, u64)> = Vec::with_capacity(owners.len());
    let mut maximum = 0;
    for (start, join, count) in owners {
        active.retain(|(end, _)| *end > start);
        active.push((join, count));
        let total = active.iter().try_fold(0u64, |sum, (_, count)| {
            sum.checked_add(*count)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })
        })?;
        maximum = maximum.max(total);
    }
    Ok(maximum)
}

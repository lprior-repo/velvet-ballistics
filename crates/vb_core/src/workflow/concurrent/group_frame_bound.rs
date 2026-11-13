use crate::ids::{BranchCount, FanoutLimit, StepIdx};
use crate::workflow::CompiledNodeKind as K;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FrameBoundError {
    InvalidShape {
        owner: StepIdx,
    },
    OverFanout {
        owner: StepIdx,
        actual: u64,
        max: u16,
    },
    CountOverflow {
        owner: StepIdx,
        actual: u64,
    },
}

fn finish(owner: StepIdx, total: u64) -> Result<u16, FrameBoundError> {
    u16::try_from(total).map_err(|_| FrameBoundError::CountOverflow {
        owner,
        actual: total,
    })
}

pub(super) fn group_frame_peak(
    owner: StepIdx,
    kind: &K,
    peaks: &[u16],
    max: u16,
) -> Result<u16, FrameBoundError> {
    match kind {
        K::ForEachConcurrentStart { limit, at_once, .. } => {
            foreach_peak(owner, *limit, *at_once, peaks, max)
        }
        K::TogetherConcurrentStart { branches, .. } | K::FirstSuccessStart { branches, .. } => {
            branches_peak(owner, branches.as_ref(), peaks, max)
        }
        _ => Err(FrameBoundError::InvalidShape { owner }),
    }
}

fn foreach_peak(
    owner: StepIdx,
    limit: FanoutLimit,
    at_once: BranchCount,
    peaks: &[u16],
    max: u16,
) -> Result<u16, FrameBoundError> {
    if at_once.get() == 0 || peaks.len() != 1 {
        return Err(FrameBoundError::InvalidShape { owner });
    }
    let child = peaks
        .first()
        .copied()
        .ok_or(FrameBoundError::InvalidShape { owner })?;
    if child == 0 {
        return Err(FrameBoundError::InvalidShape { owner });
    }
    check_fanout(owner, u64::from(at_once.get()), max)?;
    let effective = u64::from(limit.get().min(u32::from(at_once.get())));
    let product = effective
        .checked_mul(u64::from(child))
        .ok_or(overflow(owner))?;
    let total = product.checked_add(1).ok_or(overflow(owner))?;
    finish(owner, total)
}

fn branches_peak(
    owner: StepIdx,
    branches: &[StepIdx],
    peaks: &[u16],
    max: u16,
) -> Result<u16, FrameBoundError> {
    if branches.is_empty() || branches.len() != peaks.len() {
        return Err(FrameBoundError::InvalidShape { owner });
    }
    if peaks.contains(&0) {
        return Err(FrameBoundError::InvalidShape { owner });
    }
    let count = u64::try_from(branches.len()).map_err(|_| FrameBoundError::CountOverflow {
        owner,
        actual: u64::MAX,
    })?;
    check_fanout(owner, count, max)?;
    let total = peaks.iter().try_fold(1u64, |acc, p| {
        acc.checked_add(u64::from(*p)).ok_or(overflow(owner))
    })?;
    finish(owner, total)
}
fn overflow(owner: StepIdx) -> FrameBoundError {
    FrameBoundError::CountOverflow {
        owner,
        actual: u64::MAX,
    }
}

fn check_fanout(owner: StepIdx, count: u64, max: u16) -> Result<(), FrameBoundError> {
    if count > u64::from(max) {
        return Err(FrameBoundError::OverFanout {
            owner,
            actual: count,
            max,
        });
    }
    Ok(())
}

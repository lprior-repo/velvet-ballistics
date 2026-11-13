use vb_core::{ids, workflow};

#[path = "../src/workflow/concurrent/group_frame_bound.rs"]
mod group_frame_bound;

use group_frame_bound::{FrameBoundError, group_frame_peak};
use ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use workflow::CompiledNodeKind as K;

fn owner() -> StepIdx {
    StepIdx::new(7)
}

fn foreach_kind(limit: u32, cap: u16) -> K {
    K::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(limit),
        at_once: BranchCount::new(cap),
        body: StepIdx::new(1),
        join: StepIdx::new(9),
    }
}

fn branches(first: bool) -> K {
    let branches = vec![StepIdx::new(1), StepIdx::new(3)].into_boxed_slice();
    if first {
        K::FirstSuccessStart {
            branches,
            join: StepIdx::new(9),
        }
    } else {
        K::TogetherConcurrentStart {
            branches,
            join: StepIdx::new(9),
        }
    }
}

#[test]
fn foreach_basic_limit_capped() {
    let kind = foreach_kind(20, 8);
    let res = group_frame_peak(owner(), &kind, &[5], 10);
    assert_eq!(res, Ok(41));
}

#[test]
fn foreach_limit_smaller_than_cap() {
    let kind = foreach_kind(1, 8);
    let res = group_frame_peak(owner(), &kind, &[3], 10);
    assert_eq!(res, Ok(4));
}

#[test]
fn foreach_zero_limit_uses_one() {
    let kind = foreach_kind(0, 8);
    let res = group_frame_peak(owner(), &kind, &[65535], 10);
    assert_eq!(res, Ok(1));
}

#[test]
fn foreach_zero_at_once_invalid() {
    let kind = foreach_kind(10, 0);
    let res = group_frame_peak(owner(), &kind, &[5], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

#[test]
fn foreach_wrong_child_count_empty() {
    let kind = foreach_kind(10, 8);
    let res = group_frame_peak(owner(), &kind, &[], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

#[test]
fn foreach_wrong_child_count_two() {
    let kind = foreach_kind(10, 8);
    let res = group_frame_peak(owner(), &kind, &[1, 2], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

#[test]
fn foreach_child_zero_invalid() {
    let kind = foreach_kind(10, 8);
    let res = group_frame_peak(owner(), &kind, &[0], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

#[test]
fn foreach_over_fanout_even_zero_limit() {
    let kind = foreach_kind(0, 3);
    let res = group_frame_peak(owner(), &kind, &[5], 2);
    assert_eq!(
        res,
        Err(FrameBoundError::OverFanout {
            owner: owner(),
            actual: 3,
            max: 2
        })
    );
}

#[test]
fn foreach_max_values_overflow() {
    let kind = foreach_kind(u32::MAX, u16::MAX);
    let res = group_frame_peak(owner(), &kind, &[u16::MAX], u16::MAX);
    assert_eq!(
        res,
        Err(FrameBoundError::CountOverflow {
            owner: owner(),
            actual: 4294836226
        })
    );
}

#[test]
fn branches_together_fixed_peaks() {
    let kind = branches(false);
    let res = group_frame_peak(owner(), &kind, &[3, 7], 10);
    assert_eq!(res, Ok(11));
}

#[test]
fn branches_first_success_same_calc() {
    let kind = branches(true);
    let res = group_frame_peak(owner(), &kind, &[3, 7], 10);
    assert_eq!(res, Ok(11));
}

#[test]
fn branches_mismatched_lane_count() {
    let kind = branches(false);
    let res = group_frame_peak(owner(), &kind, &[3], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

#[test]
fn branches_zero_child_peak_invalid() {
    let kind = branches(false);
    let res = group_frame_peak(owner(), &kind, &[0, 7], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

#[test]
fn branches_over_fanout() {
    let kind = branches(false);
    let res = group_frame_peak(owner(), &kind, &[1, 1], 1);
    assert_eq!(
        res,
        Err(FrameBoundError::OverFanout {
            owner: owner(),
            actual: 2,
            max: 1
        })
    );
}

#[test]
fn branches_sum_overflow() {
    let kind = branches(false);
    let res = group_frame_peak(owner(), &kind, &[65535, 1], 10);
    assert_eq!(
        res,
        Err(FrameBoundError::CountOverflow {
            owner: owner(),
            actual: 65537
        })
    );
}

#[test]
fn non_concurrent_kind_invalid_shape() {
    let kind = K::Nop;
    let res = group_frame_peak(owner(), &kind, &[], 10);
    assert_eq!(res, Err(FrameBoundError::InvalidShape { owner: owner() }));
}

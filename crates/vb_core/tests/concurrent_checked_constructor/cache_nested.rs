use super::{
    CompiledNodeKind, CompiledWorkflow, WorkflowError, assert_accepted, declare_input, n, parts,
};
use vb_core::execution_lanes::GroupMode;
use vb_core::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use vb_core::workflow::{ContractResource, WorkflowParts};

#[path = "../concurrent_together_support/mod.rs"]
mod support;

fn nested_foreach_parts() -> WorkflowParts {
    let mut submitted = support::pair(GroupMode::ForEachOrdered, GroupMode::ForEachOrdered);
    let mut nodes = submitted.nodes.to_vec();
    for (index, node) in nodes.iter_mut().enumerate() {
        if let CompiledNodeKind::ForEachConcurrentStart { limit, at_once, .. } = &mut node.kind {
            match index {
                0 => {
                    *limit = FanoutLimit::new(8);
                    *at_once = BranchCount::new(8);
                }
                1 => {
                    *limit = FanoutLimit::new(4);
                    *at_once = BranchCount::new(4);
                }
                _ => {}
            }
        }
    }
    submitted.nodes = nodes.into_boxed_slice();
    submitted
}

#[test]
fn nested_foreach_caches_peak_frames_groups() -> Result<(), WorkflowError> {
    let submitted = nested_foreach_parts();
    let wf = CompiledWorkflow::try_from_concurrent_parts(submitted.clone())?;
    assert_accepted(&wf, &submitted, 41, 9);
    Ok(())
}

#[test]
fn deep_foreach_bridge_rejects_nested_together_exactly() {
    let mut submitted = support::deep(GroupMode::ForEachOrdered);
    let mut configured = false;
    for (index, node) in submitted.nodes.iter_mut().enumerate() {
        if index == 1 {
            if let CompiledNodeKind::ForEachConcurrentStart { at_once, .. } = &mut node.kind {
                *at_once = BranchCount::new(2);
                configured = true;
            }
        }
    }
    assert!(configured);
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(submitted),
        Err(WorkflowError::NestedTogether {
            outer: StepIdx::new(0),
            inner: StepIdx::new(2),
        })
    );
}

fn foreach_start(at_once: u16) -> CompiledNodeKind {
    CompiledNodeKind::ForEachConcurrentStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: FanoutLimit::new(4),
        at_once: BranchCount::new(at_once),
        body: StepIdx::new(1),
        join: StepIdx::new(3),
    }
}

fn foreach_parts(name: &str, at_once: u16) -> WorkflowParts {
    let mut submitted = parts(
        name,
        Box::new([
            n(0, foreach_start(at_once), None, None),
            n(1, CompiledNodeKind::Nop, None, Some(2)),
            n(
                2,
                CompiledNodeKind::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: None,
                },
                None,
                Some(3),
            ),
            n(
                3,
                CompiledNodeKind::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(1),
                Some(4),
            ),
            n(
                4,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]),
        2,
    );
    declare_input(&mut submitted);
    submitted
}

#[test]
fn zero_at_once_rejects_branch_count() {
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(foreach_parts("zero_at_once", 0)).map(|_| ()),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::BranchCount
        })
    );
}

#[test]
fn above_max_fanout_rejects_with_overfanout_error() {
    let submitted = foreach_parts("over_fanout", 65);
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(submitted).map(|_| ()),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Fanout
        })
    );
}

#[path = "cache_complex.rs"]
mod cache_complex;

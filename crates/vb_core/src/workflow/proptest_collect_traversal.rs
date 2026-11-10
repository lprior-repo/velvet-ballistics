#![cfg(test)]
#![forbid(unsafe_code)]

use super::proptest_collect_budget::{check_collect_budget, make_collect_workflow};
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, SlotFlowError, SlotFlowFailure, WorkflowError,
};
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

proptest! {
    #[test]
    fn proptest_collect_traversal(limit in any::<u32>(), page_size in any::<u32>()) {
        check_collect_budget(limit, page_size)?;
    }

    #[test]
    fn proptest_collect_traversal_owned_loop(limit in 0u32..=499_999, page_size in any::<u32>()) {
        check_collect_budget(limit, page_size)?;
    }
}

#[test]
fn proptest_collect_traversal_limit_zero() -> TestCaseResult {
    check_collect_budget(0, 1)
}

#[test]
fn proptest_collect_traversal_limit_one() -> TestCaseResult {
    check_collect_budget(1, 1)
}

#[test]
fn proptest_collect_traversal_limit_max() -> TestCaseResult {
    check_collect_budget(u32::MAX, u32::MAX)
}

#[test]
fn collect_traversal_rejects_unpaired_latch() -> TestCaseResult {
    let mut parts = make_collect_workflow(1, 1);
    let latch = parts
        .nodes
        .get_mut(2)
        .ok_or_else(|| TestCaseError::fail("missing latch"))?;
    latch.kind = CompiledNodeKind::CollectNext {
        collector_slot: SlotIdx::new(0),
        body: StepIdx::new(1),
        done: StepIdx::new(3),
    };
    prop_assert_eq!(
        CompiledWorkflow::try_from_parts(parts),
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(2),
        })
    );
    Ok(())
}

#[test]
fn collect_traversal_requires_initialized_source() -> TestCaseResult {
    for optional in [false, true] {
        let mut parts = make_collect_workflow(1, 1);
        if optional {
            let input = parts
                .input_slots
                .first_mut()
                .ok_or_else(|| TestCaseError::fail("missing source input"))?;
            input.constraint.optional = true;
        } else {
            parts.input_slots = Box::new([]);
        }
        prop_assert_eq!(
            CompiledWorkflow::try_from_parts(parts),
            Err(WorkflowError::SlotFlow(SlotFlowError {
                step: StepIdx::new(0),
                slot: Some(SlotIdx::new(0)),
                reason: SlotFlowFailure::UninitializedRead,
            }))
        );
    }
    Ok(())
}

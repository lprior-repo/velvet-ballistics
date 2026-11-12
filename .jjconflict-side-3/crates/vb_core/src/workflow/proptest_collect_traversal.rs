#![cfg(test)]
#![forbid(unsafe_code)]

use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, SlotFlowError, SlotFlowFailure, WorkflowError,
};
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

use super::proptest_collect_budget::{
    check_collect_budget, expected_budget_total, make_collect_workflow, make_yaml_collect_workflow,
};

fn bounded_collect_strategy() -> impl Strategy<Value = (u32, u32)> {
    (0..1000u32, 0..1000u32)
}

fn assert_collect_region(limit: u32, page_size: u32) -> Result<(), String> {
    let parts = make_collect_workflow(limit, page_size);
    let compiled = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let region = compiled
        .loop_region(StepIdx::new(2))
        .ok_or("loop region at control step 2 missing")?;
    assert_eq!(region.start(), StepIdx::ZERO);
    assert_eq!(region.body(), StepIdx::new(1));
    assert_eq!(region.control(), StepIdx::new(2));
    assert_eq!(region.done(), StepIdx::new(3));
    Ok(())
}

proptest! {
    #[test]
    fn proptest_collect_traversal((limit, page_size) in bounded_collect_strategy()) {
        assert_collect_region(limit, page_size).map_err(TestCaseError::fail)?;
    }

    #[test]
    fn proptest_collect_traversal_unrestricted(limit in any::<u32>(), page_size in any::<u32>()) {
        check_collect_budget(limit, page_size)?;
    }

    #[test]
    fn proptest_collect_traversal_owned_loop(limit in 0u32..=499_999, page_size in any::<u32>()) {
        check_collect_budget(limit, page_size)?;
    }
}

#[test]
fn proptest_collect_traversal_limit_zero() -> Result<(), String> {
    assert_collect_region(0, 1)?;
    check_collect_budget(0, 1).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn proptest_collect_traversal_limit_one() -> Result<(), String> {
    assert_collect_region(1, 1)?;
    check_collect_budget(1, 1).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn proptest_collect_traversal_limit_max() -> Result<(), String> {
    let parts = make_collect_workflow(u32::MAX, u32::MAX);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepCountOverflow { actual }) => {
            let expected = expected_budget_total(u32::MAX).map_err(|e| e.to_string())?;
            assert_eq!(actual, expected);
        }
        other => return Err(format!("expected StepCountOverflow, got {other:?}")),
    }
    check_collect_budget(u32::MAX, u32::MAX).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn collect_traversal_rejects_unpaired_latch() -> TestCaseResult {
    let mut parts = make_yaml_collect_workflow(1, 1);
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
        let mut parts = make_yaml_collect_workflow(1, 1);
        if optional {
            let input = parts
                .input_slots
                .first_mut()
                .ok_or_else(|| TestCaseError::fail("missing source input"))?;
            input.constraint.presence = super::Presence::Optional;
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

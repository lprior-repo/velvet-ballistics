#![cfg(test)]
#![forbid(unsafe_code)]

use crate::budget::{BoundednessPolicy, BudgetDimension, BudgetError, WholeWorkflowBudget};
use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowError, WorkflowParts, node,
};
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

/// Source is supplied by the caller; collector and result are initialized by the loop.
pub(super) fn make_collect_workflow(limit: u32, page_size: u32) -> WorkflowParts {
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(3);
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([CompiledInputSlot {
            slot: source,
            kind: InputSlotKind::List,
            constraint: CompiledInputConstraint::required(),
        }]),
        name: "collect-budget".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::CollectStart {
                    source,
                    limit,
                    page_size,
                    body,
                    done,
                },
                Some(collector),
                None,
            ),
            node(
                body,
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::CollectNext {
                    collector_slot: collector,
                    body,
                    done,
                },
                None,
                None,
            ),
            node(
                done,
                CompiledNodeKind::CollectFinish {
                    collector_slot: collector,
                },
                Some(SlotIdx::new(3)),
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([crate::value::ConstValue::I64(0)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

pub(super) fn check_collect_budget(limit: u32, page_size: u32) -> TestCaseResult {
    let parts = make_collect_workflow(limit, page_size);
    // Start and finish execute once; the body and latch have a conservative
    // minimum of one activation even when the declared item limit is zero.
    let steps = u64::from(limit)
        .max(1)
        .checked_mul(2)
        .and_then(|count| count.checked_add(2))
        .ok_or_else(|| TestCaseError::fail("test step bound overflow"))?;
    let computed =
        WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract);
    let constructed = CompiledWorkflow::try_from_parts(parts.clone());
    if steps > u64::from(u32::MAX) {
        let expected = WorkflowError::StepCountOverflow { actual: steps };
        prop_assert_eq!(computed, Err(expected.clone()));
        prop_assert_eq!(constructed, Err(expected));
        return Ok(());
    }
    let budget = computed.map_err(|error| TestCaseError::fail(format!("budget: {error:?}")))?;
    prop_assert_eq!(budget.max_total_steps, steps);
    prop_assert_eq!(budget.max_gather_items, limit);
    let pages = u64::from(limit).div_ceil(u64::from(page_size).max(1));
    prop_assert_eq!(u64::from(budget.max_gather_pages), pages);
    prop_assert_eq!(budget.max_nesting_depth, 1);
    let policy = BoundednessPolicy::DEFAULT;
    if steps > policy.max_total_steps {
        prop_assert_eq!(
            policy.validate(&budget),
            Err(BudgetError::TotalStepsExceeded {
                actual: steps,
                limit: policy.max_total_steps,
            })
        );
        prop_assert_eq!(
            constructed,
            Err(WorkflowError::BudgetPolicyExceeded {
                detail: BudgetDimension::TotalSteps,
            })
        );
    } else {
        prop_assert_eq!(policy.validate(&budget), Ok(()));
        let workflow = constructed
            .map_err(|error| TestCaseError::fail(format!("valid collect rejected: {error:?}")))?;
        prop_assert_eq!(workflow.to_parts(), parts);
        let region = workflow
            .loop_region(StepIdx::new(2))
            .ok_or_else(|| TestCaseError::fail("missing CollectNext ownership"))?;
        prop_assert_eq!(region.start(), StepIdx::new(0));
        prop_assert_eq!(region.body(), StepIdx::new(1));
        prop_assert_eq!(region.control(), StepIdx::new(2));
        prop_assert_eq!(region.done(), StepIdx::new(3));
    }
    Ok(())
}

#[test]
fn proptest_collect_budget_limits() -> TestCaseResult {
    for limit in [0, 1, 499_999, 500_000, u32::MAX / 2, u32::MAX] {
        for page_size in [0, 1, u32::MAX] {
            check_collect_budget(limit, page_size)?;
        }
    }
    Ok(())
}

proptest! {
    #[test]
    fn proptest_collect_budget_arbitrary(limit in any::<u32>(), page_size in any::<u32>()) {
        check_collect_budget(limit, page_size)?;
    }

    #[test]
    fn proptest_collect_budget_admitted(limit in 0u32..=499_999, page_size in any::<u32>()) {
        check_collect_budget(limit, page_size)?;
    }
}

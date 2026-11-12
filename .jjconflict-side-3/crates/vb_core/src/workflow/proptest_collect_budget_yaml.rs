use crate::budget::{BoundednessPolicy, BudgetDimension, BudgetError, WholeWorkflowBudget};
use crate::ids::{ConstIdx, SlotIdx, StepIdx};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowError, WorkflowParts, node,
};
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

pub(super) fn make_collect_workflow(limit: u32, page_size: u32) -> WorkflowParts {
    let mut parts = super::make_collect_workflow(limit, page_size);
    parts.name = "collect-budget".into();
    parts.nodes = Box::new([
        super::collect_start_node(limit, page_size),
        body_node(),
        super::collect_next_node(),
        finish_node(),
    ]);
    parts.constants = Box::new([ConstValue::I64(0)]);
    parts.slot_count = 4;
    parts
}

fn body_node() -> CompiledNode {
    node(
        StepIdx::new(1),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(2)),
    )
}

fn finish_node() -> CompiledNode {
    node(
        StepIdx::new(3),
        CompiledNodeKind::CollectFinish {
            collector_slot: SlotIdx::new(1),
        },
        Some(SlotIdx::new(3)),
        None,
    )
}

fn expected_steps(limit: u32) -> Result<u64, TestCaseError> {
    u64::from(limit)
        .max(1)
        .checked_mul(2)
        .and_then(|count| count.checked_add(2))
        .ok_or_else(|| TestCaseError::fail("test step bound overflow"))
}

pub(super) fn check_collect_budget(limit: u32, page_size: u32) -> TestCaseResult {
    let parts = make_collect_workflow(limit, page_size);
    let steps = expected_steps(limit)?;
    let computed =
        WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract);
    let constructed = CompiledWorkflow::try_from_parts(parts.clone());
    if steps > u64::from(u32::MAX) {
        assert_overflow(computed, constructed, steps)
    } else {
        let budget = computed.map_err(|error| TestCaseError::fail(format!("budget: {error:?}")))?;
        assert_dimensions(&budget, limit, page_size, steps)?;
        assert_policy(parts, budget, constructed, steps)
    }
}

fn assert_overflow(
    computed: Result<WholeWorkflowBudget, WorkflowError>,
    constructed: Result<CompiledWorkflow, WorkflowError>,
    steps: u64,
) -> TestCaseResult {
    let expected = WorkflowError::StepCountOverflow { actual: steps };
    prop_assert_eq!(computed, Err(expected.clone()));
    prop_assert_eq!(constructed, Err(expected));
    Ok(())
}

fn assert_dimensions(
    budget: &WholeWorkflowBudget,
    limit: u32,
    page_size: u32,
    steps: u64,
) -> TestCaseResult {
    prop_assert_eq!(budget.max_total_steps, steps);
    prop_assert_eq!(budget.max_gather_items, limit);
    let pages = u64::from(limit).div_ceil(u64::from(page_size).max(1));
    prop_assert_eq!(u64::from(budget.max_gather_pages), pages);
    prop_assert_eq!(budget.max_nesting_depth, 1);
    Ok(())
}

fn assert_policy(
    parts: WorkflowParts,
    budget: WholeWorkflowBudget,
    constructed: Result<CompiledWorkflow, WorkflowError>,
    steps: u64,
) -> TestCaseResult {
    let policy = BoundednessPolicy::DEFAULT;
    if steps > policy.max_total_steps {
        assert_policy_exceeded(&budget, constructed, steps)
    } else {
        prop_assert_eq!(policy.validate(&budget), Ok(()));
        assert_admitted(parts, constructed)
    }
}

fn assert_policy_exceeded(
    budget: &WholeWorkflowBudget,
    constructed: Result<CompiledWorkflow, WorkflowError>,
    steps: u64,
) -> TestCaseResult {
    let policy = BoundednessPolicy::DEFAULT;
    prop_assert_eq!(
        policy.validate(budget),
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
    Ok(())
}

fn assert_admitted(
    parts: WorkflowParts,
    constructed: Result<CompiledWorkflow, WorkflowError>,
) -> TestCaseResult {
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
    Ok(())
}

pub(super) fn boundary_cases() -> impl Iterator<Item = (u32, u32)> {
    [0, 1, 499_999, 500_000, 2_147_483_647, u32::MAX]
        .into_iter()
        .flat_map(|limit| [0, 1, u32::MAX].into_iter().map(move |page| (limit, page)))
}

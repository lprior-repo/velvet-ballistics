// Verification artifact: proptest_collect_budget.rs
// PO: PO-026 (Collect budget boundary values)
// Verifier: proptest
// Command: cargo test -p vb_core -- proptest_collect_budget --test-threads=1

#![cfg(test)]
#![forbid(unsafe_code)]

use crate::budget::WholeWorkflowBudget;
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
};
use proptest::prelude::*;

fn collect_start_node(limit: u32, page_size: u32) -> CompiledNode {
    node(
        StepIdx::ZERO,
        CompiledNodeKind::CollectStart {
            source: SlotIdx::ZERO,
            limit: limit.into(),
            page_size,
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
        Some(SlotIdx::new(1)),
        None,
    )
}

fn collect_next_node() -> CompiledNode {
    node(
        StepIdx::new(2),
        CompiledNodeKind::CollectNext {
            collector_slot: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
        None,
        None,
    )
}

fn collect_finish_node() -> CompiledNode {
    node(
        StepIdx::new(3),
        CompiledNodeKind::CollectFinish {
            collector_slot: SlotIdx::new(1),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(4)),
    )
}

fn collect_nodes(limit: u32, page_size: u32) -> Box<[CompiledNode]> {
    Box::from([
        collect_start_node(limit, page_size),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(2)),
        ),
        collect_next_node(),
        collect_finish_node(),
        node(
            StepIdx::new(4),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
            None,
            None,
        ),
    ])
}

pub(super) fn make_collect_workflow(limit: u32, page_size: u32) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from("test_collect_budget"),
        digest: crate::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: collect_nodes(limit, page_size),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

pub(super) fn expected_budget_total(limit: u32) -> Result<u64, TestCaseError> {
    u64::from(limit)
        .max(1)
        .checked_mul(2)
        .and_then(|total| total.checked_add(3))
        .ok_or_else(|| TestCaseError::fail("budget oracle overflow"))
}

fn budget_boundary_strategy() -> impl Strategy<Value = (u32, u32)> {
    prop_oneof![
        Just((0, 0)),
        Just((0, 1)),
        Just((1, 0)),
        Just((1, 1)),
        Just((u32::MAX, 1)),
        Just((1, u32::MAX)),
        Just((u32::MAX, u32::MAX)),
    ]
}

fn budget_arbitrary_strategy() -> impl Strategy<Value = (u32, u32)> {
    (any::<u32>(), any::<u32>())
}

fn assert_budget_overflow(parts: WorkflowParts, total: u64) -> Result<(), TestCaseError> {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(crate::workflow::WorkflowError::StepCountOverflow { actual }) => {
            prop_assert_eq!(actual, total, "StepCountOverflow actual must match oracle");
            Ok(())
        }
        other => Err(TestCaseError::fail(format!(
            "expected StepCountOverflow for total={total}, got {other:?}"
        ))),
    }
}

fn assert_gather_dimensions(
    budget: &WholeWorkflowBudget,
    limit: u32,
    page_size: u32,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(
        budget.max_gather_items,
        limit,
        "max_gather_items must equal limit"
    );
    let pages = u64::from(limit).div_ceil(u64::from(page_size).max(1));
    let pages_u32 = u32::try_from(pages).map_err(|error| TestCaseError::fail(error.to_string()))?;
    prop_assert_eq!(
        budget.max_gather_pages,
        pages_u32,
        "max_gather_pages must match ceil(limit/page_size)"
    );
    Ok(())
}

fn assert_budget_dimensions(
    parts: WorkflowParts,
    limit: u32,
    page_size: u32,
    total: u64,
) -> Result<(), TestCaseError> {
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
        .map_err(|error| TestCaseError::fail(format!("budget compute failed: {error}")))?;
    prop_assert_eq!(
        budget.max_total_steps,
        total,
        "max_total_steps must match oracle"
    );
    assert_gather_dimensions(&budget, limit, page_size)
}

fn assert_collect_budget(limit: u32, page_size: u32) -> Result<(), TestCaseError> {
    let parts = make_collect_workflow(limit, page_size);
    let total = expected_budget_total(limit)?;
    if total > u64::from(u32::MAX) {
        assert_budget_overflow(parts, total)
    } else {
        assert_budget_dimensions(parts, limit, page_size, total)
    }
}

proptest! {
    #[test]
    fn proptest_collect_budget_limits((limit, page_size) in budget_boundary_strategy()) {
        assert_collect_budget(limit, page_size)?;
    }

    #[test]
    fn proptest_collect_budget_arbitrary((limit, page_size) in budget_arbitrary_strategy()) {
        assert_collect_budget(limit, page_size)?;
    }
}

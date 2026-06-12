//! Test chunk 006 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 1325–1598 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};

use super::prelude::*;

const fn test_budget(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> WholeWorkflowBudget {
    WholeWorkflowBudget {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 3,
        max_trace_events: 4,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 5,
        max_blob_bytes: 6,
        max_input_bytes: 7,
        max_queue_depth: 0,
    }
}

const fn test_policy(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> BoundednessPolicy {
    BoundednessPolicy {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    }
}

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

fn single_node_workflow() -> Vec<CompiledNode> {
    vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }]
}

fn test_node(id: u16, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn finish_node(id: u16) -> CompiledNode {
    test_node(
        id,
        None,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    )
}

fn nested_fanout_loop_nodes() -> Vec<CompiledNode> {
    vec![
        test_node(
            0,
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(5),
            },
        ),
        test_node(
            1,
            None,
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(2), StepIdx::new(3)].into_boxed_slice(),
                join: StepIdx::new(4),
            },
        ),
        test_node(2, None, CompiledNodeKind::Nop),
        test_node(3, None, CompiledNodeKind::Nop),
        test_node(
            4,
            None,
            CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: SlotIdx::new(2),
            },
        ),
        test_node(
            5,
            Some(6),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(3),
            },
        ),
        finish_node(6),
    ]
}

fn sequential_collect_reduce_repeat_wait_nodes() -> Vec<CompiledNode> {
    vec![
        test_node(
            0,
            None,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 3,
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        ),
        test_node(1, None, CompiledNodeKind::Nop),
        test_node(
            2,
            Some(3),
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(2),
            },
        ),
        test_node(
            3,
            None,
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(2),
                accumulator: SlotIdx::new(3),
                initial: ConstIdx::new(0),
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        ),
        test_node(4, None, CompiledNodeKind::Nop),
        test_node(
            5,
            Some(6),
            CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(3),
            },
        ),
        test_node(
            6,
            None,
            CompiledNodeKind::RepeatStart {
                max_attempts: 2,
                body: StepIdx::new(7),
                done: StepIdx::new(8),
            },
        ),
        test_node(
            7,
            None,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(4),
            },
        ),
        test_node(
            8,
            Some(9),
            CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(5),
            },
        ),
        finish_node(9),
    ]
}

fn conditional_max_nodes() -> Vec<CompiledNode> {
    vec![
        test_node(
            0,
            None,
            CompiledNodeKind::ChooseSlot {
                branches: vec![
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(1),
                        target: StepIdx::new(3),
                    },
                ]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(5)),
            },
        ),
        test_node(1, Some(2), CompiledNodeKind::Nop),
        finish_node(2),
        test_node(3, Some(4), CompiledNodeKind::Nop),
        test_node(4, Some(5), CompiledNodeKind::Nop),
        finish_node(5),
    ]
}

// -------------------------------------------------------------------------
// 1. Step budget creation and validation
// -------------------------------------------------------------------------

#[test]
fn step_budget_creation_at_one() -> Result<(), String> {
    let b = StepBudget::new(1);
    ensure_equal(b.remaining(), 1)
}

#[test]
fn step_budget_creation_at_max() -> Result<(), String> {
    let b = StepBudget::new(crate::limits::MAX_STEP_BUDGET);
    ensure_equal(b.remaining(), crate::limits::MAX_STEP_BUDGET)
}

#[test]
fn step_budget_creation_at_zero() -> Result<(), String> {
    let b = StepBudget::new(0);
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_creation_clamps_large_value() -> Result<(), String> {
    let b = StepBudget::new(u64::MAX);
    ensure_equal(b.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// 2. Budget consumption tracking (single step, multi-step)
// -------------------------------------------------------------------------

#[test]
fn step_budget_single_consumption_decrements() -> Result<(), String> {
    let mut b = StepBudget::new(5);
    let taken = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken, true)?;
    ensure_equal(b.remaining(), 4)
}

//! Test helper functions shared across the chunked budget test files.
//!
//! These helpers were originally defined inline in the 7339-line
//! `tests.rs` file. After the file was split into 29 chunks
//! (`chunk_001.rs` through `chunk_029.rs`), the helpers were extracted
//! to this module so all chunks can use them via `use super::prelude::*;`.

#![allow(dead_code, clippy::all, clippy::pedantic)]

use crate::ids::{ConstIdx, SlotIdx, StepIdx};
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

pub(crate) const fn test_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants: 1,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 0,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_input_bytes: 1,
        max_output_bytes: 1,
        max_blob_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_retry_attempts: 0,
        max_fanout: 64,
        max_collect_items: 0,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        ..ResourceContract::DEFAULT
    }
}

pub(crate) const fn test_budget(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> crate::budget::WholeWorkflowBudget {
    crate::budget::WholeWorkflowBudget {
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

pub(crate) const fn test_policy(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> crate::budget::BoundednessPolicy {
    crate::budget::BoundednessPolicy {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..crate::budget::BoundednessPolicy::DEFAULT
    }
}

pub(crate) fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

pub(crate) fn single_node_workflow() -> Vec<CompiledNode> {
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

pub(crate) fn test_node(id: u16, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

pub(crate) fn finish_node(id: u16) -> CompiledNode {
    test_node(
        id,
        None,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    )
}

pub(crate) fn nested_fanout_loop_nodes() -> Vec<CompiledNode> {
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

pub(crate) fn sequential_collect_reduce_repeat_wait_nodes() -> Vec<CompiledNode> {
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

pub(crate) fn conditional_max_nodes() -> Vec<CompiledNode> {
    use crate::workflow::SlotBranch;
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

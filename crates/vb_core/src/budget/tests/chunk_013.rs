//! Test chunk 013 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3179–3449 of the original. Semantic content is
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


fn repeat_start_max_attempts_tracks_maximum_not_sum() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(6, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_repeat_attempts, 10)
}

// -------------------------------------------------------------------------
// Additional coverage: StepBudget::MAX is const
// -------------------------------------------------------------------------

#[test]
fn step_budget_max_is_const_compatible() -> Result<(), String> {
    const _MAX: StepBudget = StepBudget::MAX;
    ensure_equal(_MAX.remaining(), crate::limits::MAX_STEP_BUDGET)
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget debug format
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_debug_format() -> Result<(), String> {
    let budget = test_budget(10, 20, 4, 2);
    let debug = format!("{budget:?}");
    ensure_equal(debug.contains("WholeWorkflowBudget"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn boundedness_policy_debug_format() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let debug = format!("{policy:?}");
    ensure_equal(debug.contains("BoundednessPolicy"), true)?;
    ensure_equal(debug.contains("max_total_steps"), true)
}

#[test]
fn budget_error_debug_format() -> Result<(), String> {
    let err = BudgetError::FanoutExceeded {
        actual: 5,
        limit: 3,
    };
    let debug = format!("{err:?}");
    ensure_equal(debug.contains("FanoutExceeded"), true)
}

// =========================================================================
// Additional edge-case tests — Budget construction, checked operations
// =========================================================================

#[test]
fn whole_workflow_budget_zero_fields_is_valid() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(budget.max_total_steps, 0)?;
    ensure_equal(budget.max_total_slots, 0)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

#[test]
fn whole_workflow_budget_max_fields() -> Result<(), String> {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::MAX,
        max_total_slots: u64::MAX,
        max_fanout: u16::MAX,
        max_nesting_depth: u16::MAX,
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
        max_timer_entries: u32::MAX,
        max_trace_events: u64::MAX,
        max_journal_batch_bytes: u32::MAX,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_queue_depth: u32::MAX,
    };
    ensure_equal(budget.max_total_steps, u64::MAX)?;
    ensure_equal(budget.max_total_slots, u64::MAX)?;
    ensure_equal(budget.max_fanout, u16::MAX)?;
    ensure_equal(budget.max_nesting_depth, u16::MAX)
}

#[test]
fn boundedness_policy_default_values_are_sensible() -> Result<(), String> {
    let p = BoundednessPolicy::DEFAULT;
    ensure_equal(p.max_total_steps, 1_000)?;
    ensure_equal(p.max_total_slots, 65_535)?;
    ensure_equal(p.max_fanout, 64)?;
    ensure_equal(p.max_nesting_depth, 8)?;
    ensure_equal(p.absolute_max_action_tickets, 100_000)?;
    ensure_equal(p.absolute_max_parallel, 256)?;
    ensure_equal(p.absolute_max_run_time_seconds, 2_592_000)?;
    ensure_equal(p.absolute_max_result_bytes, 262_144)?;
    ensure_equal(p.absolute_max_steps_executable, 1_000_000)
}

#[test]
fn budget_error_all_variants_display_non_empty() -> Result<(), String> {
    let errors = [
        BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::TotalSlotsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::FanoutExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::NestingDepthExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ParallelExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ActionTicketsExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::RunTimeExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::ResultBytesExceeded {
            actual: 1,
            limit: 0,
        },
        BudgetError::StepsExecutableExceeded {
            actual: 1,
            limit: 0,
        },
    ];
    for err in &errors {
        let display = format!("{err}");
        if display.is_empty() {
            return Err(format!("BudgetError display is empty for {err:?}"));
        }
    }
    Ok(())
}

#[test]
fn budget_error_equality_same_variants() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    ensure_equal(a, b)
}

#[test]
fn budget_error_inequality_different_actual() -> Result<(), String> {
    let a = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    let b = BudgetError::TotalStepsExceeded {
        actual: 6,
        limit: 3,
    };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn budget_error_clone_preserves_equality() -> Result<(), String> {
    let a = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 5,
    };
    let b = a.clone();
    ensure_equal(a, b)
}

#[test]
fn budget_error_from_workflow_error_preserves_variant() -> Result<(), String> {
    let wf_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(0),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel for workflow error, got {other:?}"
        )),
    }
}

#[test]

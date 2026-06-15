//! Edge-case tests for the step engine and budget validation.
//!
//! Covers verified gaps not exercised by the main `tests.rs` module:
//! - `resume_action_completion` when a Do node has `next: None`
//! - `validate_step_ceilings` at boundary values (zero, exact limit, over limit)

use crate::action::ActionTicket;
use crate::budget::{AggregateBudgetError, AggregateResourceBudget};
use crate::engine::step::{
    EngineSignal, RunFrame, ValueStore, resume_action_completion, step_once,
};
use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers (mirrors step/tests.rs conventions)
// ---------------------------------------------------------------------------

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

fn test_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// resume_action_completion — MissingNextStep path
// ---------------------------------------------------------------------------

/// GAP-STEP-001: When a Do node carries `next: None`, `resume_action_completion`
/// must return `EngineError::MissingNextStep` rather than panicking or
/// silently succeeding.
///
/// The Do node in this workflow has `next: None`, which is the exact
/// configuration that triggers line 174 of `engine/step.rs`:
///   .next.ok_or(EngineError::MissingNextStep { step })?
#[test]
fn resume_action_completion_missing_next_step_returns_error() -> Result<(), String> {
    // Build a workflow where the Do node has next: None (no successor step).
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("missing_next_gap"),
        digest: WorkflowDigest::from_bytes([0xE1; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None, // GAP: no next step
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    // Suspend the Do node.
    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };

    // Call resume_action_completion — should hit MissingNextStep error.
    let result = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(99),
        Taint::Clean,
    );

    match result {
        Err(EngineError::MissingNextStep { step }) => {
            ensure_equal(step, StepIdx::new(0))?;
        }
        Err(other) => Err(format!("expected MissingNextStep, got {other:?}"))?,
        Ok(_) => Err("expected Err(MissingNextStep), got Ok".to_string())?,
    }

    // The step should still be Running (never marked succeeded or failed).
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

// ---------------------------------------------------------------------------
// validate_step_ceilings — boundary values
// ---------------------------------------------------------------------------

/// GAP-BUDGET-001: max_step_budget_per_tick = 0 returns StepCeilingExceeded.
#[test]
fn validate_step_ceilings_zero_step_budget_returns_error() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 1,
        ..AggregateResourceBudget::default()
    };

    let result = crate::budget::validate_step_ceilings(&budget);
    match result {
        Err(AggregateBudgetError::StepCeilingExceeded { requested, limit }) => {
            ensure_equal(requested, 0)?;
            ensure_equal(limit, 1_000_000)
        }
        Err(other) => Err(format!("expected StepCeilingExceeded, got {other:?}"))?,
        Ok(_) => Err("expected Err(StepCeilingExceeded), got Ok".to_string())?,
    }
}

/// GAP-BUDGET-002: max_step_budget_per_tick = HARD_MAX (1_000_000) is valid.
#[test]
fn validate_step_ceilings_exact_limit_step_budget_ok() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1_000_000,
        max_transitions_per_tick: 1_000_000,
        ..AggregateResourceBudget::default()
    };

    let result = crate::budget::validate_step_ceilings(&budget);
    result.map_err(|e| format!("exact limit should be Ok, got {e:?}"))
}

/// GAP-BUDGET-003: max_step_budget_per_tick = 1_000_001 exceeds HARD_MAX.
#[test]
fn validate_step_ceilings_over_limit_step_budget_returns_error() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1_000_001,
        max_transitions_per_tick: 1,
        ..AggregateResourceBudget::default()
    };

    let result = crate::budget::validate_step_ceilings(&budget);
    match result {
        Err(AggregateBudgetError::StepCeilingExceeded { requested, limit }) => {
            ensure_equal(requested, 1_000_001)?;
            ensure_equal(limit, 1_000_000)
        }
        Err(other) => Err(format!("expected StepCeilingExceeded, got {other:?}"))?,
        Ok(_) => Err("expected Err(StepCeilingExceeded), got Ok".to_string())?,
    }
}

/// GAP-BUDGET-004: max_transitions_per_tick = 0 returns PerTickCeilingExceeded.
#[test]
fn validate_step_ceilings_zero_transitions_returns_error() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 0,
        ..AggregateResourceBudget::default()
    };

    let result = crate::budget::validate_step_ceilings(&budget);
    match result {
        Err(AggregateBudgetError::PerTickCeilingExceeded { requested, limit }) => {
            ensure_equal(requested, 0)?;
            ensure_equal(limit, 1_000_000)
        }
        Err(other) => Err(format!("expected PerTickCeilingExceeded, got {other:?}"))?,
        Ok(_) => Err("expected Err(PerTickCeilingExceeded), got Ok".to_string())?,
    }
}

/// GAP-BUDGET-005: max_transitions_per_tick = HARD_MAX (1_000_000) is valid.
#[test]
fn validate_step_ceilings_exact_limit_transitions_ok() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1_000_000,
        ..AggregateResourceBudget::default()
    };

    let result = crate::budget::validate_step_ceilings(&budget);
    result.map_err(|e| format!("exact limit should be Ok, got {e:?}"))
}

/// GAP-BUDGET-006: max_transitions_per_tick = 1_000_001 exceeds HARD_MAX.
#[test]
fn validate_step_ceilings_over_limit_transitions_returns_error() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1_000_001,
        ..AggregateResourceBudget::default()
    };

    let result = crate::budget::validate_step_ceilings(&budget);
    match result {
        Err(AggregateBudgetError::PerTickCeilingExceeded { requested, limit }) => {
            ensure_equal(requested, 1_000_001)?;
            ensure_equal(limit, 1_000_000)
        }
        Err(other) => Err(format!("expected PerTickCeilingExceeded, got {other:?}"))?,
        Ok(_) => Err("expected Err(PerTickCeilingExceeded), got Ok".to_string())?,
    }
}

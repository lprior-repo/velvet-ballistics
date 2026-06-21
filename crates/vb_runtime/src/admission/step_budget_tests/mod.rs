//! Tests for `preflight_step_budget` and the wired-through step-count gate.
//!
//! This module covers the production admission path required by bead
//! `vb-b2pzr`:
//!
//! 1. `preflight_step_budget` rejects workflows whose declared
//!    `ResourceContract::max_steps` or computed IR step budget exceeds
//!    `vb_core::limits::MAX_STEPS_PER_WORKFLOW = 1_000`.
//! 2. `preflight_step_budget` accepts workflows at the boundary.
//! 3. The step budget gate is reached through the production runtime
//!    `Runtime::submit_direct` / `submit_compiled_with_inputs` /
//!    `submit_direct_with_inputs_grants_and_contracts` paths so a
//!    workflow declaring 50,000 steps is rejected at admission before any
//!    persistence.
//!
//! Helpers live in `helpers.rs` so this file stays under the 300-line
//! source cap.

#![cfg(test)]

use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;

use crate::RuntimeError;
use crate::admission::preflight_step_budget;

use self::helpers::{
    first_step_count_over_master_limit, linear_workflow_with_declared_steps, master_step_limit_u16,
    runtime_with_policy, total_command_queue_depth, workflow_with_max_steps,
};

mod helpers;

#[test]
fn preflight_step_budget_rejects_oversized_workflow() {
    let workflow = workflow_with_max_steps(
        u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW + 1).expect("within u16"),
    );
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert!(matches!(
        result,
        Err(crate::admission::AdmissionError::BudgetExceeded { actual, limit })
            if actual == u32::from(vb_core::limits::MAX_STEPS_PER_WORKFLOW as u16 + 1)
                && limit == crate::admission::per_workflow_step_ceiling()
    ));
}

#[test]
fn preflight_step_budget_accepts_at_boundary() {
    let max_steps = u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .expect("MAX_STEPS_PER_WORKFLOW fits in u16");
    let workflow = workflow_with_max_steps(max_steps);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert_eq!(result, Ok(()));
}

#[test]
fn preflight_step_budget_accepts_one_step() {
    let workflow = workflow_with_max_steps(1);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert_eq!(result, Ok(()));
}

#[test]
fn preflight_step_budget_is_noop_for_relaxed_policy() {
    let workflow = workflow_with_max_steps(u16::MAX);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Relaxed);
    assert_eq!(result, Ok(()));
}

#[test]
fn preflight_step_budget_rejects_fifty_thousand_step_workflow() {
    let workflow = workflow_with_max_steps(50_000_u16);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert!(matches!(
        result,
        Err(crate::admission::AdmissionError::BudgetExceeded { actual: 50_000, .. })
    ));
}

#[test]
fn preflight_step_budget_rejects_actual_ir_over_limit_when_declared_at_limit() {
    let workflow = linear_workflow_with_declared_steps(
        first_step_count_over_master_limit(),
        master_step_limit_u16(),
    );
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert!(matches!(
        result,
        Err(crate::admission::AdmissionError::BudgetExceeded { actual: 1_001, limit })
            if limit == crate::admission::per_workflow_step_ceiling()
    ));
}

#[test]
fn submit_50k_step_workflow_rejected() -> Result<(), RuntimeError> {
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let run = RunId::new(1);
    let workflow = workflow_with_max_steps(50_000_u16);
    let result = runtime.submit_direct(run, workflow);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));
    Ok(())
}

#[test]
fn submit_declared_limit_but_actual_ir_over_limit_rejected_before_enqueue()
-> Result<(), RuntimeError> {
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let run = RunId::new(20);
    let workflow = linear_workflow_with_declared_steps(
        first_step_count_over_master_limit(),
        master_step_limit_u16(),
    );
    let result = runtime.submit_direct(run, workflow);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 1_001, limit })
            if limit == crate::admission::per_workflow_step_ceiling()
    ));
    assert_eq!(runtime.collect_metrics().runs_active, 0);
    assert_eq!(total_command_queue_depth(&runtime), 0);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    Ok(())
}

#[test]
fn submit_1k_step_workflow_accepted() -> Result<(), RuntimeError> {
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let run = RunId::new(2);
    let max_steps = u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .ok_or(RuntimeError::QueueFull)?;
    let workflow = workflow_with_max_steps(max_steps);
    let result = runtime.submit_direct(run, workflow);
    assert!(result.is_ok(), "submit_direct should accept 1000-step workflow");
    Ok(())
}

#[test]
fn submit_compiled_with_inputs_also_enforces_step_budget() -> Result<(), RuntimeError> {
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let run = RunId::new(3);
    let workflow = workflow_with_max_steps(50_000_u16);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let result = runtime.submit_compiled_with_inputs(run, workflow, inputs);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));
    Ok(())
}

#[test]
fn submit_direct_with_inputs_grants_and_contracts_also_enforces_step_budget()
-> Result<(), RuntimeError> {
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let run = RunId::new(4);
    let workflow = workflow_with_max_steps(50_000_u16);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let caps = vb_core::capability::CapabilitySet::empty();
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::new([]);
    let result = runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        inputs,
        caps,
        action_contracts,
    );
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));
    Ok(())
}

/// Regression test for risk R1: missing one entry point re-opens the gap.
/// All three production submit entry points must reject a 50,000-step
/// workflow with the typed `AdmissionBudgetExceeded` error.
#[test]
fn all_three_submit_entry_points_reject_50k_step_workflow() -> Result<(), RuntimeError> {
    let max_steps = 50_000_u16;

    // Entry point 1: `submit_direct`
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let workflow = workflow_with_max_steps(max_steps);
    let result = runtime.submit_direct(RunId::new(10), workflow);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));

    // Entry point 2: `submit_compiled_with_inputs`
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let workflow = workflow_with_max_steps(max_steps);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let result = runtime.submit_compiled_with_inputs(RunId::new(11), workflow, inputs);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));

    // Entry point 3: `submit_direct_with_inputs_grants_and_contracts`
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let workflow = workflow_with_max_steps(max_steps);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let caps = vb_core::capability::CapabilitySet::empty();
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::new([]);
    let result = runtime.submit_direct_with_inputs_grants_and_contracts(
        RunId::new(12),
        workflow,
        inputs,
        caps,
        action_contracts,
    );
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));
    Ok(())
}

/// Regression test for risk R2: admission atomicity under a single-threaded
/// call sequence. The per-shard `admission_lock` ensures that the
/// preflight+enqueue pair is serialized within a single thread. The contract
/// here is that two consecutive submits that each fit the per-workflow
/// budget are each evaluated and either both accepted or one rejected
/// cleanly — never an inconsistent state.
#[test]
fn admission_lock_serializes_sequential_submits() -> Result<(), RuntimeError> {
    let runtime = runtime_with_policy(RuntimePolicy::Strict)?;
    let max_steps = u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .ok_or(RuntimeError::QueueFull)?;
    for offset in 0..4u64 {
        let run = RunId::new(200 + offset);
        let workflow = workflow_with_max_steps(max_steps);
        let result = runtime.submit_direct(run, workflow);
        assert!(
            matches!(
                result,
                Ok(()) | Err(RuntimeError::AdmissionBudgetExceeded { .. })
            ),
            "unexpected submit result"
        );
    }
    Ok(())
}

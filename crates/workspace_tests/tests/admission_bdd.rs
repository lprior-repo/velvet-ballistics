#![forbid(unsafe_code)]
//! BDD scenarios for the runtime admission step-budget gate.
//!
//! These tests encode the master contract `velvet-ballistics-MASTER.md` §13
//! "Steps | 1000" as executable Given/When/Then scenarios. They are the
//! behavioral counterparts to the unit tests in
//! `crates/vb_runtime/src/admission/step_budget_tests.rs`.
//!
//! The preflight is the typed step-count gate that the production
//! admission path uses. The full `Runtime::submit_direct` integration
//! is covered by the in-crate unit tests in `step_budget_tests.rs` (which
//! use a private artifact store constructor); from outside the
//! `vb_runtime` crate, the typed preflight is the public budget-gate
//! API that this BDD file exercises.
//!
//! Master contract references:
//! - `velvet-ballistics-MASTER.md` §13 line 479: Steps | 1000.
//! - `velvet-ballistics-MASTER.md` §20: admission enforces the budget
//!   before persistence.

use vb_core::ids::StepIdx;
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use vb_runtime::admission::preflight_step_budget;

/// Builds a `CompiledWorkflow` whose `ResourceContract::max_steps` is the
/// supplied value. The node graph is a single `Nop` because the budget
/// preflight only inspects `max_steps`.
fn workflow_with_max_steps(max_steps: u16) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("admission_bdd_max_steps_{max_steps}").into(),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract {
            max_steps,
            ..ResourceContract::DEFAULT
        },
        step_names: Box::from([Box::from("s0")]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

/// **Scenario: oversized workflow is rejected at admission**
///
/// - **Given** a workflow with 50,000 steps (above the master contract
///   per-workflow ceiling of 1,000).
/// - **When** the workflow is presented to the production admission
///   preflight under `RuntimePolicy::Strict`.
/// - **Then** the preflight fails with the typed
///   `AdmissionError::BudgetExceeded { actual: 50_000, limit: 1_000 }` so
///   the runtime fails closed before any persistence (master §20).
#[test]
fn given_oversized_workflow_when_submitted_then_admission_rejects() {
    // Given a workflow with 50,000 steps.
    let workflow = workflow_with_max_steps(50_000_u16);

    // When the workflow is presented to the preflight under strict policy.
    let preflight = preflight_step_budget(&workflow, RuntimePolicy::Strict);

    // Then the preflight fails with the typed BudgetExceeded error.
    assert!(
        matches!(
            preflight,
            Err(vb_runtime::admission::AdmissionError::BudgetExceeded {
                actual: 50_000,
                limit: 1_000,
            })
        ),
        "preflight_step_budget should reject 50,000-step workflow with BudgetExceeded, got {preflight:?}"
    );
}

/// **Scenario: workflow at the step-count ceiling is admitted**
///
/// - **Given** a workflow whose `max_steps` is exactly
///   `MAX_STEPS_PER_WORKFLOW = 1_000`.
/// - **When** the workflow is presented to the preflight under
///   `RuntimePolicy::Strict`.
/// - **Then** the preflight returns `Ok(())` because the limit is
///   inclusive.
#[test]
fn given_at_limit_workflow_when_submitted_then_admission_admits() {
    // Given a workflow with exactly 1,000 steps.
    let workflow = workflow_with_max_steps(
        u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
            .expect("MAX_STEPS_PER_WORKFLOW fits in u16"),
    );

    // When the workflow is presented to the preflight.
    let preflight = preflight_step_budget(&workflow, RuntimePolicy::Strict);

    // Then the preflight admits the workflow (limit is inclusive).
    assert_eq!(preflight, Ok(()));
}

/// **Scenario: relaxed policy bypasses the step-budget gate**
///
/// Documents the master contract §20 carve-out: under `Relaxed` policy
/// the step-budget preflight is a no-op so dev/test workflows are not
/// blocked by the ceiling. The preflight returns `Ok(())` regardless of
/// the step count.
#[test]
fn given_relaxed_policy_when_oversized_workflow_then_preflight_admits() {
    // Given an oversized workflow and a relaxed policy.
    let workflow = workflow_with_max_steps(50_000_u16);

    // When the preflight runs.
    let preflight = preflight_step_budget(&workflow, RuntimePolicy::Relaxed);

    // Then the relaxed policy admits the workflow (no step-budget gate).
    assert_eq!(preflight, Ok(()));
}

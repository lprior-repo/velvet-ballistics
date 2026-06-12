#![forbid(unsafe_code)]
//! BDD scenarios for the runtime admission step-budget gate.
//!
//! These tests encode the master contract `velvet-ballistics-MASTER.md` §13
//! "Steps | 1000" as executable Given/When/Then scenarios. They are the
//! behavioral counterparts to the unit tests in
//! `crates/vb_runtime/src/admission/step_budget_tests.rs`.
//!
//! The preflight is the typed step-count gate that the production
//! admission path uses. It validates both the declared resource contract
//! and the computed compiled IR budget. The full `Runtime::submit_direct`
//! integration is covered by the in-crate unit tests in
//! `step_budget_tests.rs` (which use a private artifact store constructor);
//! from outside the `vb_runtime` crate, the typed preflight is the public
//! budget-gate API that this BDD file exercises.
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
/// supplied value. The node graph is a single `Nop` so this helper isolates
/// declared-contract admission behavior.
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

/// Builds an adversarial compiled workflow whose actual linear IR contains
/// `node_count` reachable `Nop` nodes while the declared resource contract
/// reports only `declared_max_steps`.
fn linear_workflow_with_declared_steps(
    node_count: u16,
    declared_max_steps: u16,
) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("admission_bdd_actual_{node_count}_declared_{declared_max_steps}").into(),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0xB1; 32]),
        nodes: linear_nodes(node_count),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract {
            max_steps: declared_max_steps,
            ..ResourceContract::DEFAULT
        },
        step_names: linear_step_names(node_count),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

fn linear_nodes(node_count: u16) -> Box<[CompiledNode]> {
    let mut nodes = Vec::with_capacity(usize::from(node_count));
    let mut index = 0u16;
    while index < node_count {
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: next_linear_step(index, node_count),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        index = index.saturating_add(1);
    }
    nodes.into_boxed_slice()
}

fn next_linear_step(index: u16, node_count: u16) -> Option<StepIdx> {
    match index.checked_add(1) {
        Some(next) if next < node_count => Some(StepIdx::new(next)),
        _ => None,
    }
}

fn linear_step_names(node_count: u16) -> Box<[Box<str>]> {
    let mut names = Vec::with_capacity(usize::from(node_count));
    let mut index = 0u16;
    while index < node_count {
        names.push(format!("s{index}").into_boxed_str());
        index = index.saturating_add(1);
    }
    names.into_boxed_slice()
}

fn master_step_limit_u16() -> u16 {
    match u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW) {
        Ok(value) => value,
        Err(_) => u16::MAX,
    }
}

fn first_step_count_over_master_limit() -> u16 {
    match vb_core::limits::MAX_STEPS_PER_WORKFLOW.checked_add(1) {
        Some(value) => match u16::try_from(value) {
            Ok(converted) => converted,
            Err(_) => u16::MAX,
        },
        None => u16::MAX,
    }
}

/// **Scenario: oversized workflow is rejected at admission**
///
/// - **Given** a workflow declaring 50,000 steps (above the master contract
///   per-workflow ceiling of 1,000).
/// - **When** the workflow is presented to the production admission
///   preflight under `RuntimePolicy::Strict`.
/// - **Then** the preflight fails with the typed
///   `AdmissionError::BudgetExceeded { actual: 50_000, limit: 1_000 }` so
///   the runtime fails closed before any persistence (master §20).
#[test]
fn given_oversized_workflow_when_submitted_then_admission_rejects() {
    // Given a workflow declaring 50,000 steps.
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

/// **Scenario: actual compiled IR budget outranks a false declaration**
///
/// - **Given** a workflow whose declared `ResourceContract::max_steps` is at
///   the 1,000-step ceiling but whose compiled IR has 1,001 reachable steps.
/// - **When** the workflow is presented to the production admission
///   preflight under `RuntimePolicy::Strict`.
/// - **Then** the preflight fails with the typed
///   `AdmissionError::BudgetExceeded { actual: 1_001, limit: 1_000 }` so the
///   runtime does not trust a stale or adversarial resource declaration.
#[test]
fn given_declared_limit_but_actual_ir_over_limit_when_submitted_then_admission_rejects() {
    // Given a workflow whose declaration is within bounds but whose IR is not.
    let workflow = linear_workflow_with_declared_steps(
        first_step_count_over_master_limit(),
        master_step_limit_u16(),
    );

    // When the workflow is presented to the preflight under strict policy.
    let preflight = preflight_step_budget(&workflow, RuntimePolicy::Strict);

    // Then the computed IR budget, not the false declaration, controls admission.
    assert!(
        matches!(
            preflight,
            Err(vb_runtime::admission::AdmissionError::BudgetExceeded {
                actual: 1_001,
                limit: 1_000,
            })
        ),
        "preflight_step_budget should reject actual 1,001-step IR with BudgetExceeded, got {preflight:?}"
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

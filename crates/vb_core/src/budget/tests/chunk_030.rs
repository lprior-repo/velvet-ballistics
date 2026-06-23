use crate::budget::{BudgetError, WholeWorkflowBudget};
use crate::errors::CoreError;
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{CompiledNodeKind, WorkflowError};

use super::super::traversal::BudgetTraversalError;
use super::prelude::{ensure_equal, test_contract, test_node};

#[test]
fn budget_error_from_expression_invalid_workflow_preserves_reason() -> Result<(), String> {
    let budget_err: BudgetError = WorkflowError::Expression(CoreError::InvalidCompiledWorkflow {
        reason: "expression invalid workflow reason",
    })
    .into();

    match budget_err {
        BudgetError::InvalidCompiledWorkflow { reason } => {
            ensure_equal(reason, "expression invalid workflow reason")
        }
        other => Err(format!(
            "expected InvalidCompiledWorkflow reason, got {other:?}"
        )),
    }
}

#[test]
fn budget_error_from_resource_contract_variants_preserves_resource() -> Result<(), String> {
    let exceeded: BudgetError = WorkflowError::ResourceContractExceeded {
        resource: "max_steps",
    }
    .into();
    let too_large: BudgetError = WorkflowError::ResourceContractTooLarge {
        resource: "max_slots",
    }
    .into();

    match exceeded {
        BudgetError::ResourceContractExceeded { resource } => ensure_equal(resource, "max_steps")?,
        other => return Err(format!("expected ResourceContractExceeded, got {other:?}")),
    }
    match too_large {
        BudgetError::ResourceContractTooLarge { resource } => ensure_equal(resource, "max_slots"),
        other => Err(format!("expected ResourceContractTooLarge, got {other:?}")),
    }
}

#[test]
fn budget_error_from_policy_detail_preserves_detail() -> Result<(), String> {
    let budget_err: BudgetError = WorkflowError::BudgetPolicyExceeded {
        detail: "max_parallel_in_flight",
    }
    .into();

    match budget_err {
        BudgetError::BudgetPolicyExceeded { detail } => {
            ensure_equal(detail, "max_parallel_in_flight")
        }
        other => Err(format!("expected BudgetPolicyExceeded, got {other:?}")),
    }
}

#[test]
fn budget_error_from_traversal_invalid_workflow_preserves_reason() -> Result<(), String> {
    let traversal_err = BudgetTraversalError::InvalidCompiledWorkflow {
        reason: "done continuation missing",
    };
    let budget_err: BudgetError = traversal_err.into();

    match budget_err {
        BudgetError::InvalidCompiledWorkflow { reason } => {
            ensure_equal(reason, "done continuation missing")
        }
        other => Err(format!(
            "expected traversal InvalidCompiledWorkflow reason, got {other:?}"
        )),
    }
}

#[test]
fn budget_rejects_done_last_without_next_when_done_is_non_terminal() -> Result<(), String> {
    let nodes = vec![
        test_node(
            0,
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(0),
                limit: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        ),
        test_node(1, None, CompiledNodeKind::Nop),
        test_node(
            2,
            None,
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(0),
            },
        ),
    ];
    let contract = test_contract(3, 1);

    match WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract) {
        Err(WorkflowError::Expression(CoreError::InvalidCompiledWorkflow { reason })) => {
            ensure_equal(
                reason,
                "loop done node requires explicit continuation unless it is terminal",
            )
        }
        other => Err(format!(
            "expected invalid compiled workflow for missing done continuation, got {other:?}"
        )),
    }
}

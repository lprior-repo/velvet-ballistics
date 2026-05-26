#![forbid(unsafe_code)]
//! Resource limit validation for workflow documents.

#![allow(unreachable_pub)]
//!
//! Validates that a workflow's declared resource contract stays within
//! protocol hard limits.

use crate::{ValidationError, ValidationResult};
use vb_core::span::Span;

use crate::type_sigs::{ResourceLimits, WorkflowTypes};

/// Validates resource contract bounds against protocol hard limits.
pub fn validate_resource_limits(
    workflow: &WorkflowTypes,
    hard_limits: &ResourceLimits,
) -> ValidationResult<()> {
    check_resource_bound(
        "max_steps",
        workflow.steps.len(),
        workflow.resource_contract.max_steps,
        hard_limits.max_steps,
    )?;
    check_resource_bound(
        "max_slots",
        workflow.steps.len(),
        workflow.resource_contract.max_slots,
        hard_limits.max_slots,
    )?;
    check_resource_bound(
        "max_constants",
        0,
        workflow.resource_contract.max_constants,
        hard_limits.max_constants,
    )?;
    check_declared_bound(
        "max_accessors",
        workflow.resource_contract.max_accessors,
        hard_limits.max_accessors,
    )?;
    check_declared_bound(
        "max_expressions",
        workflow.resource_contract.max_expressions,
        hard_limits.max_expressions,
    )?;
    check_declared_bound(
        "max_expr_stack",
        workflow.resource_contract.max_expr_stack,
        hard_limits.max_expr_stack,
    )?;
    check_declared_bound(
        "max_step_budget_per_tick",
        workflow.resource_contract.max_step_budget_per_tick,
        hard_limits.max_step_budget_per_tick,
    )?;
    check_declared_bound(
        "max_input_bytes",
        workflow.resource_contract.max_input_bytes,
        hard_limits.max_input_bytes,
    )?;
    check_declared_bound(
        "max_output_bytes",
        workflow.resource_contract.max_output_bytes,
        hard_limits.max_output_bytes,
    )?;
    check_declared_bound(
        "max_blob_bytes",
        workflow.resource_contract.max_blob_bytes,
        hard_limits.max_blob_bytes,
    )?;
    check_declared_bound(
        "max_ipc_payload_bytes",
        workflow.resource_contract.max_ipc_payload_bytes,
        hard_limits.max_ipc_payload_bytes,
    )?;
    check_declared_bound(
        "max_retry_attempts",
        workflow.resource_contract.max_retry_attempts,
        hard_limits.max_retry_attempts,
    )?;
    check_declared_bound(
        "max_fanout",
        workflow.resource_contract.max_fanout,
        hard_limits.max_fanout,
    )?;
    check_declared_bound(
        "max_collect_items",
        workflow.resource_contract.max_collect_items,
        hard_limits.max_collect_items,
    )?;
    check_declared_bound(
        "max_queue_depth",
        workflow.resource_contract.max_queue_depth,
        hard_limits.max_queue_depth,
    )?;
    check_declared_bound(
        "max_journal_batch_bytes",
        workflow.resource_contract.max_journal_batch_bytes,
        hard_limits.max_journal_batch_bytes,
    )
}

fn check_resource_bound(
    resource: &str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> ValidationResult<()> {
    check_declared_bound(resource, declared, hard_limit)?;
    if actual > declared {
        return Err(ValidationError::LimitExceeded {
            resource: resource.to_owned(),
            span: Span::ZERO,
        });
    }
    Ok(())
}

fn check_declared_bound(
    resource: &str,
    declared: usize,
    hard_limit: usize,
) -> ValidationResult<()> {
    if declared == 0 {
        return Err(ValidationError::LimitRequired {
            resource: resource.to_owned(),
            span: Span::ZERO,
        });
    }
    if declared > hard_limit {
        return Err(ValidationError::LimitExceeded {
            resource: resource.to_owned(),
            span: Span::ZERO,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_sigs::{StepKind, StepTypes, TypedValue, ValueType};
    use vb_core::span::Span;

    fn make_workflow(steps: Vec<StepTypes>) -> WorkflowTypes {
        WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps,
            resource_contract: ResourceLimits::default(),
        }
    }

    fn finish_step(id: &str, result: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Finish { result },
        }
    }

    // -- Pass cases --

    #[test]
    fn accepts_within_bounds() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        assert_eq!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Ok(())
        );
    }

    #[test]
    fn accepts_empty_workflow() {
        let wf = make_workflow(vec![]);
        assert_eq!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Ok(())
        );
    }

    #[test]
    fn accepts_declared_equals_hard() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits::default(),
        };
        assert_eq!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Ok(())
        );
    }

    #[test]
    fn accepts_declared_below_hard() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_steps: 500,
                ..ResourceLimits::default()
            },
        };
        assert_eq!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Ok(())
        );
    }

    // -- Fail cases --

    #[test]
    fn rejects_zero_declared_limit() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_fanout: 0,
                ..ResourceLimits::default()
            },
        };
        assert!(matches!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Err(ValidationError::LimitRequired { .. })
        ));
    }

    #[test]
    fn rejects_declared_exceeding_hard() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_steps: 100,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits {
            max_steps: 50,
            ..ResourceLimits::default()
        };
        assert_eq!(
            validate_resource_limits(&wf, &hard),
            Err(ValidationError::LimitExceeded {
                resource: "max_steps".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn rejects_actual_exceeding_declared() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![finish_step("s1", TypedValue::Literal(ValueType::Number)); 10],
            resource_contract: ResourceLimits {
                max_steps: 5,
                ..ResourceLimits::default()
            },
        };
        assert!(matches!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Err(ValidationError::LimitExceeded { .. })
        ));
    }

    #[test]
    fn rejects_fanout_exceeding_hard() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_fanout: 64,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits {
            max_fanout: 32,
            ..ResourceLimits::default()
        };
        assert_eq!(
            validate_resource_limits(&wf, &hard),
            Err(ValidationError::LimitExceeded {
                resource: "max_fanout".to_owned(),
                span: Span::ZERO
            })
        );
    }

    #[test]
    fn rejects_zero_max_steps() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_steps: 0,
                ..ResourceLimits::default()
            },
        };
        assert!(matches!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Err(ValidationError::LimitRequired { .. })
        ));
    }

    #[test]
    fn rejects_zero_max_slots() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_slots: 0,
                ..ResourceLimits::default()
            },
        };
        assert!(matches!(
            validate_resource_limits(&wf, &ResourceLimits::default()),
            Err(ValidationError::LimitRequired { .. })
        ));
    }
}

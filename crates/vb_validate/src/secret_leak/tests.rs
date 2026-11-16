//! Tests for resource limit validation.

use crate::ValidationError;
use crate::type_taint::{
    ResourceLimits, StepKind, StepTypes, TypedValue, ValueType, WorkflowTypes,
    validate_resource_limits,
};

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

#[test]
fn fixture_resource_contract_defaults_are_preserved() {
    let legacy = crate::type_sigs::fixtures::make_workflow(vec![]).resource_contract;
    let production = make_workflow(vec![]).resource_contract;
    assert_eq!(legacy.max_steps, production.max_steps, "max_steps");
    assert_eq!(legacy.max_slots, production.max_slots, "max_slots");
    assert_eq!(
        legacy.max_constants, production.max_constants,
        "max_constants"
    );
    assert_eq!(
        legacy.max_accessors, production.max_accessors,
        "max_accessors"
    );
    assert_eq!(
        legacy.max_expressions, production.max_expressions,
        "max_expressions"
    );
    assert_eq!(
        legacy.max_expr_stack, production.max_expr_stack,
        "max_expr_stack"
    );
    assert_eq!(
        legacy.max_step_budget_per_tick, production.max_step_budget_per_tick,
        "max_step_budget_per_tick"
    );
    assert_eq!(
        legacy.max_input_bytes, production.max_input_bytes,
        "max_input_bytes"
    );
    assert_eq!(
        legacy.max_output_bytes, production.max_output_bytes,
        "max_output_bytes"
    );
    assert_eq!(
        legacy.max_blob_bytes, production.max_blob_bytes,
        "max_blob_bytes"
    );
    assert_eq!(
        legacy.max_ipc_payload_bytes, production.max_ipc_payload_bytes,
        "max_ipc_payload_bytes"
    );
    assert_eq!(
        legacy.max_retry_attempts, production.max_retry_attempts,
        "max_retry_attempts"
    );
    assert_eq!(legacy.max_fanout, production.max_fanout, "max_fanout");
    assert_eq!(
        legacy.max_collect_items, production.max_collect_items,
        "max_collect_items"
    );
    assert_eq!(
        legacy.max_queue_depth, production.max_queue_depth,
        "max_queue_depth"
    );
    assert_eq!(
        legacy.max_journal_batch_bytes, production.max_journal_batch_bytes,
        "max_journal_batch_bytes"
    );
}

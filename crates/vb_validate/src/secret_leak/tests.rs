//! Tests for resource limit validation.

#[cfg(test)]
use crate::secret_leak::validate_resource_limits;
#[cfg(test)]
use crate::type_sigs::{ResourceLimits, StepKind, StepTypes, TypedValue, ValueType, WorkflowTypes};
#[cfg(test)]
use crate::{ValidationError, ValidationResult};

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

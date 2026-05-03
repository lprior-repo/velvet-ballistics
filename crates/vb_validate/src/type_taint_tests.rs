//! Tests for type_taint module.
//!
//! These tests verify type checking, taint propagation, and resource limit
//! validation for workflow documents.

use crate::type_sigs::{
    InputDecl, ResourceLimits, StepKind, StepTypes, Taint, TypedValue, ValueFact, ValueType,
    WorkflowTypes,
};
use crate::type_check::validate_types;
use crate::taint_prop::validate_taint;
use crate::secret_leak::validate_resource_limits;
use crate::ValidationError;

fn make_workflow(steps: Vec<StepTypes>) -> WorkflowTypes {
    WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        resource_contract: ResourceLimits::default(),
    }
}

fn save_step(id: &str, value: TypedValue) -> StepTypes {
    StepTypes {
        id: id.to_owned(),
        kind: StepKind::Save { value },
    }
}

fn choose_step(id: &str, condition: TypedValue) -> StepTypes {
    StepTypes {
        id: id.to_owned(),
        kind: StepKind::Choose { condition },
    }
}

fn finish_step(id: &str, result: TypedValue) -> StepTypes {
    StepTypes {
        id: id.to_owned(),
        kind: StepKind::Finish { result },
    }
}

#[test]
fn accepts_clean_finish() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn rejects_secret_finish_direct() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.token".into()),
    )]);
    wf.secrets.push("token".to_owned());
    assert!(matches!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak)
    ));
}

#[test]
fn rejects_secret_finish_via_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert!(matches!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak)
    ));
}

#[test]
fn accepts_boolean_choose() {
    let wf = make_workflow(vec![
        save_step("flag", TypedValue::Literal(ValueType::Boolean)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_types(&wf), Ok(()));
}

#[test]
fn rejects_number_choose() {
    let wf = make_workflow(vec![
        save_step("flag", TypedValue::Literal(ValueType::Number)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert!(matches!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn accepts_literal_boolean_choose() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Boolean),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
}

#[test]
fn rejects_literal_text_choose() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Text),
    )]);
    assert!(matches!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn accepts_clean_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.user".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn rejects_secret_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.key".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert!(matches!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak)
    ));
}

#[test]
fn resource_limits_accept_within_bounds() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits::default();
    assert_eq!(validate_resource_limits(&wf, &hard), Ok(()));
}

#[test]
fn resource_limits_reject_exceeded_steps() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits {
        max_steps: 0,
        max_slots: 65_535,
        max_constants: 8_192,
        ..ResourceLimits::default()
    };
    assert!(matches!(
        validate_resource_limits(&wf, &hard),
        Err(ValidationError::LimitExceeded { .. })
    ));
}

#[test]
fn rejects_nested_secret_composite() {
    let mut wf = make_workflow(vec![
        save_step(
            "cap",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert!(matches!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak)
    ));
}

#[test]
fn accepts_any_type_choose() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Any)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_types(&wf), Ok(()));
}

#[test]
fn rejects_null_choose() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Null)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert!(matches!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn accepts_clean_var_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$vars.label".into()),
    )]);
    wf.vars.push(("label".to_owned(), ValueType::Boolean));
    assert_eq!(validate_taint(&wf), Ok(()));
}

// ---------------------------------------------------------------------------
// BDD exact-assertion tests
// ---------------------------------------------------------------------------

#[test]
fn validate_types_returns_type_mismatch_for_wrong_type() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Number)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        })
    );
}

#[test]
fn validate_taint_returns_secret_result_leak_for_secret_in_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn validate_taint_returns_forbidden_reference_to_untrusted_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn validate_resource_limits_accepts_within_limits() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_resource_limits_rejects_too_many_steps() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits {
        max_steps: 0,
        max_slots: 65_535,
        max_constants: 8_192,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        })
    );
}

#[test]
fn validate_resource_limits_rejects_declared_limit_exceeding_hard() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 100,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits {
        max_steps: 50,
        max_slots: 65_535,
        max_constants: 8_192,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        })
    );
}

#[test]
fn validate_resource_limits_returns_limit_required_for_zero_declared_runtime_limit() {
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
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_fanout".to_owned(),
        })
    );
}

#[test]
fn validate_resource_limits_rejects_declared_fanout_exceeding_hard() {
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
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_fanout".to_owned(),
        })
    );
}

#[test]
fn value_type_as_str_returns_correct_names() {
    assert_eq!(ValueType::Null.as_str(), "null");
    assert_eq!(ValueType::Boolean.as_str(), "boolean");
    assert_eq!(ValueType::Number.as_str(), "number");
    assert_eq!(ValueType::Text.as_str(), "text");
    assert_eq!(ValueType::Object.as_str(), "object");
    assert_eq!(ValueType::List.as_str(), "list");
    assert_eq!(ValueType::Any.as_str(), "any");
}

#[test]
fn taint_merge_propagates_secret() {
    let clean = Taint::Clean;
    let secret = Taint::Secret;
    assert_eq!(clean.merge(secret), Taint::Secret);
    assert_eq!(secret.merge(clean), Taint::Secret);
    assert_eq!(secret.merge(secret), Taint::Secret);
    assert_eq!(clean.merge(clean), Taint::Clean);
}

#[test]
fn value_fact_clean_creates_clean_taint() {
    let fact = ValueFact::clean(ValueType::Number);
    assert_eq!(fact.value_type, ValueType::Number);
    assert_eq!(fact.taint, Taint::Clean);
}

#[test]
fn value_fact_secret_creates_secret_taint() {
    let fact = ValueFact::secret(ValueType::Text);
    assert_eq!(fact.value_type, ValueType::Text);
    assert_eq!(fact.taint, Taint::Secret);
}

#[test]
fn validate_types_accepts_any_type_choose() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Any)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_types_rejects_null_choose_exact() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Null)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "null".to_owned(),
        })
    );
}

#[test]
fn validate_types_rejects_text_choose_exact() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Text),
    )]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "text".to_owned(),
        })
    );
}

#[test]
fn validate_types_accepts_literal_boolean_choose() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Boolean),
    )]);
    let result = validate_types(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_clean_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.user".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_rejects_secret_input_finish_exact() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.key".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn validate_taint_rejects_nested_secret_composite_exact() {
    let mut wf = make_workflow(vec![
        save_step(
            "cap",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn resource_limits_default_values() {
    let limits = ResourceLimits::default();
    assert_eq!(limits.max_steps, 1_000);
    assert_eq!(limits.max_slots, 65_535);
    assert_eq!(limits.max_constants, 8_192);
    assert_eq!(limits.max_accessors, 8_192);
    assert_eq!(limits.max_expressions, 4_096);
    assert_eq!(limits.max_expr_stack, 64);
    assert_eq!(limits.max_retry_attempts, 10);
    assert_eq!(limits.max_fanout, 256);
    assert_eq!(limits.max_collect_items, 1_000);
}

// ---------------------------------------------------------------------------
// Adversarial BDD tests: validation bypass attacks
// ---------------------------------------------------------------------------

#[test]
fn adversarial_secret_leak_via_direct_reference_in_finish_is_rejected() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn adversarial_secret_leak_via_two_step_indirection_is_rejected() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        save_step("relay", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn adversarial_secret_leak_via_composite_with_clean_and_secret_is_rejected() {
    let mut wf = make_workflow(vec![
        save_step(
            "mixed",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Reference("$secrets.password".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("password".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn adversarial_secret_leak_via_secret_input_is_rejected() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.password".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "password".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

#[test]
fn adversarial_type_mismatch_object_in_choose_is_rejected() {
    let wf = make_workflow(vec![choose_step(
        "bad_route",
        TypedValue::Literal(ValueType::Object),
    )]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "object".to_owned(),
        })
    );
}

#[test]
fn adversarial_type_mismatch_list_in_choose_is_rejected() {
    let wf = make_workflow(vec![choose_step(
        "bad_route",
        TypedValue::Literal(ValueType::List),
    )]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "list".to_owned(),
        })
    );
}

#[test]
fn adversarial_resource_limit_declared_exceeding_hard_limit_is_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 1_000,
            max_slots: 100_000,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_slots".to_owned(),
        })
    );
}

#[test]
fn adversarial_resource_limit_actual_exceeding_declared_is_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![finish_step("s1", TypedValue::Literal(ValueType::Number)); 10],
        resource_contract: ResourceLimits {
            max_steps: 5,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        })
    );
}

#[test]
fn adversarial_clean_input_passes_taint_check() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.username".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "username".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_in_choose_condition_does_not_leak_but_type_check_passes_for_any() {
    let mut wf = make_workflow(vec![
        save_step("val", TypedValue::Reference("$secrets.token".into())),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_types(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_deeply_nested_composite_taint_propagates() {
    let mut wf = make_workflow(vec![
        save_step(
            "nested",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Text),
                    TypedValue::Reference("$secrets.deep_secret".into()),
                ]),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("deep_secret".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

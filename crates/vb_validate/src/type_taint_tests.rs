#![forbid(unsafe_code)]
//! Tests for type_taint module (extracted from type_taint.rs)

use crate::type_taint::{
    InputDecl, ResourceLimits, StepKind, StepTypes, Taint, TypedValue, ValueFact, ValueType,
    WorkflowTypes, validate_resource_limits, validate_taint, validate_types,
};
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
    let result1 = clean.merge(secret);
    let result2 = secret.merge(clean);
    let result3 = secret.merge(secret);
    let result4 = clean.merge(clean);
    assert_eq!(result1, Taint::Secret);
    assert_eq!(result2, Taint::Secret);
    assert_eq!(result3, Taint::Secret);
    assert_eq!(result4, Taint::Clean);
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
                    TypedValue::Composite(vec![
                        TypedValue::Literal(ValueType::Boolean),
                        TypedValue::Reference("$secrets.deep_secret".into()),
                    ]),
                ]),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("deep_secret".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Err(ValidationError::SecretResultLeak));
}

// ===========================================================================
// Section 38 behavioral property tests
// ===========================================================================

#[test]
fn section38_taint_safety_secret_result_leak_direct_reference() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn section38_taint_safety_secret_result_leak_via_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn section38_taint_safety_secret_result_leak_via_input() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.credential".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "credential".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn section38_taint_safety_clean_finish_passes() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

// ===========================================================================
// Comprehensive taint propagation tests
// ===========================================================================

#[test]
fn taint_secret_save_marks_slot_tainted() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_secret_input_save_marks_slot_tainted() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$input.api_key".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "api_key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_clean_input_save_marks_slot_clean() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$input.username".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "username".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_clean_var_save_marks_slot_clean() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$vars.counter".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.vars.push(("counter".to_owned(), ValueType::Number));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_literal_save_marks_slot_clean() {
    let wf = make_workflow(vec![
        save_step("cap", TypedValue::Literal(ValueType::Text)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_propagates_through_three_hop_slot_chain() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.db_password".into())),
        save_step("relay1", TypedValue::Slot(0)),
        save_step("relay2", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("db_password".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_independent_slots_isolated_clean_finish() {
    let mut wf = make_workflow(vec![
        save_step("secret_cap", TypedValue::Reference("$secrets.key".into())),
        save_step("clean_cap", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("key".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_independent_slots_tainted_finish_fails() {
    let mut wf = make_workflow(vec![
        save_step("secret_cap", TypedValue::Reference("$secrets.key".into())),
        save_step("clean_cap", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("key".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_cross_slot_contamination_via_composite() {
    let mut wf = make_workflow(vec![
        save_step("secret_cap", TypedValue::Reference("$secrets.token".into())),
        save_step(
            "mixed",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Number),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_nested_slot_relay_with_composite() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.cred".into())),
        save_step("s1", TypedValue::Slot(0)),
        save_step(
            "s2",
            TypedValue::Composite(vec![
                TypedValue::Slot(1),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("cred".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_finish_direct_secret_reference_rejected() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.private_key".into()),
    )]);
    wf.secrets.push("private_key".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_finish_secret_input_reference_rejected() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.secret_value".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "secret_value".to_owned(),
        schema_type: ValueType::Number,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_finish_tainted_slot_rejected() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.session_id".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("session_id".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_finish_composite_with_secret_rejected() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Literal(ValueType::Number),
            TypedValue::Reference("$secrets.hidden".into()),
        ]),
    )]);
    wf.secrets.push("hidden".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_finish_composite_with_tainted_slot_rejected() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.bearer".into())),
        finish_step("done", TypedValue::Composite(vec![TypedValue::Slot(0)])),
    ]);
    wf.secrets.push("bearer".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_clean_finish_literal() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_clean_finish_clean_input_reference() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.email".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "email".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_clean_finish_clean_var_reference() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$vars.status".into()),
    )]);
    wf.vars.push(("status".to_owned(), ValueType::Text));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_clean_finish_clean_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$input.user_id".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "user_id".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_clean_finish_composite_of_clean() {
    let wf = make_workflow(vec![
        save_step("cap", TypedValue::Literal(ValueType::Number)),
        finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_clean_finish_with_unused_secrets_in_workflow() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.unused".into())),
        save_step("data", TypedValue::Literal(ValueType::Text)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("unused".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_expression_composite_with_secret_reference() {
    let mut wf = make_workflow(vec![
        save_step(
            "expr",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Reference("$secrets.otp".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("otp".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_expression_composite_with_tainted_slot() {
    let mut wf = make_workflow(vec![
        save_step("secret_cap", TypedValue::Reference("$secrets.hash".into())),
        save_step(
            "expr",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("hash".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_expression_composite_clean_remains_clean() {
    let wf = make_workflow(vec![
        save_step("clean_val", TypedValue::Literal(ValueType::Number)),
        save_step(
            "expr",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_expression_deeply_nested_composite_with_secret() {
    let mut wf = make_workflow(vec![
        save_step(
            "deep",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Text),
                    TypedValue::Composite(vec![
                        TypedValue::Literal(ValueType::Boolean),
                        TypedValue::Reference("$secrets.buried".into()),
                    ]),
                ]),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("buried".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_expression_composite_with_secret_input() {
    let mut wf = make_workflow(vec![
        save_step(
            "expr",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Reference("$input.password".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "password".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_expression_composite_with_relayed_taint() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.nonce".into())),
        save_step("s1", TypedValue::Slot(0)),
        save_step("expr", TypedValue::Composite(vec![TypedValue::Slot(1)])),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("nonce".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_expression_inline_composite_in_finish_rejected() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Literal(ValueType::Number),
            TypedValue::Reference("$secrets.inline_secret".into()),
        ]),
    )]);
    wf.secrets.push("inline_secret".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_multiple_secrets_only_one_used() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.alpha".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("alpha".to_owned());
    wf.secrets.push("beta".to_owned());
    wf.secrets.push("gamma".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_choose_with_secret_condition_no_finish_leak() {
    let mut wf = make_workflow(vec![
        save_step("val", TypedValue::Reference("$secrets.flag".into())),
        choose_step("route", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Literal(ValueType::Number)),
    ]);
    wf.secrets.push("flag".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_no_secrets_all_clean() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Number)),
        save_step("flag", TypedValue::Literal(ValueType::Boolean)),
        choose_step("route", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_unknown_reference_resolves_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.nonexistent".into()),
    )]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_composite_with_multiple_taint_sources() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.s".into())),
        save_step(
            "expr",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Reference("$input.cred".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("s".to_owned());
    wf.inputs.push(InputDecl {
        name: "cred".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn taint_composite_clean_with_declared_secrets() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.data".into())),
        save_step(
            "expr",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.inputs.push(InputDecl {
        name: "data".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    wf.secrets.push("unused_secret".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_propagates_through_deep_slot_chain() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.db_password".into())),
        save_step("s1", TypedValue::Slot(0)),
        save_step("s2", TypedValue::Slot(1)),
        save_step("s3", TypedValue::Slot(2)),
        save_step("s4", TypedValue::Slot(3)),
        save_step("s5", TypedValue::Slot(4)),
        finish_step("done", TypedValue::Slot(5)),
    ]);
    wf.secrets.push("db_password".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

#[test]
fn clean_deep_slot_chain_passes() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        save_step("s1", TypedValue::Slot(0)),
        save_step("s2", TypedValue::Slot(1)),
        save_step("s3", TypedValue::Slot(2)),
        save_step("s4", TypedValue::Slot(3)),
        finish_step("done", TypedValue::Slot(4)),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_mixed_slots_isolated_independent_reads() {
    let mut wf = make_workflow(vec![
        save_step("tainted", TypedValue::Reference("$secrets.api_key".into())),
        save_step("clean", TypedValue::Literal(ValueType::Text)),
        save_step("clean_relay", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("api_key".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn taint_mixed_slots_isolated_tainted_read_rejected() {
    let mut wf = make_workflow(vec![
        save_step("tainted", TypedValue::Reference("$secrets.api_key".into())),
        save_step("clean", TypedValue::Literal(ValueType::Text)),
        save_step("clean_relay", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("api_key".to_owned());
    assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
}

// =========================================================================
// BLACKHAT security regression tests
// =========================================================================

#[test]
fn blackhat_resource_limits_max_slots_uses_step_count_not_slot_count() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            finish_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("s1", TypedValue::Literal(ValueType::Number)),
        ],
        resource_contract: ResourceLimits {
            max_steps: 10,
            max_slots: 1,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    match validate_resource_limits(&wf, &hard) {
        Err(ValidationError::LimitExceeded { resource }) => {
            assert_eq!(resource, "max_slots");
        }
        Err(ValidationError::LimitRequired { .. }) => {}
        other => {
            assert!(
                matches!(
                    other,
                    Err(ValidationError::LimitExceeded { .. }
                        | ValidationError::LimitRequired { .. })
                ),
                "blackhat: expected LimitExceeded or LimitRequired for max_slots, got {other:?}"
            );
        }
    }
}

#[test]
fn blackhat_unknown_reference_is_clean_not_secret() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$unknown_root.field".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: unknown reference roots should resolve as clean"
    );

    let wf2 = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.nonexistent".into()),
    )]);
    assert_eq!(
        validate_taint(&wf2),
        Ok(()),
        "blackhat: unknown input name should resolve as clean"
    );
}

#[test]
fn blackhat_reference_without_dot_is_clean_any() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: reference without dot should be clean"
    );
}

#[test]
fn blackhat_reference_without_dollar_prefix_is_clean_text() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("not_a_reference".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: non-dollar reference should be clean"
    );
}

#[test]
fn blackhat_zero_declared_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_input_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert!(
        matches!(result, Err(ValidationError::LimitRequired { .. })),
        "blackhat: zero declared limit should be rejected, got {result:?}"
    );
    if let Err(ValidationError::LimitRequired { resource }) = result {
        assert_eq!(resource, "max_input_bytes");
    }
}

#[test]
fn blackhat_choose_tainted_condition_does_not_propagate() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.flag".into())),
        choose_step("route", TypedValue::Slot(0)),
        save_step("s1", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("flag".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: tainted choose condition should not propagate taint to finish"
    );
}

#[test]
fn blackhat_taint_merge_commutative() {
    assert_eq!(
        Taint::Clean.merge(Taint::Secret),
        Taint::Secret.merge(Taint::Clean),
        "blackhat: taint merge must be commutative"
    );
}

#[test]
fn blackhat_uninitialized_slot_is_clean() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(5)),
    ]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: uninitialized slot should resolve as clean"
    );
}

#[test]
fn blackhat_empty_composite_is_clean() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Composite(vec![])),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: empty composite should be clean"
    );
}

#[test]
fn blackhat_out_of_bounds_slot_write_no_panic() {
    let wf = make_workflow(vec![
        save_step("overflow", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

// =========================================================================
// Comprehensive taint propagation and type-checking tests
// =========================================================================

fn taint_propagates_through_arithmetic_style_composite() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "arithmetic_result",
            TypedValue::Composite(vec![
                TypedValue::Reference("$input.secret_salary".into()),
                TypedValue::Literal(ValueType::Number),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "secret_salary".to_owned(),
        schema_type: ValueType::Number,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for arithmetic composite, got {result:?}"
        ));
    }
    Ok(())
}

fn taint_propagates_through_comparison_style_composite() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "comparison_result",
            TypedValue::Composite(vec![
                TypedValue::Reference("$input.public_id".into()),
                TypedValue::Reference("$secrets.compare_key".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "public_id".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    wf.secrets.push("compare_key".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for comparison composite, got {result:?}"
        ));
    }
    Ok(())
}

fn taint_propagates_through_logic_style_composite() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("flag", TypedValue::Reference("$secrets.logic_val".into())),
        save_step(
            "logic_result",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Boolean),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("logic_val".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for logic composite, got {result:?}"
        ));
    }
    Ok(())
}

fn clean_composite_stays_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "clean_expr",
            TypedValue::Composite(vec![
                TypedValue::Reference("$input.a".into()),
                TypedValue::Reference("$input.b".into()),
                TypedValue::Literal(ValueType::Number),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "a".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    wf.inputs.push(InputDecl {
        name: "b".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for clean composite expression, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_taint_propagates_through_arithmetic_style_composite() {
    taint_propagates_through_arithmetic_style_composite().unwrap()
}

#[test]
fn run_taint_propagates_through_comparison_style_composite() {
    taint_propagates_through_comparison_style_composite().unwrap()
}

#[test]
fn run_taint_propagates_through_logic_style_composite() {
    taint_propagates_through_logic_style_composite().unwrap()
}

#[test]
fn run_clean_composite_stays_clean() {
    clean_composite_stays_clean().unwrap()
}

fn secret_origin_propagates_through_all_downstream_paths() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.api_key".into())),
        save_step("s1", TypedValue::Slot(0)),
        save_step(
            "s2",
            TypedValue::Composite(vec![
                TypedValue::Slot(1),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
        save_step("s3", TypedValue::Slot(2)),
        finish_step("done", TypedValue::Slot(3)),
    ]);
    wf.inputs.push(InputDecl {
        name: "api_key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for secret origin chain, got {result:?}"
        ));
    }
    Ok(())
}

fn secret_origin_relay_slot_is_tainted() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.secret_val".into())),
        save_step("s1", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.inputs.push(InputDecl {
        name: "secret_val".to_owned(),
        schema_type: ValueType::Number,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for relay of secret input, got {result:?}"
        ));
    }
    Ok(())
}

fn secret_origin_composite_slot_is_tainted() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.secret_val".into())),
        save_step("s1", TypedValue::Composite(vec![TypedValue::Slot(0)])),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.inputs.push(InputDecl {
        name: "secret_val".to_owned(),
        schema_type: ValueType::Number,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for composite of secret input, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_secret_origin_propagates_through_all_downstream_paths() {
    secret_origin_propagates_through_all_downstream_paths().unwrap()
}

#[test]
fn run_secret_origin_relay_slot_is_tainted() {
    secret_origin_relay_slot_is_tainted().unwrap()
}

#[test]
fn run_secret_origin_composite_slot_is_tainted() {
    secret_origin_composite_slot_is_tainted().unwrap()
}

fn slot_to_slot_single_relay_propagates_taint() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("origin", TypedValue::Reference("$secrets.db_url".into())),
        save_step("copy", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("db_url".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for single slot relay, got {result:?}"
        ));
    }
    Ok(())
}

fn slot_to_slot_clean_relay_stays_clean() -> Result<(), String> {
    let wf = make_workflow(vec![
        save_step("origin", TypedValue::Literal(ValueType::Number)),
        save_step("copy", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!("expected Ok for clean slot relay, got {result:?}"));
    }
    Ok(())
}

fn slot_to_slot_branching_relays_both_tainted() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "origin",
            TypedValue::Reference("$secrets.master_key".into()),
        ),
        save_step("branch_a", TypedValue::Slot(0)),
        save_step("branch_b", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("master_key".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for branching relay, got {result:?}"
        ));
    }
    Ok(())
}

fn slot_to_slot_two_hop_relay_carries_taint() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.cred".into())),
        save_step("s1", TypedValue::Slot(0)),
        save_step("s2", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("cred".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for two-hop relay, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_slot_to_slot_single_relay_propagates_taint() {
    slot_to_slot_single_relay_propagates_taint().unwrap()
}

#[test]
fn run_slot_to_slot_clean_relay_stays_clean() {
    slot_to_slot_clean_relay_stays_clean().unwrap()
}

#[test]
fn run_slot_to_slot_branching_relays_both_tainted() {
    slot_to_slot_branching_relays_both_tainted().unwrap()
}

#[test]
fn run_slot_to_slot_two_hop_relay_carries_taint() {
    slot_to_slot_two_hop_relay_carries_taint().unwrap()
}

fn conditional_taint_choose_does_not_taint_downstream() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "tainted_flag",
            TypedValue::Reference("$secrets.branch_sel".into()),
        ),
        save_step("clean_data", TypedValue::Literal(ValueType::Number)),
        choose_step("branch", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("branch_sel".to_owned());
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: choose does not propagate taint to downstream finish, got {result:?}"
        ));
    }
    Ok(())
}

fn conditional_taint_finish_after_choose_reads_tainted() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("clean_flag", TypedValue::Literal(ValueType::Boolean)),
        save_step(
            "tainted_data",
            TypedValue::Reference("$secrets.payload".into()),
        ),
        choose_step("branch", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("payload".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak: finishing tainted slot after choose, got {result:?}"
        ));
    }
    Ok(())
}

fn conditional_taint_multiple_chooses_interleaved() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("flag1", TypedValue::Literal(ValueType::Boolean)),
        choose_step("branch1", TypedValue::Slot(0)),
        save_step(
            "flag2",
            TypedValue::Reference("$secrets.secret_flag".into()),
        ),
        choose_step("branch2", TypedValue::Slot(2)),
        save_step("clean_result", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(4)),
    ]);
    wf.secrets.push("secret_flag".to_owned());
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: choose conditions do not taint downstream, got {result:?}"
        ));
    }
    Ok(())
}

fn conditional_taint_clean_boolean_choose_passes_both_validators() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("flag", TypedValue::Reference("$input.is_admin".into())),
        choose_step("route", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Literal(ValueType::Text)),
    ]);
    wf.inputs.push(InputDecl {
        name: "is_admin".to_owned(),
        schema_type: ValueType::Boolean,
        is_secret: false,
    });
    let type_result = validate_types(&wf);
    if type_result != Ok(()) {
        return Err(format!(
            "expected Ok for type check with boolean input, got {type_result:?}"
        ));
    }
    let taint_result = validate_taint(&wf);
    if taint_result != Ok(()) {
        return Err(format!(
            "expected Ok for taint check with clean input, got {taint_result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_conditional_taint_choose_does_not_taint_downstream() {
    conditional_taint_choose_does_not_taint_downstream().unwrap()
}

#[test]
fn run_conditional_taint_finish_after_choose_reads_tainted() {
    conditional_taint_finish_after_choose_reads_tainted().unwrap()
}

#[test]
fn run_conditional_taint_multiple_chooses_interleaved() {
    conditional_taint_multiple_chooses_interleaved().unwrap()
}

#[test]
fn run_conditional_taint_clean_boolean_choose_passes_both_validators() {
    conditional_taint_clean_boolean_choose_passes_both_validators().unwrap()
}

fn accessor_secret_input_field_carries_taint() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "field_read",
            TypedValue::Reference("$input.credential.token".into()),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "credential".to_owned(),
        schema_type: ValueType::Object,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for secret field access, got {result:?}"
        ));
    }
    Ok(())
}

fn accessor_clean_input_nested_field_stays_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "field_read",
            TypedValue::Reference("$input.user.profile.name".into()),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Object,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for clean nested field access, got {result:?}"
        ));
    }
    Ok(())
}

fn accessor_secret_field_via_secrets_namespace() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "val",
            TypedValue::Reference("$secrets.db.connection_string".into()),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("db".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for secret accessor, got {result:?}"
        ));
    }
    Ok(())
}

fn accessor_var_field_is_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "val",
            TypedValue::Reference("$vars.config.threshold".into()),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.vars.push(("config".to_owned(), ValueType::Object));
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!("expected Ok for var accessor, got {result:?}"));
    }
    Ok(())
}

fn accessor_secret_in_composite_propagates_taint() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "mixed",
            TypedValue::Composite(vec![
                TypedValue::Reference("$secrets.key.sub_key".into()),
                TypedValue::Literal(ValueType::Number),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("key".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for secret accessor in composite, got {result:?}"
        ));
    }
    Ok(())
}

fn accessor_composite_of_clean_accessors_stays_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "combined",
            TypedValue::Composite(vec![
                TypedValue::Reference("$input.data.value".into()),
                TypedValue::Reference("$vars.settings.limit".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "data".to_owned(),
        schema_type: ValueType::Object,
        is_secret: false,
    });
    wf.vars.push(("settings".to_owned(), ValueType::Object));
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for composite of clean accessors, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_accessor_secret_input_field_carries_taint() {
    accessor_secret_input_field_carries_taint().unwrap()
}

#[test]
fn run_accessor_clean_input_nested_field_stays_clean() {
    accessor_clean_input_nested_field_stays_clean().unwrap()
}

#[test]
fn run_accessor_secret_field_via_secrets_namespace() {
    accessor_secret_field_via_secrets_namespace().unwrap()
}

#[test]
fn run_accessor_var_field_is_clean() {
    accessor_var_field_is_clean().unwrap()
}

#[test]
fn run_accessor_secret_in_composite_propagates_taint() {
    accessor_secret_in_composite_propagates_taint().unwrap()
}

#[test]
fn run_accessor_composite_of_clean_accessors_stays_clean() {
    accessor_composite_of_clean_accessors_stays_clean().unwrap()
}

fn fully_clean_workflow_passes_both_validators() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("data", TypedValue::Reference("$input.count".into())),
        save_step("threshold", TypedValue::Reference("$vars.limit".into())),
        save_step("flag", TypedValue::Literal(ValueType::Boolean)),
        choose_step("route", TypedValue::Slot(2)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "count".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    wf.vars.push(("limit".to_owned(), ValueType::Number));
    let type_result = validate_types(&wf);
    if type_result != Ok(()) {
        return Err(format!(
            "expected Ok for type validation, got {type_result:?}"
        ));
    }
    let taint_result = validate_taint(&wf);
    if taint_result != Ok(()) {
        return Err(format!(
            "expected Ok for taint validation, got {taint_result:?}"
        ));
    }
    Ok(())
}

fn clean_path_through_relay_chain() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.value".into())),
        save_step("s1", TypedValue::Slot(0)),
        save_step("s2", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.inputs.push(InputDecl {
        name: "value".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!("expected Ok for clean relay chain, got {result:?}"));
    }
    Ok(())
}

fn clean_composite_in_finish_passes() -> Result<(), String> {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Literal(ValueType::Number),
            TypedValue::Literal(ValueType::Text),
            TypedValue::Literal(ValueType::Boolean),
        ]),
    )]);
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for clean composite finish, got {result:?}"
        ));
    }
    Ok(())
}

fn clean_finish_with_secrets_in_other_paths() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step(
            "secret_slot",
            TypedValue::Reference("$secrets.unused".into()),
        ),
        save_step("clean_slot", TypedValue::Literal(ValueType::Text)),
        choose_step("route", TypedValue::Literal(ValueType::Boolean)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("unused".to_owned());
    let taint_result = validate_taint(&wf);
    if taint_result != Ok(()) {
        return Err(format!(
            "expected Ok: secrets in non-finish path should not cause leak, got {taint_result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_fully_clean_workflow_passes_both_validators() {
    fully_clean_workflow_passes_both_validators().unwrap()
}

#[test]
fn run_clean_path_through_relay_chain() {
    clean_path_through_relay_chain().unwrap()
}

#[test]
fn run_clean_composite_in_finish_passes() {
    clean_composite_in_finish_passes().unwrap()
}

#[test]
fn run_clean_finish_with_secrets_in_other_paths() {
    clean_finish_with_secrets_in_other_paths().unwrap()
}

fn taint_merge_secret_plus_secret() -> Result<(), String> {
    let result = Taint::Secret.merge(Taint::Secret);
    if result != Taint::Secret {
        return Err(format!("expected Taint::Secret, got {result:?}"));
    }
    Ok(())
}

fn taint_merge_clean_plus_clean() -> Result<(), String> {
    let result = Taint::Clean.merge(Taint::Clean);
    if result != Taint::Clean {
        return Err(format!("expected Taint::Clean, got {result:?}"));
    }
    Ok(())
}

fn taint_merge_secret_plus_clean_directions() -> Result<(), String> {
    let forward = Taint::Secret.merge(Taint::Clean);
    let backward = Taint::Clean.merge(Taint::Secret);
    if forward != Taint::Secret {
        return Err(format!(
            "expected Taint::Secret for Secret.merge(Clean), got {forward:?}"
        ));
    }
    if backward != Taint::Secret {
        return Err(format!(
            "expected Taint::Secret for Clean.merge(Secret), got {backward:?}"
        ));
    }
    Ok(())
}

fn taint_merge_composite_of_two_secret_sources() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.alpha".into())),
        save_step(
            "merged",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Reference("$input.beta".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("alpha".to_owned());
    wf.inputs.push(InputDecl {
        name: "beta".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for merged secrets, got {result:?}"
        ));
    }
    Ok(())
}

fn taint_merge_secret_dominates_over_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("secret_slot", TypedValue::Reference("$secrets.dom".into())),
        save_step(
            "merged",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Reference("$input.clean_val".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("dom".to_owned());
    wf.inputs.push(InputDecl {
        name: "clean_val".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak: secret taint dominates, got {result:?}"
        ));
    }
    Ok(())
}

fn taint_merge_three_distinct_secret_sources() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.a".into())),
        save_step("s1", TypedValue::Reference("$input.b".into())),
        save_step(
            "merged",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Slot(1),
                TypedValue::Reference("$secrets.c".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("a".to_owned());
    wf.secrets.push("c".to_owned());
    wf.inputs.push(InputDecl {
        name: "b".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak for three merged secrets, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_taint_merge_secret_plus_secret() {
    taint_merge_secret_plus_secret().unwrap()
}

#[test]
fn run_taint_merge_clean_plus_clean() {
    taint_merge_clean_plus_clean().unwrap()
}

#[test]
fn run_taint_merge_secret_plus_clean_directions() {
    taint_merge_secret_plus_clean_directions().unwrap()
}

#[test]
fn run_taint_merge_composite_of_two_secret_sources() {
    taint_merge_composite_of_two_secret_sources().unwrap()
}

#[test]
fn run_taint_merge_secret_dominates_over_clean() {
    taint_merge_secret_dominates_over_clean().unwrap()
}

#[test]
fn run_taint_merge_three_distinct_secret_sources() {
    taint_merge_three_distinct_secret_sources().unwrap()
}

fn boundary_empty_workflow_passes() -> Result<(), String> {
    let wf = make_workflow(vec![]);
    let type_result = validate_types(&wf);
    if type_result != Ok(()) {
        return Err(format!(
            "expected Ok for empty workflow types, got {type_result:?}"
        ));
    }
    let taint_result = validate_taint(&wf);
    if taint_result != Ok(()) {
        return Err(format!(
            "expected Ok for empty workflow taint, got {taint_result:?}"
        ));
    }
    Ok(())
}

fn boundary_no_secrets_at_all() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.x".into())),
        save_step("s1", TypedValue::Reference("$vars.y".into())),
        save_step("s2", TypedValue::Literal(ValueType::Text)),
        finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Slot(1),
                TypedValue::Slot(2),
            ]),
        ),
    ]);
    wf.inputs.push(InputDecl {
        name: "x".to_owned(),
        schema_type: ValueType::Number,
        is_secret: false,
    });
    wf.vars.push(("y".to_owned(), ValueType::Number));
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for all-clean workflow, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_all_slots_tainted() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.a".into())),
        save_step("s1", TypedValue::Reference("$input.b".into())),
        save_step("s2", TypedValue::Slot(0)),
        save_step(
            "s3",
            TypedValue::Composite(vec![TypedValue::Slot(0), TypedValue::Slot(1)]),
        ),
        finish_step("done", TypedValue::Slot(3)),
    ]);
    wf.secrets.push("a".to_owned());
    wf.inputs.push(InputDecl {
        name: "b".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak when all slots tainted, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_all_slots_tainted_finish_uses_literal() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.x".into())),
        save_step("s1", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Literal(ValueType::Number)),
    ]);
    wf.secrets.push("x".to_owned());
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: finish uses literal even though all slots tainted, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_forward_slot_reference_is_clean() -> Result<(), String> {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(99)),
    ]);
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: out-of-bounds slot resolves clean, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_self_referential_slot_is_clean() -> Result<(), String> {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: self-referential slot resolves clean, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_cycle_like_pattern_all_clean() -> Result<(), String> {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        save_step("s1", TypedValue::Slot(2)),
        save_step("s2", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for cycle-like pattern, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_bare_finish_literal() -> Result<(), String> {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![finish_step("done", TypedValue::Literal(ValueType::Null))],
        resource_contract: ResourceLimits::default(),
    };
    let type_result = validate_types(&wf);
    if type_result != Ok(()) {
        return Err(format!(
            "expected Ok for bare finish types, got {type_result:?}"
        ));
    }
    let taint_result = validate_taint(&wf);
    if taint_result != Ok(()) {
        return Err(format!(
            "expected Ok for bare finish taint, got {taint_result:?}"
        ));
    }
    Ok(())
}

fn boundary_slot_overwrite_second_write_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.x".into())),
        save_step("s0_again", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("x".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak: slot[0] still has tainted value, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_slot_index_overwritten_to_clean() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.y".into())),
        save_step("s1", TypedValue::Literal(ValueType::Text)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("y".to_owned());
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: finish from clean slot, got {result:?}"
        ));
    }
    Ok(())
}

fn boundary_long_clean_chain_passes() -> Result<(), String> {
    let mut steps = Vec::new();
    steps.push(save_step("s0", TypedValue::Literal(ValueType::Number)));
    for i in 1..10 {
        let prev = i - 1;
        steps.push(save_step(&format!("s{i}"), TypedValue::Slot(prev)));
    }
    steps.push(finish_step("done", TypedValue::Slot(9)));
    let wf = make_workflow(steps);
    let result = validate_taint(&wf);
    if result != Ok(()) {
        return Err(format!("expected Ok for long clean chain, got {result:?}"));
    }
    Ok(())
}

#[test]
fn run_boundary_empty_workflow_passes() {
    boundary_empty_workflow_passes().unwrap()
}

#[test]
fn run_boundary_no_secrets_at_all() {
    boundary_no_secrets_at_all().unwrap()
}

#[test]
fn run_boundary_all_slots_tainted() {
    boundary_all_slots_tainted().unwrap()
}

#[test]
fn run_boundary_all_slots_tainted_finish_uses_literal() {
    boundary_all_slots_tainted_finish_uses_literal().unwrap()
}

#[test]
fn run_boundary_forward_slot_reference_is_clean() {
    boundary_forward_slot_reference_is_clean().unwrap()
}

#[test]
fn run_boundary_self_referential_slot_is_clean() {
    boundary_self_referential_slot_is_clean().unwrap()
}

#[test]
fn run_boundary_cycle_like_pattern_all_clean() {
    boundary_cycle_like_pattern_all_clean().unwrap()
}

#[test]
fn run_boundary_bare_finish_literal() {
    boundary_bare_finish_literal().unwrap()
}

#[test]
fn run_boundary_slot_overwrite_second_write_clean() {
    boundary_slot_overwrite_second_write_clean().unwrap()
}

#[test]
fn run_boundary_slot_index_overwritten_to_clean() {
    boundary_slot_index_overwritten_to_clean().unwrap()
}

#[test]
fn run_boundary_long_clean_chain_passes() {
    boundary_long_clean_chain_passes().unwrap()
}

fn type_check_object_in_choose_rejected() -> Result<(), String> {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Object),
    )]);
    let result = validate_types(&wf);
    let expected = Err(ValidationError::TypeMismatch {
        expected: "boolean".to_owned(),
        found: "object".to_owned(),
    });
    if result != expected {
        return Err(format!("expected TypeMismatch(object), got {result:?}"));
    }
    Ok(())
}

fn type_check_list_in_choose_rejected() -> Result<(), String> {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::List),
    )]);
    let result = validate_types(&wf);
    let expected = Err(ValidationError::TypeMismatch {
        expected: "boolean".to_owned(),
        found: "list".to_owned(),
    });
    if result != expected {
        return Err(format!("expected TypeMismatch(list), got {result:?}"));
    }
    Ok(())
}

fn type_check_number_in_choose_rejected() -> Result<(), String> {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Number),
    )]);
    let result = validate_types(&wf);
    let expected = Err(ValidationError::TypeMismatch {
        expected: "boolean".to_owned(),
        found: "number".to_owned(),
    });
    if result != expected {
        return Err(format!("expected TypeMismatch(number), got {result:?}"));
    }
    Ok(())
}

fn type_check_any_from_unresolved_ref_accepted() -> Result<(), String> {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Reference("$input.missing".into())),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok: unresolved ref resolves as Any, got {result:?}"
        ));
    }
    Ok(())
}

fn type_check_save_composite_passes() -> Result<(), String> {
    let wf = make_workflow(vec![
        save_step(
            "comp",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
        finish_step("done", TypedValue::Literal(ValueType::Null)),
    ]);
    let result = validate_types(&wf);
    if result != Ok(()) {
        return Err(format!(
            "expected Ok for save with composite, got {result:?}"
        ));
    }
    Ok(())
}

fn type_check_multiple_finishes_first_tainted() -> Result<(), String> {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.early".into())),
        finish_step("f1", TypedValue::Slot(0)),
        finish_step("f2", TypedValue::Literal(ValueType::Number)),
    ]);
    wf.secrets.push("early".to_owned());
    let result = validate_taint(&wf);
    if result != Err(ValidationError::SecretResultLeak) {
        return Err(format!(
            "expected SecretResultLeak from first tainted finish, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_type_check_object_in_choose_rejected() {
    type_check_object_in_choose_rejected().unwrap()
}

#[test]
fn run_type_check_list_in_choose_rejected() {
    type_check_list_in_choose_rejected().unwrap()
}

#[test]
fn run_type_check_number_in_choose_rejected() {
    type_check_number_in_choose_rejected().unwrap()
}

#[test]
fn run_type_check_any_from_unresolved_ref_accepted() {
    type_check_any_from_unresolved_ref_accepted().unwrap()
}

#[test]
fn run_type_check_save_composite_passes() {
    type_check_save_composite_passes().unwrap()
}

#[test]
fn run_type_check_multiple_finishes_first_tainted() {
    type_check_multiple_finishes_first_tainted().unwrap()
}

fn resource_limits_zero_constant_pool_rejected() -> Result<(), String> {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_constants: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    if !matches!(result, Err(ValidationError::LimitRequired { .. })) {
        return Err(format!(
            "expected LimitRequired for zero max_constants, got {result:?}"
        ));
    }
    Ok(())
}

fn resource_limits_collect_items_exceeding_hard_rejected() -> Result<(), String> {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_collect_items: 10_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits {
        max_collect_items: 500,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    let expected = Err(ValidationError::LimitExceeded {
        resource: "max_collect_items".to_owned(),
    });
    if result != expected {
        return Err(format!(
            "expected LimitExceeded for max_collect_items, got {result:?}"
        ));
    }
    Ok(())
}

fn resource_limits_retry_attempts_exceeding_hard_rejected() -> Result<(), String> {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_retry_attempts: 100,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits {
        max_retry_attempts: 5,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    let expected = Err(ValidationError::LimitExceeded {
        resource: "max_retry_attempts".to_owned(),
    });
    if result != expected {
        return Err(format!(
            "expected LimitExceeded for max_retry_attempts, got {result:?}"
        ));
    }
    Ok(())
}

fn resource_limits_zero_queue_depth_rejected() -> Result<(), String> {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_queue_depth: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    let expected = Err(ValidationError::LimitRequired {
        resource: "max_queue_depth".to_owned(),
    });
    if result != expected {
        return Err(format!(
            "expected LimitRequired for zero max_queue_depth, got {result:?}"
        ));
    }
    Ok(())
}

#[test]
fn run_resource_limits_zero_constant_pool_rejected() {
    resource_limits_zero_constant_pool_rejected().unwrap()
}

#[test]
fn run_resource_limits_collect_items_exceeding_hard_rejected() {
    resource_limits_collect_items_exceeding_hard_rejected().unwrap()
}

#[test]
fn run_resource_limits_retry_attempts_exceeding_hard_rejected() {
    resource_limits_retry_attempts_exceeding_hard_rejected().unwrap()
}

#[test]
fn run_resource_limits_zero_queue_depth_rejected() {
    resource_limits_zero_queue_depth_rejected().unwrap()
}

// =========================================================================
// BLACKHAT comprehensive security regression tests
// =========================================================================

// ---------------------------------------------------------------------------
// BLACKHAT: Secret taint propagation through composite values
// ---------------------------------------------------------------------------

#[test]
fn blackhat_composite_finish_directly_with_secret_reference() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![TypedValue::Reference("$secrets.password".into())]),
    )]);
    wf.secrets.push("password".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: composite in finish directly referencing a secret must leak"
    );
}

#[test]
fn blackhat_composite_finish_with_secret_slot_and_clean_literal() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.api_key".into())),
        finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
    ]);
    wf.secrets.push("api_key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: composite of secret slot + clean literal in finish must leak"
    );
}

#[test]
fn blackhat_composite_of_composites_carries_secret_taint() {
    let mut wf = make_workflow(vec![
        save_step(
            "outer",
            TypedValue::Composite(vec![TypedValue::Composite(vec![TypedValue::Reference(
                "$secrets.innermost".into(),
            )])]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("innermost".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: nested composites must propagate secret taint outward"
    );
}

#[test]
fn blackhat_composite_with_two_secret_inputs_leaks() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Reference("$input.username".into()),
            TypedValue::Reference("$input.token".into()),
        ]),
    )]);
    wf.inputs.push(InputDecl {
        name: "username".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    wf.inputs.push(InputDecl {
        name: "token".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: composite with one secret input and one clean must still leak"
    );
}

#[test]
fn blackhat_composite_with_only_clean_elements_passes() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Literal(ValueType::Number),
            TypedValue::Literal(ValueType::Text),
            TypedValue::Literal(ValueType::Boolean),
        ]),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite with only clean literals must pass"
    );
}

#[test]
fn blackhat_secret_propagates_through_save_then_composite_then_finish() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.cred".into())),
        save_step(
            "s1",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Number),
            ]),
        ),
        save_step("s2", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("cred".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: secret through save->composite->relay->finish must leak"
    );
}

#[test]
fn blackhat_secret_in_composite_saved_to_slot_then_relayed() {
    let mut wf = make_workflow(vec![
        save_step(
            "s0",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.key".into())]),
        ),
        save_step("s1", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: composite with secret saved to slot, then relayed, must leak"
    );
}

#[test]
fn blackhat_composite_with_secret_var_reference() {
    // vars are always clean, so this should pass
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Reference("$vars.count".into()),
            TypedValue::Literal(ValueType::Number),
        ]),
    )]);
    wf.vars.push(("count".to_owned(), ValueType::Number));
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite with var reference should be clean since vars are never secret"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Resource limits - zero limits should fail
// ---------------------------------------------------------------------------

#[test]
fn blackhat_zero_max_accessors_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_accessors: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_accessors".to_owned(),
        }),
        "blackhat: zero max_accessors must be rejected"
    );
}

#[test]
fn blackhat_zero_max_expressions_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expressions: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_expressions".to_owned(),
        }),
        "blackhat: zero max_expressions must be rejected"
    );
}

#[test]
fn blackhat_zero_max_expr_stack_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expr_stack: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_expr_stack".to_owned(),
        }),
        "blackhat: zero max_expr_stack must be rejected"
    );
}

#[test]
fn blackhat_zero_max_step_budget_per_tick_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_step_budget_per_tick: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_step_budget_per_tick".to_owned(),
        }),
        "blackhat: zero max_step_budget_per_tick must be rejected"
    );
}

#[test]
fn blackhat_zero_max_output_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_output_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_output_bytes".to_owned(),
        }),
        "blackhat: zero max_output_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_blob_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_blob_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_blob_bytes".to_owned(),
        }),
        "blackhat: zero max_blob_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_ipc_payload_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_ipc_payload_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_ipc_payload_bytes".to_owned(),
        }),
        "blackhat: zero max_ipc_payload_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_retry_attempts_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_retry_attempts: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_retry_attempts".to_owned(),
        }),
        "blackhat: zero max_retry_attempts must be rejected"
    );
}

#[test]
fn blackhat_zero_max_journal_batch_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_journal_batch_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_journal_batch_bytes".to_owned(),
        }),
        "blackhat: zero max_journal_batch_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_steps_rejected() {
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
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_steps".to_owned(),
        }),
        "blackhat: zero max_steps must be rejected"
    );
}

#[test]
fn blackhat_zero_max_slots_rejected() {
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
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_slots".to_owned(),
        }),
        "blackhat: zero max_slots must be rejected"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Resource limits - declared exceeding hard limits
// ---------------------------------------------------------------------------

#[test]
fn blackhat_max_accessors_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_accessors: 100_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_accessors".to_owned(),
        }),
        "blackhat: max_accessors exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_expressions_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expressions: 50_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_expressions".to_owned(),
        }),
        "blackhat: max_expressions exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_expr_stack_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expr_stack: 1_024,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_expr_stack".to_owned(),
        }),
        "blackhat: max_expr_stack exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_step_budget_per_tick_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_step_budget_per_tick: 1_000_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_step_budget_per_tick".to_owned(),
        }),
        "blackhat: max_step_budget_per_tick exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_input_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_input_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_input_bytes".to_owned(),
        }),
        "blackhat: max_input_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_output_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_output_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_output_bytes".to_owned(),
        }),
        "blackhat: max_output_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_blob_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_blob_bytes: 100_000_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_blob_bytes".to_owned(),
        }),
        "blackhat: max_blob_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_ipc_payload_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_ipc_payload_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_ipc_payload_bytes".to_owned(),
        }),
        "blackhat: max_ipc_payload_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_journal_batch_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_journal_batch_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_journal_batch_bytes".to_owned(),
        }),
        "blackhat: max_journal_batch_bytes exceeding hard limit must be rejected"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Type checking - Choose with non-boolean conditions
// ---------------------------------------------------------------------------

#[test]
fn blackhat_choose_with_number_slot_fails_type_check() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        }),
        "blackhat: choose with number slot must fail type check"
    );
}

#[test]
fn blackhat_choose_with_text_slot_fails_type_check() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Text)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "text".to_owned(),
        }),
        "blackhat: choose with text slot must fail type check"
    );
}

#[test]
fn blackhat_choose_with_null_literal_fails_type_check() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Null),
    )]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "null".to_owned(),
        }),
        "blackhat: choose with null literal must fail type check"
    );
}

#[test]
fn blackhat_choose_with_any_type_from_unknown_reference_passes() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Reference("$input.missing_field".into()),
    )]);
    assert_eq!(
        validate_types(&wf),
        Ok(()),
        "blackhat: choose with unresolved reference resolves as Any and should pass"
    );
}

#[test]
fn blackhat_choose_with_boolean_from_secret_input_passes_type_check() {
    let mut wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Reference("$input.is_admin".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "is_admin".to_owned(),
        schema_type: ValueType::Boolean,
        is_secret: true,
    });
    assert_eq!(
        validate_types(&wf),
        Ok(()),
        "blackhat: choose with boolean input (even if secret) should pass type check"
    );
}

#[test]
fn blackhat_multiple_chooses_first_bad_stops_early() {
    let wf = make_workflow(vec![
        choose_step("route1", TypedValue::Literal(ValueType::Number)),
        choose_step("route2", TypedValue::Literal(ValueType::Boolean)),
    ]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        }),
        "blackhat: first bad choose must stop validation early"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Reference resolution paths
// ---------------------------------------------------------------------------

#[test]
fn blackhat_input_reference_resolves_correct_type_and_taint() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.name".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "name".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $input.name for clean input must pass taint"
    );
}

#[test]
fn blackhat_input_reference_with_nested_path_resolves() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.user.profile.name".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Object,
        is_secret: false,
    });
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: deeply nested input path must resolve to input fact"
    );
}

#[test]
fn blackhat_var_reference_resolves_clean() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$var.counter".into()),
    )]);
    wf.vars.push(("counter".to_owned(), ValueType::Number));
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $var alias must resolve and vars are always clean"
    );
}

#[test]
fn blackhat_vars_reference_resolves_clean() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$vars.counter".into()),
    )]);
    wf.vars.push(("counter".to_owned(), ValueType::Number));
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $vars reference must resolve and vars are always clean"
    );
}

#[test]
fn blackhat_secrets_reference_resolves_tainted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.db_password".into()),
    )]);
    wf.secrets.push("db_password".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: $secrets.X must resolve as tainted"
    );
}

#[test]
fn blackhat_secrets_nested_path_resolves_tainted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.db.connection".into()),
    )]);
    wf.secrets.push("db".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: $secrets.X.Y must resolve using first segment and be tainted"
    );
}

#[test]
fn blackhat_unknown_root_reference_is_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$output.result".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: unknown root like $output should resolve as clean"
    );
}

#[test]
fn blackhat_reference_without_dollar_is_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("plain_string".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: reference without $ prefix must be clean text"
    );
}

#[test]
fn blackhat_reference_without_dot_suffix_is_clean_any() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: reference with $ but no dot must be clean"
    );
}

#[test]
fn blackhat_secrets_reference_with_dot_only_is_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $secrets. with empty name should resolve clean (no match)"
    );
}

#[test]
fn blackhat_input_and_secrets_same_name_different_taint() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.key".into())),
        save_step("s1", TypedValue::Reference("$secrets.key".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    wf.secrets.push("key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: same name in input (clean) and secrets, finishing input slot is clean"
    );
}

#[test]
fn blackhat_input_and_secrets_same_name_secret_finishes_tainted() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.key".into())),
        save_step("s1", TypedValue::Reference("$secrets.key".into())),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    wf.secrets.push("key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: same name in input and secrets, finishing secrets slot is tainted"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Edge cases - empty workflow, single-step, max-value limits
// ---------------------------------------------------------------------------

#[test]
fn blackhat_empty_workflow_passes_both_validators() {
    let wf = make_workflow(vec![]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_single_save_step_workflow_passes_types() {
    let wf = make_workflow(vec![save_step(
        "only",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_single_choose_step_with_boolean_passes() {
    let wf = make_workflow(vec![choose_step(
        "only",
        TypedValue::Literal(ValueType::Boolean),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_single_finish_step_with_literal_passes() {
    let wf = make_workflow(vec![finish_step(
        "only",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_resource_limits_at_exact_hard_limit_passes() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 1_000,
            max_slots: 65_535,
            max_constants: 8_192,
            max_accessors: 8_192,
            max_expressions: 4_096,
            max_expr_stack: 64,
            max_step_budget_per_tick: 10_000,
            max_input_bytes: 1_048_576,
            max_output_bytes: 1_048_576,
            max_blob_bytes: 16_777_216,
            max_ipc_payload_bytes: 1_048_576,
            max_retry_attempts: 10,
            max_fanout: 256,
            max_collect_items: 1_000,
            max_queue_depth: 1_024,
            max_journal_batch_bytes: 1_048_576,
        },
    };
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "blackhat: resource limits exactly at hard limits must pass"
    );
}

#[test]
fn blackhat_resource_limits_one_over_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 1_001,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        }),
        "blackhat: max_steps one over hard limit must be rejected"
    );
}

#[test]
fn blackhat_resource_limits_actual_equals_declared_passes() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            finish_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("s1", TypedValue::Literal(ValueType::Number)),
            finish_step("s2", TypedValue::Literal(ValueType::Number)),
        ],
        resource_contract: ResourceLimits {
            max_steps: 3,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "blackhat: actual step count exactly equal to declared max_steps must pass"
    );
}

#[test]
fn blackhat_resource_limits_actual_exceeds_declared_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            finish_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("s1", TypedValue::Literal(ValueType::Number)),
            finish_step("s2", TypedValue::Literal(ValueType::Number)),
            finish_step("s3", TypedValue::Literal(ValueType::Number)),
        ],
        resource_contract: ResourceLimits {
            max_steps: 3,
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
        }),
        "blackhat: 4 actual steps with declared max_steps=3 must be rejected"
    );
}

#[test]
fn blackhat_workflow_with_only_saves_and_no_finish_passes() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        save_step("s1", TypedValue::Literal(ValueType::Text)),
    ]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_save_secret_then_clean_save_then_clean_finish_passes() {
    let mut wf = make_workflow(vec![
        save_step(
            "secret_slot",
            TypedValue::Reference("$secrets.ignored".into()),
        ),
        save_step("clean_slot", TypedValue::Literal(ValueType::Text)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("ignored".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: finishing a clean slot while another slot has secret data should pass"
    );
}

#[test]
fn blackhat_multiple_finishes_only_first_leak_detected() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.early".into())),
        finish_step("leak", TypedValue::Slot(0)),
        finish_step("clean", TypedValue::Literal(ValueType::Number)),
    ]);
    wf.secrets.push("early".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: first tainted finish must be detected; second clean finish not reached"
    );
}

#[test]
fn blackhat_save_overwrites_slot_taint_from_secret_to_clean() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.x".into())),
        save_step("s0_overwrite", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("x".to_owned());
    // slot 0 gets tainted by s0, then overwritten to clean by s0_overwrite
    // BUT: the overwrite uses write_slot which writes to slots[index],
    // where index=1 for the second save_step. So slot 0 stays tainted.
    assert_eq!(
        validate_taint(&wf),
        Err(ValidationError::SecretResultLeak),
        "blackhat: slot 0 remains tainted; second save writes to slot 1, not slot 0"
    );
}

#[test]
fn blackhat_taint_does_not_propagate_through_choose() {
    let mut wf = make_workflow(vec![
        save_step("flag", TypedValue::Reference("$secrets.selector".into())),
        choose_step("route", TypedValue::Slot(0)),
        save_step("result", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("selector".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: taint in choose condition must not propagate to subsequent steps"
    );
}

#[test]
fn blackhat_composite_of_empty_vec_is_clean() {
    let wf = make_workflow(vec![finish_step("done", TypedValue::Composite(vec![]))]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: empty composite in finish should be clean"
    );
}

#[test]
fn blackhat_resource_limits_all_zero_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 0,
            max_slots: 0,
            max_constants: 0,
            max_accessors: 0,
            max_expressions: 0,
            max_expr_stack: 0,
            max_step_budget_per_tick: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            max_blob_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert!(
        matches!(result, Err(ValidationError::LimitRequired { .. })),
        "blackhat: all-zero resource limits must be rejected with LimitRequired, got {result:?}"
    );
    if let Err(ValidationError::LimitRequired { resource }) = result {
        assert_eq!(
            resource, "max_steps",
            "blackhat: first zero limit checked should be max_steps"
        );
    }
}

#[test]
fn blackhat_resource_limits_all_exceeding_hard_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 999_999,
            max_slots: 999_999,
            max_constants: 999_999,
            max_accessors: 999_999,
            max_expressions: 999_999,
            max_expr_stack: 999_999,
            max_step_budget_per_tick: 999_999,
            max_input_bytes: 999_999,
            max_output_bytes: 999_999,
            max_blob_bytes: 999_999,
            max_ipc_payload_bytes: 999_999,
            max_retry_attempts: 999_999,
            max_fanout: 999_999,
            max_collect_items: 999_999,
            max_queue_depth: 999_999,
            max_journal_batch_bytes: 999_999,
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert!(
        matches!(result, Err(ValidationError::LimitExceeded { .. })),
        "blackhat: all limits exceeding hard must be rejected with LimitExceeded, got {result:?}"
    );
    if let Err(ValidationError::LimitExceeded { resource }) = result {
        assert_eq!(
            resource, "max_steps",
            "blackhat: first limit exceeding hard should be max_steps"
        );
    }
}

// ---------------------------------------------------------------------------
// Edge-case tests: taint merge, value fact constructors, empty workflow,
// zero-step resource limits
// ---------------------------------------------------------------------------

#[test]
fn edge_taint_merge_clean_clean_returns_clean() {
    assert_eq!(
        Taint::Clean.merge(Taint::Clean),
        Taint::Clean,
        "edge: Clean + Clean must equal Clean"
    );
}

#[test]
fn edge_taint_merge_clean_secret_returns_secret() {
    assert_eq!(
        Taint::Clean.merge(Taint::Secret),
        Taint::Secret,
        "edge: Clean + Secret must equal Secret"
    );
}

#[test]
fn edge_taint_merge_secret_clean_returns_secret() {
    assert_eq!(
        Taint::Secret.merge(Taint::Clean),
        Taint::Secret,
        "edge: Secret + Clean must equal Secret"
    );
}

#[test]
fn edge_taint_merge_secret_secret_returns_secret() {
    assert_eq!(
        Taint::Secret.merge(Taint::Secret),
        Taint::Secret,
        "edge: Secret + Secret must equal Secret"
    );
}

#[test]
fn edge_value_fact_clean_constructor_returns_clean_taint() {
    let fact = ValueFact::clean(ValueType::Boolean);
    assert_eq!(fact.value_type, ValueType::Boolean);
    assert_eq!(fact.taint, Taint::Clean);
}

#[test]
fn edge_value_fact_secret_constructor_returns_secret_taint() {
    let fact = ValueFact::secret(ValueType::Object);
    assert_eq!(fact.value_type, ValueType::Object);
    assert_eq!(fact.taint, Taint::Secret);
}

#[test]
fn edge_empty_workflow_passes_all_validators() {
    let wf = make_workflow(vec![]);
    assert_eq!(
        validate_types(&wf),
        Ok(()),
        "edge: empty workflow must pass type validation"
    );
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "edge: empty workflow must pass taint validation"
    );
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "edge: empty workflow must pass resource limit validation"
    );
}

#[test]
fn edge_resource_limits_zero_max_steps_with_non_empty_steps_fails_limit_required() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![finish_step("s0", TypedValue::Literal(ValueType::Number))],
        resource_contract: ResourceLimits {
            max_steps: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_steps".to_owned(),
        }),
        "edge: max_steps=0 with non-empty steps must fail with LimitRequired"
    );
}

#[test]
fn taint_merge_clean_clean_is_clean() {
    assert_eq!(Taint::Clean.merge(Taint::Clean), Taint::Clean);
}

#[test]
fn taint_merge_clean_secret_is_secret() {
    assert_eq!(Taint::Clean.merge(Taint::Secret), Taint::Secret);
}

#[test]
fn taint_merge_secret_clean_is_secret() {
    assert_eq!(Taint::Secret.merge(Taint::Clean), Taint::Secret);
}

#[test]
fn taint_merge_secret_secret_is_secret() {
    assert_eq!(Taint::Secret.merge(Taint::Secret), Taint::Secret);
}

#[test]
fn value_fact_clean_has_clean_taint() {
    let fact = ValueFact::clean(ValueType::Number);
    assert_eq!(fact.taint, Taint::Clean);
    assert_eq!(fact.value_type, ValueType::Number);
}

#[test]
fn value_fact_secret_has_secret_taint() {
    let fact = ValueFact::secret(ValueType::Text);
    assert_eq!(fact.taint, Taint::Secret);
    assert_eq!(fact.value_type, ValueType::Text);
}

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
fn validate_taint_accepts_secret_in_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_reference_to_untrusted_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
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
fn validate_taint_accepts_secret_input_finish_exact() {
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
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_nested_secret_composite_exact() {
    let mut wf = make_workflow(vec![
        save_step(
            "cap",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
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


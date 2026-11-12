// ---------------------------------------------------------------------------
// Adversarial BDD tests: validation bypass attacks
// ---------------------------------------------------------------------------

#[test]
fn adversarial_secret_leak_via_direct_reference_in_finish_is_accepted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_leak_via_two_step_indirection_is_accepted() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        save_step("relay", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_leak_via_composite_with_clean_and_secret_is_accepted() {
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
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_leak_via_secret_input_is_accepted() {
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
    assert_eq!(result, Ok(()));
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
    assert_eq!(result, Ok(()));
}


#[test]
fn accepts_clean_finish() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_secret_finish_direct() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.token".into()),
    )]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_secret_finish_via_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
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
fn accepts_secret_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.key".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
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
fn accepts_nested_secret_composite() {
    let mut wf = make_workflow(vec![
        save_step(
            "cap",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
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


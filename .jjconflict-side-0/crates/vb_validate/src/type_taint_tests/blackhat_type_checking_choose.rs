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


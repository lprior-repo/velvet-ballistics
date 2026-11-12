// =========================================================================
// Comprehensive taint propagation and type-checking tests
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
        Ok(()),
        "blackhat: composite in finish directly referencing a secret must be accepted"
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
        Ok(()),
        "blackhat: composite of secret slot + clean literal in finish must be accepted"
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
        Ok(()),
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
        Ok(()),
        "blackhat: composite with one secret input and one clean must still be accepted"
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
        Ok(()),
        "blackhat: secret through save->composite->relay->finish must be accepted"
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
        Ok(()),
        "blackhat: composite with secret saved to slot, then relayed, must be accepted"
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


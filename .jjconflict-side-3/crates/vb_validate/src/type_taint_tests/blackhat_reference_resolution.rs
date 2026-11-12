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
        Ok(()),
        "blackhat: $secrets.X must resolve as tainted and be accepted"
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
        Ok(()),
        "blackhat: $secrets.X.Y must resolve using first segment and be accepted"
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
        Ok(()),
        "blackhat: same name in input and secrets, finishing secrets slot is accepted"
    );
}


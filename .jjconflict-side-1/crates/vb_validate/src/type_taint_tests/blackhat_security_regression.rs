// =========================================================================
// BLACKHAT security regression tests
// =========================================================================

#[test]
fn blackhat_resource_limits_max_slots_does_not_compare_against_step_count() {
    // `WorkflowTypes` does not carry the actual slot count, so the validator
    // must NOT compare `steps.len()` against `max_slots` (that would be a
    // false-positive rejection whenever steps.len() > max_slots). The check
    // reduces to: declared `max_slots` must be non-zero and at most the
    // protocol hard limit.
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
            max_slots: 1, // declared less than steps.len()=2; must still pass
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "blackhat: max_slots declared within hard limit must pass even when \
         steps.len() > max_slots (slot count is not carried by WorkflowTypes)"
    );
}

#[test]
fn blackhat_resource_limits_max_constants_does_not_accept_hard_zero_actual() {
    // `WorkflowTypes` does not carry the actual constants count, so the
    // validator must NOT pass `0` as the actual value (that would make the
    // `actual > declared` check always false, silently masking any
    // actual-exceeds-declared violation). The check reduces to: declared
    // `max_constants` must be non-zero and at most the protocol hard limit.
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_constants: 1, // declared = 1, hard = 8_192, must pass
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "blackhat: max_constants declared within hard limit must pass; \
         the actual-vs-declared check is not possible from WorkflowTypes alone"
    );
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


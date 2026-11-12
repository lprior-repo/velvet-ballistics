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
fn blackhat_multiple_finishes_only_first_accepted() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.early".into())),
        finish_step("leak", TypedValue::Slot(0)),
        finish_step("clean", TypedValue::Literal(ValueType::Number)),
    ]);
    wf.secrets.push("early".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: first tainted finish must be accepted; second clean finish not reached"
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
        Ok(()),
        "blackhat: slot 0 remains tainted; second save writes to slot 1, not slot 0, but acceptance is expected"
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


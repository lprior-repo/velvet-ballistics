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


use vb_core::limits::MAX_STEP_BUDGET;
use vb_core::workflow::compiled_query::{
    MAX_QUERY_PAYLOAD_BYTES, QueryParseError, from_bytes_compiled_queries,
};
use vb_core::workflow::compiled_slug::codec::MAX_SLUG_PAYLOAD_BYTES;
use vb_core::workflow::compiled_slug::{SlugParseError, from_bytes_compiled_slugs};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, CoreError, MaxAttempts, ResourceContract,
    RunFrame, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
    validate_node_bounds, validate_resource_contract,
};

fn node(id: u16, next: Option<StepIdx>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next,
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn finish(id: u16) -> CompiledNode {
    node(
        id,
        None,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    )
}

fn parts(nodes: Vec<CompiledNode>) -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from("regression"),
        digest: WorkflowDigest::from_bytes([0x42; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[test]
fn cw010_rejects_zero_and_oversized_step_budget_contracts() {
    let mut zero_budget = parts(vec![finish(0)]);
    zero_budget.resource_contract.max_step_budget_per_tick = 0;
    assert_eq!(
        validate_resource_contract(&zero_budget),
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_step_budget_per_tick",
        })
    );

    let mut oversized = parts(vec![finish(0)]);
    oversized.resource_contract.max_step_budget_per_tick = MAX_STEP_BUDGET.saturating_add(1);
    assert_eq!(
        validate_resource_contract(&oversized),
        Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_step_budget_per_tick",
        })
    );
}

#[test]
fn cw011_slug_and_query_decoders_reject_oversized_payloads_before_decode() {
    let oversized_slug = vec![0u8; MAX_SLUG_PAYLOAD_BYTES.saturating_add(1)];
    assert_eq!(
        from_bytes_compiled_slugs(&oversized_slug, u64::MAX),
        Err(SlugParseError::PayloadTooLarge {
            size: oversized_slug.len(),
            max: MAX_SLUG_PAYLOAD_BYTES,
        })
    );

    let oversized_query = vec![0u8; MAX_QUERY_PAYLOAD_BYTES.saturating_add(1)];
    assert_eq!(
        from_bytes_compiled_queries(&oversized_query, u64::MAX),
        Err(QueryParseError::PayloadTooLarge {
            size: oversized_query.len(),
            max: MAX_QUERY_PAYLOAD_BYTES,
        })
    );
}

#[test]
fn cf004_new_and_reinitialize_default_parallel_limit_to_zero() -> Result<(), CoreError> {
    let mut frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 1)?;
    assert_eq!(frame.max_parallel_in_flight(), 0);
    assert_eq!(
        frame.add_parallel_in_flight(1),
        Err(CoreError::BudgetExceeded {
            budget: "parallel_in_flight",
            limit: 0,
        })
    );

    frame.set_max_parallel_in_flight(2);
    frame.add_parallel_in_flight(2)?;
    frame.reinitialize(RunId::new(2), StepIdx::new(0), 2, 1)?;
    assert_eq!(frame.max_parallel_in_flight(), 0);
    assert_eq!(frame.parallel_in_flight(), 0);
    assert_eq!(
        frame.add_parallel_in_flight(1),
        Err(CoreError::BudgetExceeded {
            budget: "parallel_in_flight",
            limit: 0,
        })
    );
    Ok(())
}

#[test]
fn cw006_error_handler_nested_error_slot_is_slot_validated() {
    let invalid = parts(vec![
        node(
            0,
            None,
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(1),
                error_slot: Some(SlotIdx::new(1)),
            },
        ),
        finish(1),
    ]);

    assert_eq!(
        CompiledWorkflow::try_from_parts(invalid),
        Err(WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(1),
        })
    );
}

#[test]
fn cw007_backward_jump_is_rejected_by_forward_edge_validator() {
    let invalid = parts(vec![
        node(0, Some(StepIdx::new(1)), CompiledNodeKind::Nop),
        node(
            1,
            None,
            CompiledNodeKind::Jump {
                target: StepIdx::new(0),
            },
        ),
    ]);

    assert_eq!(
        CompiledWorkflow::try_from_parts(invalid),
        Err(WorkflowError::BackwardEdge {
            from: StepIdx::new(1),
            to: StepIdx::new(0),
        })
    );
}

#[test]
fn cw003_validate_node_bounds_checks_on_error_and_kind_targets() {
    let mut on_error_invalid = parts(vec![finish(0)]);
    if let Some(first) = on_error_invalid.nodes.first_mut() {
        first.on_error = Some(StepIdx::new(2));
    }
    assert_eq!(
        validate_node_bounds(&on_error_invalid),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(2),
        })
    );

    let jump_invalid = parts(vec![node(
        0,
        None,
        CompiledNodeKind::Jump {
            target: StepIdx::new(2),
        },
    )]);
    assert_eq!(
        validate_node_bounds(&jump_invalid),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(2),
        })
    );
}

#[test]
fn cf012_max_attempts_zero_returns_invalid_repeat_state() {
    assert_eq!(MaxAttempts::try_new(0), Err(CoreError::InvalidRepeatState));
}

#[test]
fn workflow_input_slots_out_of_bounds_before_duplicate() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::new(5),
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::unconstrained(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(5),
                kind: InputSlotKind::Bool,
                constraint: CompiledInputConstraint::unconstrained(),
            },
        ]),
        name: Box::<str>::from("input_slot_prec"),
        digest: WorkflowDigest::from_bytes([10; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 3, 1, 1, 1),
        step_names: Box::default(),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::InputSlotOutOfBounds { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

fn choose_slot_parts(branches: Box<[SlotBranch]>, otherwise: Option<StepIdx>) -> WorkflowParts {
    branch_parts(
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise: otherwise.into(),
        },
        Box::new([]),
        1,
    )
}

fn choose_expr_parts(
    branches: Box<[ExprBranch]>,
    otherwise: Option<StepIdx>,
    expressions: Box<[ExprProgram]>,
) -> WorkflowParts {
    branch_parts(
        CompiledNodeKind::Choose {
            branches,
            otherwise: otherwise.into(),
        },
        expressions,
        0,
    )
}

fn branch_parts(
    branch_kind: CompiledNodeKind,
    expressions: Box<[ExprProgram]>,
    slot_count: u16,
) -> WorkflowParts {
    let validated_slot_count = slot_count.max(1);
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: external_input(InputSlotKind::Bool),
        name: Box::<str>::from("branch_case"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: vec![
            crate::workflow::node(StepIdx::new(0), branch_kind, None, None),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions,
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
        slot_count: validated_slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(3, validated_slot_count, 1, 1, 1),
        step_names: Box::new([]),
    }
}

const fn resource_contract(
    max_steps: u16,
    max_slots: u16,
    max_constants: u16,
    max_expressions: u16,
    max_expr_stack: u8,
) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants,
        max_accessors: 0,
        max_expressions,
        max_expr_stack,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_input_bytes: 1,
        max_output_bytes: 1,
        max_blob_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_retry_attempts: 0,
        max_fanout: 0,
        max_collect_items: 0,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        ..ResourceContract::DEFAULT
    }
}

// -- WorkflowError exact variant assertions --

#[test]
fn workflow_error_empty_nodes_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("empty"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyNodes) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_entry_out_of_bounds_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("entry_oob"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(5),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_step_out_of_bounds_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("step_oob"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(99)),
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_slot_out_of_bounds_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(5)),
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_const_out_of_bounds_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(50),
        },
        None,
        None,
    )]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(50) => {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_node_id_mismatch_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(7),
        CompiledNodeKind::Nop,
        None,
        None,
    )]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(0) && actual == StepIdx::new(7) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_expression_wrapped_core_error_exact_variant() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;

    let parts = choose_expr_parts(
        vec![ExprBranch {
            condition: ExprIdx::new(1),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
        vec![expression].into_boxed_slice(),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds { expr }))
            if expr == ExprIdx::new(1) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_empty_branch_table_exact_variant() -> Result<(), String> {
    let parts = choose_slot_parts(Box::new([]), None);

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyBranchTable) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// Adversarial BDD tests -- workflow validation attack vectors
// =========================================================================

// --- Empty nodes attack vector ---

#[test]
fn workflow_empty_nodes_rejected_with_empty_nodes_error() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("empty"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(0, 0, 0, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyNodes) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Entry step beyond node array ---

#[test]
fn workflow_entry_step_at_node_count_rejected_with_entry_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("entry_at_boundary"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(1), // exactly at len
        resource_contract: resource_contract(1, 0, 0, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- SetConst with out-of-bounds constant pool index ---

#[test]
fn workflow_set_const_out_of_bounds_constant_returns_const_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(99),
        },
        Some(SlotIdx::new(0)),
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(99) => {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Copy with out-of-bounds source slot ---

#[test]
fn workflow_copy_out_of_bounds_source_slot_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(5),
        },
        Some(SlotIdx::new(0)),
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- EvalExpr with out-of-bounds expression index ---

#[test]
fn workflow_eval_expr_out_of_bounds_returns_expression_wrapped_error() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(99),
        },
        Some(SlotIdx::new(0)),
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds { expr }))
            if expr == ExprIdx::new(99) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Node with output slot beyond slot_count ---

#[test]
fn workflow_node_output_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        Some(SlotIdx::new(5)),
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Jump target out of bounds ---

#[test]
fn workflow_jump_target_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Jump {
            target: StepIdx::new(50),
        },
        None,
        None,
    )]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(50) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Finish with result slot out of bounds ---

#[test]
fn workflow_finish_result_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(10),
        },
        None,
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(10) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Nop with next step out of bounds ---

#[test]
fn workflow_nop_next_step_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(200)),
    )]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Resource contract max_steps set to 0 with 1 node ---

#[test]
fn workflow_zero_max_steps_with_one_node_returns_resource_contract_exceeded() -> Result<(), String>
{
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("zero_max_steps"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(0, 0, 0, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Steps,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- BuildList with slot out of bounds ---

#[test]
fn workflow_build_list_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::BuildList {
            items: vec![SlotIdx::new(0), SlotIdx::new(5)].into_boxed_slice(),
        },
        Some(SlotIdx::new(0)),
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- TogetherStart with out-of-bounds branch target ---

#[test]
fn workflow_together_start_branch_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(3, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![
        crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(99)].into_boxed_slice(),
                join: StepIdx::new(2),
            },
            None,
            None,
        ),
        crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        crate::workflow::node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
    ]
    .into_boxed_slice();
    parts.resource_contract.max_steps = 3;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- TogetherStart with out-of-bounds join target ---

#[test]
fn workflow_together_start_join_out_of_bounds_returns_step_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(2, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![
        crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1)].into_boxed_slice(),
                join: StepIdx::new(50),
            },
            None,
            None,
        ),
        crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
    ]
    .into_boxed_slice();
    parts.resource_contract.max_steps = 2;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(50) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- TogetherJoin with branch_count=0 is rejected ---

#[test]
fn workflow_together_join_zero_branch_count_returns_resource_exceeded() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::TogetherJoin {
            branch_count: 0.into(),
            accumulator: SlotIdx::new(0),
        },
        None,
        None,
    )]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::BranchCount,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- RepeatStart with max_attempts=0 is rejected ---

#[test]
fn workflow_repeat_start_zero_max_attempts_returns_resource_exceeded() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(3, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![
        crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatStart {
                max_attempts: 0.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        crate::workflow::node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
    ]
    .into_boxed_slice();
    parts.resource_contract.max_steps = 3;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::RetryAttempts,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Node ID mismatch at different positions ---

#[test]
fn workflow_second_node_id_mismatch_returns_node_id_mismatch() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(2, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![
        crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        crate::workflow::node(StepIdx::new(5), CompiledNodeKind::Nop, None, None),
    ]
    .into_boxed_slice();
    parts.resource_contract.max_steps = 2;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(1) && actual == StepIdx::new(5) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- CompiledWorkflow accessor and constant lookup return None for invalid indices ---

#[test]
fn compiled_workflow_constant_returns_none_for_out_of_bounds() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|error| error.to_string())?;

    if workflow.constant(ConstIdx::new(1)).is_some() {
        return Err(String::from("expected None for out-of-bounds constant"));
    }
    Ok(())
}

#[test]
fn compiled_workflow_expression_returns_none_for_out_of_bounds() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|error| error.to_string())?;

    if workflow.expression(ExprIdx::new(0)).is_some() {
        return Err(String::from("expected None for out-of-bounds expression"));
    }
    Ok(())
}

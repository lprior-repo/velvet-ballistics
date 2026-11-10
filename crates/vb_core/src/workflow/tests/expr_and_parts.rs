fn assert_pairwise_distinct<T>(values: &[T])
where
    T: PartialEq + Debug,
{
    assert!(
        values.iter().enumerate().all(|(left_index, left)| values
            .iter()
            .enumerate()
            .all(|(right_index, right)| (left_index == right_index) == (left == right))),
        "variants must be pairwise distinct: {values:?}"
    );
}

#[test]
fn expr_program_rejects_binary_underflow() -> Result<(), String> {
    let ops = vec![load(0), ExprOp::Eq].into_boxed_slice();

    match ExprProgram::try_from_ops(ops) {
        Err(CoreError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expr_program_rejects_capacity_overflow() -> Result<(), String> {
    let ops = vec![load(0), load(1)].into_boxed_slice();

    match check_expr_stack_bound(&ops, 1) {
        Err(CoreError::ExpressionStackOverflow { max: 1 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expr_program_rejects_extra_final_value() -> Result<(), String> {
    let ops = vec![load(0), load(1)].into_boxed_slice();

    match ExprProgram::try_from_ops(ops) {
        Err(CoreError::InvalidCompiledWorkflow { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expr_program_rejects_op_limit() -> Result<(), String> {
    let ops = vec![load(0); 257].into_boxed_slice();

    match ExprProgram::try_from_ops(ops) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "expression ops",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expr_program_rejects_stale_max_stack_metadata() -> Result<(), String> {
    let ops = vec![load(0), load(1), ExprOp::Eq].into_boxed_slice();

    match ExprProgram::try_from_parts(ops, 3) {
        Err(CoreError::InvalidCompiledWorkflow { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_accept_resource_contract_at_exact_usage_bounds() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let contract = resource_contract(1, 0, 1, 1, 1);
    let parts = finish_const_parts_with(contract, vec![expression].into_boxed_slice());

    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;

    if workflow.resource_contract() == contract {
        Ok(())
    } else {
        Err(format!(
            "unexpected resource contract: {:?}",
            workflow.resource_contract()
        ))
    }
}

#[test]
fn workflow_parts_reject_nodes_exceeding_resource_contract() -> Result<(), String> {
    expect_resource_error(resource_contract(0, 0, 1, 0, 0), ContractResource::Steps)
}

#[test]
fn workflow_parts_reject_constants_exceeding_resource_contract() -> Result<(), String> {
    expect_resource_error(
        resource_contract(1, 0, 0, 0, 0),
        ContractResource::Constants,
    )
}

#[test]
fn workflow_parts_reject_slot_count_exceeding_resource_contract() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Slots,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_expressions_exceeding_resource_contract() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let parts = finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 1),
        vec![expression].into_boxed_slice(),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Expressions,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_expression_stack_exceeding_resource_contract() -> Result<(), String> {
    let expression =
        ExprProgram::try_from_ops(vec![load(0), load(0), ExprOp::Eq].into_boxed_slice())
            .map_err(|error| error.to_string())?;
    let parts = finish_const_parts_with(
        resource_contract(1, 0, 1, 1, 1),
        vec![expression].into_boxed_slice(),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ExprStack,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_hard_limit_exceeding_contract() -> Result<(), String> {
    let contract = resource_contract(1, 0, 1, 0, 0);
    let parts = finish_const_parts_with(
        ResourceContract {
            max_expressions: u16::MAX,
            ..contract
        },
        Box::new([]),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Expressions,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_accessors_hard_limit_exceeding_contract() -> Result<(), String> {
    let contract = resource_contract(1, 0, 1, 0, 0);
    let parts = finish_const_parts_with(
        ResourceContract {
            max_accessors: u16::MAX,
            ..contract
        },
        Box::new([]),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Accessors,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_expression_stack_hard_limit_exceeding_contract() -> Result<(), String> {
    let contract = resource_contract(1, 0, 1, 0, 0);
    let parts = finish_const_parts_with(
        ResourceContract {
            max_expr_stack: u8::MAX,
            ..contract
        },
        Box::new([]),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::ExprStack,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_node_id_mismatch() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![crate::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )]
    .into_boxed_slice();

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(0) && actual == StepIdx::new(1) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_accept_choose_slot_branch_table_with_otherwise() -> Result<(), String> {
    let parts = choose_slot_parts(
        vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
    );

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn workflow_parts_reject_choose_slot_condition_out_of_bounds() -> Result<(), String> {
    let parts = choose_slot_parts(
        vec![SlotBranch {
            condition: SlotIdx::new(1),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_choose_slot_branch_target_out_of_bounds() -> Result<(), String> {
    let parts = choose_slot_parts(
        vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(3),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
    );

    expect_step_out_of_bounds(parts, StepIdx::new(3))
}

#[test]
fn workflow_parts_reject_choose_slot_otherwise_out_of_bounds() -> Result<(), String> {
    let parts = choose_slot_parts(
        vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(3)),
    );

    expect_step_out_of_bounds(parts, StepIdx::new(3))
}

#[test]
fn workflow_parts_reject_empty_branch_table_without_otherwise() -> Result<(), String> {
    let parts = choose_slot_parts(Box::new([]), None);

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyBranchTable) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_accept_choose_expr_branch_table_with_otherwise() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let parts = choose_expr_parts(
        vec![ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
        vec![expression].into_boxed_slice(),
    );

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn workflow_parts_reject_choose_expr_condition_out_of_bounds() -> Result<(), String> {
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
fn workflow_parts_reject_choose_expr_branch_target_out_of_bounds() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let parts = choose_expr_parts(
        vec![ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(3),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(1)),
        vec![expression].into_boxed_slice(),
    );

    expect_step_out_of_bounds(parts, StepIdx::new(3))
}

#[test]
fn workflow_parts_reject_choose_expr_otherwise_out_of_bounds() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let parts = choose_expr_parts(
        vec![ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(1),
        }]
        .into_boxed_slice(),
        Some(StepIdx::new(3)),
        vec![expression].into_boxed_slice(),
    );

    expect_step_out_of_bounds(parts, StepIdx::new(3))
}

#[test]
fn workflow_parts_accept_build_list_at_exact_item_limit() -> Result<(), String> {
    let items = vec![SlotIdx::new(0); MAX_LIST_ITEMS_PER_VALUE].into_boxed_slice();
    let parts = construction_parts(CompiledNodeKind::BuildList { items }, 1, 1);

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn workflow_parts_reject_build_list_over_item_limit() -> Result<(), String> {
    let items =
        vec![SlotIdx::new(0); MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)].into_boxed_slice();
    let parts = construction_parts(CompiledNodeKind::BuildList { items }, 1, 1);

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_build_list_item_slot_out_of_bounds() -> Result<(), String> {
    let parts = construction_parts(
        CompiledNodeKind::BuildList {
            items: vec![SlotIdx::new(0), SlotIdx::new(2)].into_boxed_slice(),
        },
        2,
        2,
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(2) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_accept_build_object_at_exact_field_limit() -> Result<(), String> {
    let fields = vec![(crate::ids::SymbolId::new(0), SlotIdx::new(0)); MAX_OBJECT_FIELDS_PER_VALUE]
        .into_boxed_slice();
    let parts = construction_parts_with_symbols(CompiledNodeKind::BuildObject { fields }, 1, 1, 1);

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn workflow_parts_reject_build_object_over_field_limit() -> Result<(), String> {
    let fields = vec![
        (crate::ids::SymbolId::new(0), SlotIdx::new(0));
        MAX_OBJECT_FIELDS_PER_VALUE.saturating_add(1)
    ]
    .into_boxed_slice();
    let parts = construction_parts_with_symbols(CompiledNodeKind::BuildObject { fields }, 1, 1, 1);

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ObjectFields,
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_build_object_field_slot_out_of_bounds() -> Result<(), String> {
    let parts = construction_parts_with_symbols(
        CompiledNodeKind::BuildObject {
            fields: vec![
                (crate::ids::SymbolId::new(1), SlotIdx::new(0)),
                (crate::ids::SymbolId::new(2), SlotIdx::new(3)),
            ]
            .into_boxed_slice(),
        },
        2,
        2,
        3,
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(3) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_preserve_build_object_duplicate_field_order() -> Result<(), String> {
    let key = crate::ids::SymbolId::new(5);
    let fields = vec![(key, SlotIdx::new(0)), (key, SlotIdx::new(1))].into_boxed_slice();
    let parts = construction_parts_with_symbols(CompiledNodeKind::BuildObject { fields }, 2, 2, 6);

    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let copied = workflow.to_parts();
    let node = copied
        .nodes
        .first()
        .ok_or(String::from("missing construction node"))?;

    match &node.kind {
        CompiledNodeKind::BuildObject { fields } => {
            if fields.as_ref() == [(key, SlotIdx::new(0)), (key, SlotIdx::new(1))] {
                Ok(())
            } else {
                Err(format!("unexpected fields: {fields:?}"))
            }
        }
        other => Err(format!("unexpected node kind: {other:?}")),
    }
}

fn load(index: u16) -> ExprOp {
    ExprOp::LoadConst(ConstIdx::new(index))
}

fn construction_parts(kind: CompiledNodeKind, slot_count: u16, max_slots: u16) -> WorkflowParts {
    construction_parts_with_symbols(kind, slot_count, max_slots, 0)
}

fn construction_parts_with_symbols(
    kind: CompiledNodeKind,
    slot_count: u16,
    max_slots: u16,
    symbols_count: u32,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("construction_validation"),
        digest: WorkflowDigest::from_bytes([0x42; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            kind,
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, max_slots, 0, 0, 0),
        step_names: Box::new([]),
    }
}

fn expect_resource_error(
    contract: ResourceContract,
    resource: ContractResource,
) -> Result<(), String> {
    let parts = finish_const_parts_with(contract, Box::new([]));

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded { resource: found }) if found == resource => {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

fn expect_step_out_of_bounds(parts: WorkflowParts, step: StepIdx) -> Result<(), String> {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step: found }) if found == step => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

fn finish_const_parts_with(
    resource_contract: ResourceContract,
    expressions: Box<[ExprProgram]>,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("resource_case"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions,
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract,
        step_names: Box::default(),
    }
}
#[test]
fn workflow_input_slots_accept_valid_increasing_indices() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::unconstrained(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::Bool,
                constraint: CompiledInputConstraint::unconstrained(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::Symbol,
                constraint: CompiledInputConstraint::unconstrained(),
            },
        ]),
        name: Box::<str>::from("input_slot_case"),
        digest: WorkflowDigest::from_bytes([5; 32]),
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
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 4, 1, 1, 1),
        step_names: Box::default(),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("unexpected error: {e:?}")),
    }
}

#[test]
fn workflow_input_slots_accept_empty() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("input_slot_empty"),
        digest: WorkflowDigest::from_bytes([6; 32]),
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
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 1, 1),
        step_names: Box::default(),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("unexpected error: {e:?}")),
    }
}

#[test]
fn workflow_input_slots_reject_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([CompiledInputSlot {
            slot: SlotIdx::new(5),
            kind: InputSlotKind::I64,
            constraint: CompiledInputConstraint::unconstrained(),
        }]),
        name: Box::<str>::from("input_slot_oob"),
        digest: WorkflowDigest::from_bytes([7; 32]),
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

#[test]
fn workflow_input_slots_reject_duplicate() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::unconstrained(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::Bool,
                constraint: CompiledInputConstraint::unconstrained(),
            },
        ]),
        name: Box::<str>::from("input_slot_dup"),
        digest: WorkflowDigest::from_bytes([8; 32]),
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
        Err(WorkflowError::DuplicateInputSlot { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_input_slots_reject_non_canonical_order() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::I64,
                constraint: CompiledInputConstraint::unconstrained(),
            },
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::Bool,
                constraint: CompiledInputConstraint::unconstrained(),
            },
        ]),
        name: Box::<str>::from("input_slot_order"),
        digest: WorkflowDigest::from_bytes([9; 32]),
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
        Err(WorkflowError::NonCanonicalInputSlotOrder { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

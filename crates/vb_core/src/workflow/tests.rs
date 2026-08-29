use crate::budget::BudgetError;
use crate::ids::{
    AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_PATH_DEPTH};
use crate::value::ConstValue;
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, CoreError, ExprBranch,
    ExprOp, ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
    check_expr_stack_bound, validate_budget_result,
};
use std::fmt::Debug;

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
    expect_resource_error(resource_contract(0, 0, 1, 0, 0), "max_steps")
}

#[test]
fn workflow_parts_reject_constants_exceeding_resource_contract() -> Result<(), String> {
    expect_resource_error(resource_contract(1, 0, 0, 0, 0), "max_constants")
}

#[test]
fn workflow_parts_reject_slot_count_exceeding_resource_contract() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_slots",
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
            resource: "max_expressions",
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
            resource: "max_expr_stack",
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
            resource: "max_expressions",
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
            resource: "max_accessors",
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
            resource: "max_expr_stack",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_node_id_mismatch() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }]
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
            resource: "list_items",
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
            resource: "object_fields",
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
        name: Box::<str>::from("construction_validation"),
        digest: WorkflowDigest::from_bytes([0x42; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, max_slots, 0, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    }
}

fn expect_resource_error(contract: ResourceContract, resource: &'static str) -> Result<(), String> {
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
        name: Box::<str>::from("resource_case"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions,
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract,
        step_names: Box::default(),
        input_slots: Box::default(),    }
}

fn choose_slot_parts(branches: Box<[SlotBranch]>, otherwise: Option<StepIdx>) -> WorkflowParts {
    branch_parts(
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
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
            otherwise,
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
        name: Box::<str>::from("branch_case"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: branch_kind,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions,
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: validated_slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(3, validated_slot_count, 1, 1, 1),
        step_names: Box::new([]),
        input_slots: Box::new([]),    }
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
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyNodes) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_entry_out_of_bounds_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("entry_oob"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(5),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_step_out_of_bounds_exact_variant() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("step_oob"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(99)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_error_slot_out_of_bounds_exact_variant() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(5)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(50),
        },
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(7),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }]
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
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyNodes) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Entry step beyond node array ---

#[test]
fn workflow_entry_step_at_node_count_rejected_with_entry_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("entry_at_boundary"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(1), // exactly at len
        resource_contract: resource_contract(1, 0, 0, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- SetConst with out-of-bounds constant pool index ---

#[test]
fn workflow_set_const_out_of_bounds_constant_returns_const_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(99),
        },
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(5),
        },
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(99),
        },
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(5)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Jump {
            target: StepIdx::new(50),
        },
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(10),
        },
    }]
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(200)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }]
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
        name: Box::<str>::from("zero_max_steps"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(0, 0, 0, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_steps",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- BuildList with slot out of bounds ---

#[test]
fn workflow_build_list_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 1, 1, 0, 0), Box::new([]));
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildList {
            items: vec![SlotIdx::new(0), SlotIdx::new(5)].into_boxed_slice(),
        },
    }]
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
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(99)].into_boxed_slice(),
                join: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
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
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1)].into_boxed_slice(),
                join: StepIdx::new(50),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
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
    parts.nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherJoin {
            branch_count: 0,
            accumulator: SlotIdx::new(0),
        },
    }]
    .into_boxed_slice();
    parts.slot_count = 1;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "branch_count",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- RepeatStart with max_attempts=0 is rejected ---

#[test]
fn workflow_repeat_start_zero_max_attempts_returns_resource_exceeded() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(3, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
    ]
    .into_boxed_slice();
    parts.resource_contract.max_steps = 3;

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_retry_attempts",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Node ID mismatch at different positions ---

#[test]
fn workflow_second_node_id_mismatch_returns_node_id_mismatch() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(2, 0, 1, 0, 0), Box::new([]));
    parts.nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5), // should be 1
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
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

#[test]
fn compiled_workflow_accessor_returns_none_for_out_of_bounds() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|error| error.to_string())?;

    if workflow.accessor(AccessorIdx::new(0)).is_some() {
        return Err(String::from("expected None for out-of-bounds accessor"));
    }
    Ok(())
}

#[test]
fn compiled_workflow_node_returns_none_for_out_of_bounds() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|error| error.to_string())?;

    if workflow.node(StepIdx::new(5)).is_some() {
        return Err(String::from("expected None for out-of-bounds node"));
    }
    Ok(())
}

// --- to_parts roundtrip preserves identity ---

#[test]
fn compiled_workflow_to_parts_roundtrip_preserves_fields() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let original = finish_const_parts_with(
        resource_contract(1, 0, 1, 1, 1),
        vec![expression].into_boxed_slice(),
    );
    let workflow = CompiledWorkflow::try_from_parts(original).map_err(|error| error.to_string())?;

    let recovered = workflow.to_parts();
    if recovered.name.as_ref() != workflow.name() {
        return Err(String::from("name mismatch"));
    }
    if recovered.digest != workflow.digest() {
        return Err(String::from("digest mismatch"));
    }
    if recovered.entry != workflow.entry() {
        return Err(String::from("entry mismatch"));
    }
    if recovered.slot_count != workflow.slot_count() {
        return Err(String::from("slot_count mismatch"));
    }
    Ok(())
}

// --- Phase 46: IR structural validation tests ---

fn phase46_parts_with_nodes(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    let max_steps = u16::try_from(nodes.len()).unwrap_or(u16::MAX);
    WorkflowParts {
        name: Box::<str>::from("phase46"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(max_steps, slot_count, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    }
}

#[test]
fn phase46_rejects_unreachable_node() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ],
        1,
    );
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(2) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_reachable_chain() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ],
        1,
    );
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_backward_next() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ],
        1,
    );
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to }) => {
            if from == StepIdx::new(1) && to == StepIdx::new(0) {
                Ok(())
            } else {
                Err(format!("wrong from/to: {from:?} -> {to:?}"))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_jump_backward() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(0),
                },
            },
        ],
        1,
    );
    let result = CompiledWorkflow::try_from_parts(parts).map(|_| ());
    match result {
        Err(WorkflowError::JumpCycle { step, target })
            if step == StepIdx::new(1) && target == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("expected exact JumpCycle variant, got {other:?}")),
    }
}

#[test]
fn loop_start_body_must_be_forward_edge() -> Result<(), String> {
    // Node 0 is a ForEachStart whose `body` points back at itself (StepIdx(0)).
    // Before the split, `validate_loop_done_only` ignored `body`, so this
    // configuration passed. After the split, `validate_loop_start_edges`
    // rejects the backward `body` edge with WorkflowError::BackwardEdge.
    let parts = phase46_parts_with_nodes(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 1,
                    body: StepIdx::new(0),
                    done: StepIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ],
        2,
    );
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(0) && to == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_foreach_forward() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("phase46_foreach"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(2),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(4, 3, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_improper_nesting() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("phase46_nesting"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(1),
                    item_slot: SlotIdx::new(2),
                    limit: 10,
                    body: StepIdx::new(2),
                    done: StepIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(3),
                    body: StepIdx::new(2),
                    done: StepIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(4),
                    body: StepIdx::new(1),
                    done: StepIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(5),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(6, 5, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ImproperLoopNesting { inner, outer_done }) => {
            if inner == StepIdx::new(1) && outer_done == StepIdx::new(4) {
                Ok(())
            } else {
                Err(format!(
                    "wrong inner/outer: inner={inner:?}, outer_done={outer_done:?}"
                ))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_proper_nesting() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("phase46_proper"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(1),
                    item_slot: SlotIdx::new(2),
                    limit: 10,
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: Some(StepIdx::new(3)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(3),
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(4),
                    body: StepIdx::new(1),
                    done: StepIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(5),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(6, 5, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_accepts_accessor_field() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(42))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("phase46_acc_field"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 43,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_accepts_accessor_index() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Index(7)].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("phase46_acc_index"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// =========================================================================
// Phase 46 adversarial tests -- IR structural validation edge cases
// =========================================================================

#[test]
fn phase46_rejects_cycle_via_backward_next_edge() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("cycle_next"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(0)),
                // Backward edge: node 1 -> node 0 creates a cycle.
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(1) && to == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_duplicate_step_idx_via_node_id_mismatch() -> Result<(), String> {
    // Two nodes with the same StepIdx (both claim to be step 0).
    let parts = WorkflowParts {
        name: Box::<str>::from("dup_step_idx"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            // Second node at index 1 but claims to be step 0 (duplicate id).
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(1) && actual == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_slot_idx_out_of_bounds_in_finish() -> Result<(), String> {
    // Finish node references slot 99 but slot_count is only 1.
    let parts = WorkflowParts {
        name: Box::<str>::from("slot_oob_finish"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(99),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_slot_idx_out_of_bounds_in_build_list() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("slot_oob_list"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: vec![SlotIdx::new(0), SlotIdx::new(50)].into_boxed_slice(),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 2, 0, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(50) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_slot_idx_out_of_bounds_in_output() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("slot_oob_output"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            // Output slot 200 is out of bounds for slot_count=1.
            output: Some(SlotIdx::new(200)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_unreachable_node_from_entry() -> Result<(), String> {
    // Node 0 finishes immediately; node 1 is never reached.
    let parts = WorkflowParts {
        name: Box::<str>::from("unreachable"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// Phase 46: SymbolId, accessor path depth, and untrusted input tests
// =========================================================================

// --- SymbolId range validation in accessor path Field segments ---

#[test]
fn phase46_rejects_accessor_field_symbol_out_of_bounds() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(5))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_sym_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 3, // Only symbols 0, 1, 2 exist; SymbolId(5) is out of bounds
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_accessor_field_symbol_at_boundary() -> Result<(), String> {
    // SymbolId(2) should be valid when symbols_count=3 (symbols 0, 1, 2)
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(2))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_sym_boundary"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 3,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_accessor_field_symbol_zero_when_no_symbols() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_sym_zero_no_syms"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Accessor path depth validation ---

#[test]
fn phase46_rejects_accessor_path_too_deep() -> Result<(), String> {
    let deep_path: Vec<PathSegment> = (0..=MAX_PATH_DEPTH)
        .map(|i| PathSegment::Index(u32::try_from(i).unwrap_or(0)))
        .collect();
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: deep_path.into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_deep"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::AccessorPathTooDeep { depth, max }) => {
            if depth == MAX_PATH_DEPTH.saturating_add(1) && max == MAX_PATH_DEPTH {
                Ok(())
            } else {
                Err(format!("wrong depth/max: depth={depth}, max={max}"))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_accessor_path_at_max_depth() -> Result<(), String> {
    let path: Vec<PathSegment> = (0..MAX_PATH_DEPTH)
        .map(|i| PathSegment::Index(u32::try_from(i).unwrap_or(0)))
        .collect();
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: path.into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_max_depth"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_accepts_accessor_empty_path() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([]),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_empty_path"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- SymbolId range validation in constant pool ---

#[test]
fn phase46_rejects_constant_symbol_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("const_sym_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Symbol(SymbolId::new(99))].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 5,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_constant_symbol_at_boundary() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("const_sym_boundary"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Symbol(SymbolId::new(4))].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 5,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_constant_symbol_when_zero_symbols() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("const_sym_no_syms"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Symbol(SymbolId::new(0))].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- SymbolId range validation in BuildObject fields ---

#[test]
fn phase46_rejects_build_object_symbol_out_of_bounds() -> Result<(), String> {
    let parts = construction_parts_with_symbols(
        CompiledNodeKind::BuildObject {
            fields: vec![
                (SymbolId::new(0), SlotIdx::new(0)),
                (SymbolId::new(10), SlotIdx::new(0)),
            ]
            .into_boxed_slice(),
        },
        1,
        1,
        5, // only symbols 0..4 exist
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(10) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_build_object_symbols_within_range() -> Result<(), String> {
    let parts = construction_parts_with_symbols(
        CompiledNodeKind::BuildObject {
            fields: vec![
                (SymbolId::new(0), SlotIdx::new(0)),
                (SymbolId::new(1), SlotIdx::new(0)),
            ]
            .into_boxed_slice(),
        },
        1,
        1,
        2,
    );

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- Accessor path index u32::MAX rejection (existing check, new test) ---

#[test]
fn phase46_rejects_accessor_index_u32_max() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Index(u32::MAX)].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_u32max"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::Expression(CoreError::InvalidCompiledWorkflow { reason })) => {
            if reason.contains("u32::MAX") {
                Ok(())
            } else {
                Err(format!("unexpected reason: {reason}"))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Mixed accessor path with both Field and Index ---

#[test]
fn phase46_accepts_accessor_mixed_path() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![
            PathSegment::Field(SymbolId::new(0)),
            PathSegment::Index(3),
            PathSegment::Field(SymbolId::new(1)),
        ]
        .into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_mixed"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_mixed_path_with_bad_symbol() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![
            PathSegment::Field(SymbolId::new(0)),
            PathSegment::Index(3),
            PathSegment::Field(SymbolId::new(5)), // out of bounds
        ]
        .into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_mixed_bad"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- symbols_count roundtrip through to_parts ---

#[test]
fn phase46_symbols_count_roundtrip() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("sym_roundtrip"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 42,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    if workflow.symbols_count() != 42 {
        return Err(format!(
            "expected symbols_count 42, got {}",
            workflow.symbols_count()
        ));
    }
    let recovered = workflow.to_parts();
    if recovered.symbols_count != 42 {
        return Err(format!(
            "expected symbols_count 42 in recovered parts, got {}",
            recovered.symbols_count
        ));
    }
    Ok(())
}

// --- Multiple constants with mixed SymbolId validity ---

#[test]
fn phase46_rejects_second_constant_symbol_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        name: Box::<str>::from("const_mixed_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![
            ConstValue::Symbol(SymbolId::new(0)),  // valid
            ConstValue::Symbol(SymbolId::new(50)), // out of bounds
        ]
        .into_boxed_slice(),
        slot_count: 1,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 2, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(50) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Accessor root slot validation (existing check, confirmed by test) ---

#[test]
fn phase46_rejects_accessor_root_slot_out_of_bounds() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(5),
        path: Box::new([]),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        name: Box::<str>::from("acc_root_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// CompiledNodeKind variant construction tests — all 34 variants
// =========================================================================

#[test]
fn compiled_node_kind_nop_constructs() {
    let kind = CompiledNodeKind::Nop;
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind,
    };
    assert!(matches!(node.kind, CompiledNodeKind::Nop));
}

#[test]
fn compiled_node_kind_set_const_constructs() {
    let kind = CompiledNodeKind::SetConst {
        value: ConstIdx::new(42),
    };
    let CompiledNodeKind::SetConst { value } = kind else {
        panic!("expected SetConst");
    };
    assert_eq!(value, ConstIdx::new(42));
}

#[test]
fn compiled_node_kind_copy_constructs() {
    let kind = CompiledNodeKind::Copy {
        source: SlotIdx::new(7),
    };
    let CompiledNodeKind::Copy { source } = kind else {
        panic!("expected Copy");
    };
    assert_eq!(source, SlotIdx::new(7));
}

#[test]
fn compiled_node_kind_eval_expr_constructs() {
    let kind = CompiledNodeKind::EvalExpr {
        expr: ExprIdx::new(3),
    };
    let CompiledNodeKind::EvalExpr { expr } = kind else {
        panic!("expected EvalExpr");
    };
    assert_eq!(expr, ExprIdx::new(3));
}

#[test]
fn compiled_node_kind_build_object_constructs() {
    let kind = CompiledNodeKind::BuildObject {
        fields: vec![(SymbolId::new(1), SlotIdx::new(0))].into_boxed_slice(),
    };
    let CompiledNodeKind::BuildObject { fields } = kind else {
        panic!("expected BuildObject");
    };
    assert_eq!(fields.len(), 1);
}

#[test]
fn compiled_node_kind_build_list_constructs() {
    let kind = CompiledNodeKind::BuildList {
        items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
    };
    let CompiledNodeKind::BuildList { items } = kind else {
        panic!("expected BuildList");
    };
    assert_eq!(items.len(), 2);
}

#[test]
fn compiled_node_kind_do_constructs() {
    let kind = CompiledNodeKind::Do {
        action: ActionId::new(10),
        input: SlotIdx::new(0),
    };
    let CompiledNodeKind::Do { action, input } = kind else {
        panic!("expected Do");
    };
    assert_eq!(action, ActionId::new(10));
    assert_eq!(input, SlotIdx::new(0));
}

#[test]
fn compiled_node_kind_choose_constructs() {
    let kind = CompiledNodeKind::Choose {
        branches: Box::new([]),
        otherwise: Some(StepIdx::new(1)),
    };
    let CompiledNodeKind::Choose { otherwise, .. } = kind else {
        panic!("expected Choose");
    };
    assert_eq!(otherwise, Some(StepIdx::new(1)));
}

#[test]
fn compiled_node_kind_choose_slot_constructs() {
    let kind = CompiledNodeKind::ChooseSlot {
        branches: Box::new([]),
        otherwise: Some(StepIdx::new(1)),
    };
    let CompiledNodeKind::ChooseSlot { otherwise, .. } = kind else {
        panic!("expected ChooseSlot");
    };
    assert_eq!(otherwise, Some(StepIdx::new(1)));
}

#[test]
fn compiled_node_kind_for_each_start_constructs() {
    let kind = CompiledNodeKind::ForEachStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: 100,
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::ForEachStart { limit, .. } = kind else {
        panic!("expected ForEachStart");
    };
    assert_eq!(limit, 100);
}

#[test]
fn compiled_node_kind_for_each_next_constructs() {
    let kind = CompiledNodeKind::ForEachNext {
        iterator_slot: SlotIdx::new(2),
        body: StepIdx::new(1),
        done: StepIdx::new(3),
    };
    let CompiledNodeKind::ForEachNext { iterator_slot, .. } = kind else {
        panic!("expected ForEachNext");
    };
    assert_eq!(iterator_slot, SlotIdx::new(2));
}

#[test]
fn compiled_node_kind_for_each_join_constructs() {
    let kind = CompiledNodeKind::ForEachJoin {
        output: SlotIdx::new(5),
    };
    let CompiledNodeKind::ForEachJoin { output } = kind else {
        panic!("expected ForEachJoin");
    };
    assert_eq!(output, SlotIdx::new(5));
}

#[test]
fn compiled_node_kind_together_start_constructs() {
    let kind = CompiledNodeKind::TogetherStart {
        branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
        join: StepIdx::new(3),
    };
    let CompiledNodeKind::TogetherStart { branches, join } = kind else {
        panic!("expected TogetherStart");
    };
    assert_eq!(branches.len(), 2);
    assert_eq!(join, StepIdx::new(3));
}

#[test]
fn compiled_node_kind_together_branch_constructs() {
    let kind = CompiledNodeKind::TogetherBranch {
        branch: 0,
        entry: StepIdx::new(1),
        join: StepIdx::new(3),
        accumulator: SlotIdx::new(0),
    };
    let CompiledNodeKind::TogetherBranch { branch, .. } = kind else {
        panic!("expected TogetherBranch");
    };
    assert_eq!(branch, 0);
}

#[test]
fn compiled_node_kind_together_join_constructs() {
    let kind = CompiledNodeKind::TogetherJoin {
        branch_count: 2,
        accumulator: SlotIdx::new(0),
    };
    let CompiledNodeKind::TogetherJoin { branch_count, .. } = kind else {
        panic!("expected TogetherJoin");
    };
    assert_eq!(branch_count, 2);
}

#[test]
fn compiled_node_kind_collect_start_constructs() {
    let kind = CompiledNodeKind::CollectStart {
        source: SlotIdx::new(0),
        limit: 50,
        page_size: 10,
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::CollectStart {
        limit, page_size, ..
    } = kind
    else {
        panic!("expected CollectStart");
    };
    assert_eq!(limit, 50);
    assert_eq!(page_size, 10);
}

#[test]
fn compiled_node_kind_collect_page_constructs() {
    let kind = CompiledNodeKind::CollectPage {
        collector_slot: SlotIdx::new(3),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::CollectPage { collector_slot, .. } = kind else {
        panic!("expected CollectPage");
    };
    assert_eq!(collector_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_collect_next_constructs() {
    let kind = CompiledNodeKind::CollectNext {
        collector_slot: SlotIdx::new(3),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::CollectNext { collector_slot, .. } = kind else {
        panic!("expected CollectNext");
    };
    assert_eq!(collector_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_collect_finish_constructs() {
    let kind = CompiledNodeKind::CollectFinish {
        collector_slot: SlotIdx::new(3),
    };
    let CompiledNodeKind::CollectFinish { collector_slot } = kind else {
        panic!("expected CollectFinish");
    };
    assert_eq!(collector_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_reduce_start_constructs() {
    let kind = CompiledNodeKind::ReduceStart {
        input: SlotIdx::new(0),
        accumulator: SlotIdx::new(1),
        initial: ConstIdx::new(0),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::ReduceStart { accumulator, .. } = kind else {
        panic!("expected ReduceStart");
    };
    assert_eq!(accumulator, SlotIdx::new(1));
}

#[test]
fn compiled_node_kind_reduce_next_constructs() {
    let kind = CompiledNodeKind::ReduceNext {
        iterator_slot: SlotIdx::new(2),
        accumulator: SlotIdx::new(1),
        body: StepIdx::new(1),
        done: StepIdx::new(3),
    };
    let CompiledNodeKind::ReduceNext { iterator_slot, .. } = kind else {
        panic!("expected ReduceNext");
    };
    assert_eq!(iterator_slot, SlotIdx::new(2));
}

#[test]
fn compiled_node_kind_reduce_finish_constructs() {
    let kind = CompiledNodeKind::ReduceFinish {
        accumulator: SlotIdx::new(1),
    };
    let CompiledNodeKind::ReduceFinish { accumulator } = kind else {
        panic!("expected ReduceFinish");
    };
    assert_eq!(accumulator, SlotIdx::new(1));
}

#[test]
fn compiled_node_kind_repeat_start_constructs() {
    let kind = CompiledNodeKind::RepeatStart {
        max_attempts: 5,
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::RepeatStart { max_attempts, .. } = kind else {
        panic!("expected RepeatStart");
    };
    assert_eq!(max_attempts, 5);
}

#[test]
fn compiled_node_kind_repeat_attempt_constructs() {
    let kind = CompiledNodeKind::RepeatAttempt {
        attempt_slot: SlotIdx::new(3),
        body: StepIdx::new(1),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::RepeatAttempt { attempt_slot, .. } = kind else {
        panic!("expected RepeatAttempt");
    };
    assert_eq!(attempt_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_repeat_check_constructs() {
    let kind = CompiledNodeKind::RepeatCheck {
        attempt_slot: SlotIdx::new(3),
        done: StepIdx::new(2),
    };
    let CompiledNodeKind::RepeatCheck { attempt_slot, .. } = kind else {
        panic!("expected RepeatCheck");
    };
    assert_eq!(attempt_slot, SlotIdx::new(3));
}

#[test]
fn compiled_node_kind_repeat_finish_constructs() {
    let kind = CompiledNodeKind::RepeatFinish {
        result: SlotIdx::new(0),
    };
    let CompiledNodeKind::RepeatFinish { result } = kind else {
        panic!("expected RepeatFinish");
    };
    assert_eq!(result, SlotIdx::new(0));
}

#[test]
fn compiled_node_kind_wait_until_constructs() {
    let kind = CompiledNodeKind::WaitUntil {
        deadline_slot: SlotIdx::new(4),
    };
    let CompiledNodeKind::WaitUntil { deadline_slot } = kind else {
        panic!("expected WaitUntil");
    };
    assert_eq!(deadline_slot, SlotIdx::new(4));
}

#[test]
fn compiled_node_kind_wait_event_constructs() {
    let kind = CompiledNodeKind::WaitEvent {
        event: SlotIdx::new(5),
        timeout_slot: Some(SlotIdx::new(6)),
    };
    let CompiledNodeKind::WaitEvent {
        event,
        timeout_slot,
    } = kind
    else {
        panic!("expected WaitEvent");
    };
    assert_eq!(event, SlotIdx::new(5));
    assert_eq!(timeout_slot, Some(SlotIdx::new(6)));
}

#[test]
fn compiled_node_kind_wait_event_without_timeout_constructs() {
    let kind = CompiledNodeKind::WaitEvent {
        event: SlotIdx::new(5),
        timeout_slot: None,
    };
    let CompiledNodeKind::WaitEvent { timeout_slot, .. } = kind else {
        panic!("expected WaitEvent");
    };
    assert!(timeout_slot.is_none());
}

#[test]
fn compiled_node_kind_ask_constructs() {
    let kind = CompiledNodeKind::Ask {
        prompt: SlotIdx::new(7),
        timeout_slot: Some(SlotIdx::new(8)),
    };
    let CompiledNodeKind::Ask { prompt, .. } = kind else {
        panic!("expected Ask");
    };
    assert_eq!(prompt, SlotIdx::new(7));
}

#[test]
fn compiled_node_kind_ask_resume_constructs() {
    let kind = CompiledNodeKind::AskResume {
        answer: SlotIdx::new(9),
    };
    let CompiledNodeKind::AskResume { answer } = kind else {
        panic!("expected AskResume");
    };
    assert_eq!(answer, SlotIdx::new(9));
}

#[test]
fn compiled_node_kind_retry_check_constructs() {
    let kind = CompiledNodeKind::RetryCheck {
        policy_slot: SlotIdx::new(10),
        body: StepIdx::new(1),
        exhausted: StepIdx::new(2),
    };
    let CompiledNodeKind::RetryCheck { policy_slot, .. } = kind else {
        panic!("expected RetryCheck");
    };
    assert_eq!(policy_slot, SlotIdx::new(10));
}

#[test]
fn compiled_node_kind_error_handler_constructs() {
    let kind = CompiledNodeKind::ErrorHandler {
        body: StepIdx::new(1),
        handler: StepIdx::new(2),
        error_slot: None,
    };
    let CompiledNodeKind::ErrorHandler {
        body,
        handler,
        error_slot,
    } = kind
    else {
        panic!("expected ErrorHandler");
    };
    assert_eq!(error_slot, None);
    assert_eq!(body, StepIdx::new(1));
    assert_eq!(handler, StepIdx::new(2));
}

#[test]
fn compiled_node_kind_jump_constructs() {
    let kind = CompiledNodeKind::Jump {
        target: StepIdx::new(3),
    };
    let CompiledNodeKind::Jump { target } = kind else {
        panic!("expected Jump");
    };
    assert_eq!(target, StepIdx::new(3));
}

#[test]
fn compiled_node_kind_finish_constructs() {
    let kind = CompiledNodeKind::Finish {
        result: SlotIdx::new(0),
    };
    let CompiledNodeKind::Finish { result } = kind else {
        panic!("expected Finish");
    };
    assert_eq!(result, SlotIdx::new(0));
}

// =========================================================================
// ExprOp variant construction tests
// =========================================================================

#[test]
fn expr_op_load_slot_constructs() {
    let op = ExprOp::LoadSlot(SlotIdx::new(42));
    assert_eq!(op, ExprOp::LoadSlot(SlotIdx::new(42)));
}

#[test]
fn expr_op_load_const_constructs() {
    let op = ExprOp::LoadConst(ConstIdx::new(7));
    assert_eq!(op, ExprOp::LoadConst(ConstIdx::new(7)));
}

#[test]
fn expr_op_load_accessor_constructs() {
    let op = ExprOp::LoadAccessor(AccessorIdx::new(3));
    assert_eq!(op, ExprOp::LoadAccessor(AccessorIdx::new(3)));
}

#[test]
fn expr_op_comparison_variants_are_distinct() {
    let ops = [
        ExprOp::Eq,
        ExprOp::NotEq,
        ExprOp::Gt,
        ExprOp::Gte,
        ExprOp::Lt,
        ExprOp::Lte,
    ];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_boolean_variants_are_distinct() {
    assert_ne!(ExprOp::And, ExprOp::Or);
    assert_ne!(ExprOp::And, ExprOp::Not);
    assert_ne!(ExprOp::Or, ExprOp::Not);
}

#[test]
fn expr_op_arithmetic_variants_are_distinct() {
    let ops = [ExprOp::Add, ExprOp::Sub, ExprOp::Mul, ExprOp::Div];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_string_helpers_are_distinct() {
    let ops = [
        ExprOp::Contains,
        ExprOp::StartsWith,
        ExprOp::EndsWith,
        ExprOp::Has,
    ];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_unary_helpers_are_distinct() {
    let ops = [ExprOp::Exists, ExprOp::Length, ExprOp::Empty];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_collection_helpers_are_distinct() {
    let ops = [
        ExprOp::Append,
        ExprOp::AppendIf,
        ExprOp::Merge,
        ExprOp::Sum,
        ExprOp::Count,
        ExprOp::Unique,
    ];
    assert_pairwise_distinct(&ops);
}

// =========================================================================
// ExprProgram valid construction tests
// =========================================================================

#[test]
fn expr_program_single_load_slot_succeeds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_single_load_const_succeeds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_single_load_accessor_succeeds() -> Result<(), String> {
    let ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_eq_reduces_stack() -> Result<(), String> {
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Eq,
    ]
    .into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 2 {
        return Err(format!("expected max_stack 2, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_not_preserves_stack() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_empty_ops_rejected() -> Result<(), String> {
    let ops = Box::new([]) as Box<[ExprOp]>;
    match ExprProgram::try_from_ops(ops) {
        Err(CoreError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn expr_program_try_from_parts_matches_computed_stack() -> Result<(), String> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice();
    let program = ExprProgram::try_from_parts(ops, 1).map_err(|e| e.to_string())?;
    if program.max_stack != 1 {
        return Err(format!("expected max_stack 1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_program_try_from_parts_rejects_empty_ops() -> Result<(), String> {
    let ops = Box::new([]) as Box<[ExprOp]>;
    match ExprProgram::try_from_parts(ops, 0) {
        Err(CoreError::ExpressionStackUnderflow) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// check_expr_stack_bound edge cases
// =========================================================================

#[test]
fn check_expr_stack_bound_single_load_returns_one() -> Result<(), String> {
    let ops = [ExprOp::LoadSlot(SlotIdx::new(0))];
    let result = check_expr_stack_bound(&ops, 64).map_err(|e| e.to_string())?;
    if result != 1 {
        return Err(format!("expected 1, got {result}"));
    }
    Ok(())
}

#[test]
fn check_expr_stack_bound_rejects_zero_capacity() -> Result<(), String> {
    let ops = [ExprOp::LoadSlot(SlotIdx::new(0))];
    match check_expr_stack_bound(&ops, 0) {
        Err(CoreError::ExpressionStackOverflow { max: 0 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// AccessorProgram and PathSegment construction tests
// =========================================================================

#[test]
fn accessor_program_empty_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([]),
    };
    assert_eq!(accessor.root, SlotIdx::new(0));
    assert!(accessor.path.is_empty());
}

#[test]
fn accessor_program_field_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(1),
        path: vec![PathSegment::Field(SymbolId::new(42))].into_boxed_slice(),
    };
    assert_eq!(accessor.root, SlotIdx::new(1));
    assert_eq!(accessor.path.len(), 1);
    assert_eq!(accessor.path[0], PathSegment::Field(SymbolId::new(42)));
}

#[test]
fn accessor_program_index_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Index(7)].into_boxed_slice(),
    };
    assert_eq!(accessor.path[0], PathSegment::Index(7));
}

#[test]
fn accessor_program_mixed_path_constructs() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![
            PathSegment::Field(SymbolId::new(1)),
            PathSegment::Index(0),
            PathSegment::Field(SymbolId::new(2)),
        ]
        .into_boxed_slice(),
    };
    assert_eq!(accessor.path.len(), 3);
}

#[test]
fn path_segment_field_equality() {
    assert_eq!(
        PathSegment::Field(SymbolId::new(5)),
        PathSegment::Field(SymbolId::new(5))
    );
    assert_ne!(
        PathSegment::Field(SymbolId::new(5)),
        PathSegment::Field(SymbolId::new(6))
    );
}

#[test]
fn path_segment_index_equality() {
    assert_eq!(PathSegment::Index(3), PathSegment::Index(3));
    assert_ne!(PathSegment::Index(3), PathSegment::Index(4));
}

#[test]
fn path_segment_field_and_index_are_distinct() {
    assert_ne!(PathSegment::Field(SymbolId::new(0)), PathSegment::Index(0));
}

// =========================================================================
// CompiledNode construction tests
// =========================================================================

#[test]
fn compiled_node_constructs_with_all_fields() {
    let node = CompiledNode {
        id: StepIdx::new(5),
        output: Some(SlotIdx::new(3)),
        next: Some(StepIdx::new(6)),
        on_error: Some(StepIdx::new(10)),
        error_slot: Some(SlotIdx::new(7)),
        kind: CompiledNodeKind::Nop,
    };
    assert_eq!(node.id, StepIdx::new(5));
    assert_eq!(node.output, Some(SlotIdx::new(3)));
    assert_eq!(node.next, Some(StepIdx::new(6)));
    assert_eq!(node.on_error, Some(StepIdx::new(10)));
    assert_eq!(node.error_slot, Some(SlotIdx::new(7)));
}

#[test]
fn compiled_node_optional_fields_can_be_none() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    assert!(node.output.is_none());
    assert!(node.next.is_none());
    assert!(node.on_error.is_none());
    assert!(node.error_slot.is_none());
}

// =========================================================================
// CompiledWorkflow in-bounds accessor tests
// =========================================================================

#[test]
fn compiled_workflow_node_returns_some_for_valid_index() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    let node = workflow.node(StepIdx::new(0));
    if node.is_none() {
        return Err(String::from("expected Some for valid step index"));
    }
    Ok(())
}

#[test]
fn compiled_workflow_constant_returns_some_for_valid_index() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    let constant = workflow.constant(ConstIdx::new(0));
    if constant.is_none() {
        return Err(String::from("expected Some for valid constant index"));
    }
    assert_eq!(constant, Some(&ConstValue::Null));
    Ok(())
}

#[test]
fn compiled_workflow_name_returns_name() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    if workflow.name() != "resource_case" {
        return Err(format!(
            "expected 'resource_case', got '{}'",
            workflow.name()
        ));
    }
    Ok(())
}

#[test]
fn compiled_workflow_entry_returns_entry_step() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    if workflow.entry() != StepIdx::new(0) {
        return Err(format!("expected StepIdx(0), got {:?}", workflow.entry()));
    }
    Ok(())
}

#[test]
fn compiled_workflow_node_count_returns_correct_count() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    if workflow.node_count() != 1 {
        return Err(format!("expected 1 node, got {}", workflow.node_count()));
    }
    Ok(())
}

#[test]
fn compiled_workflow_slot_count_returns_correct_value() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 5, 1, 0, 0), Box::new([]));
    parts.slot_count = 5;
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;

    if workflow.slot_count() != 5 {
        return Err(format!("expected 5, got {}", workflow.slot_count()));
    }
    Ok(())
}

#[test]
fn compiled_workflow_digest_returns_correct_value() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    let digest = workflow.digest();
    assert_eq!(digest.as_bytes(), [3u8; 32]);
    Ok(())
}

#[test]
fn compiled_workflow_expression_returns_some_for_valid_expression() -> Result<(), String> {
    let expression =
        ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice()).map_err(|e| e.to_string())?;
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 1, 1),
        vec![expression].into_boxed_slice(),
    ))
    .map_err(|e| e.to_string())?;

    let expr = workflow.expression(ExprIdx::new(0));
    if expr.is_none() {
        return Err(String::from("expected Some for valid expression index"));
    }
    Ok(())
}

// =========================================================================
// ExprBranch and SlotBranch construction tests
// =========================================================================

#[test]
fn expr_branch_constructs_and_fields_match() {
    let branch = ExprBranch {
        condition: ExprIdx::new(2),
        target: StepIdx::new(5),
    };
    assert_eq!(branch.condition, ExprIdx::new(2));
    assert_eq!(branch.target, StepIdx::new(5));
}

#[test]
fn slot_branch_constructs_and_fields_match() {
    let branch = SlotBranch {
        condition: SlotIdx::new(1),
        target: StepIdx::new(3),
    };
    assert_eq!(branch.condition, SlotIdx::new(1));
    assert_eq!(branch.target, StepIdx::new(3));
}

// =========================================================================
// WorkflowError display and equality tests
// =========================================================================

#[test]
fn workflow_error_empty_nodes_display() {
    assert_eq!(
        WorkflowError::EmptyNodes.to_string(),
        "compiled workflow must contain at least one node"
    );
}

#[test]
fn workflow_error_empty_branch_table_display() {
    assert_eq!(
        WorkflowError::EmptyBranchTable.to_string(),
        "branch table must contain a branch or otherwise target"
    );
}

#[test]
fn workflow_error_budget_policy_exceeded_display() {
    let error = WorkflowError::BudgetPolicyExceeded {
        detail: "max_total_steps",
    };
    assert!(error.to_string().contains("max_total_steps"));
}

#[test]
fn workflow_error_step_count_overflow_exact_variant_fields() -> Result<(), String> {
    match (WorkflowError::StepCountOverflow { actual: u64::MAX }) {
        WorkflowError::StepCountOverflow { actual } if actual == u64::MAX => Ok(()),
        other => Err(format!(
            "expected exact StepCountOverflow variant, got {other:?}"
        )),
    }
}

#[test]
fn workflow_error_jump_cycle_exact_variant_fields() -> Result<(), String> {
    match (WorkflowError::JumpCycle {
        step: StepIdx::new(7),
        target: StepIdx::new(3),
    }) {
        WorkflowError::JumpCycle { step, target }
            if step == StepIdx::new(7) && target == StepIdx::new(3) =>
        {
            Ok(())
        }
        other => Err(format!("expected exact JumpCycle variant, got {other:?}")),
    }
}

#[test]
fn validate_budget_maps_every_budget_error_variant_to_exact_detail() -> Result<(), String> {
    assert_budget_detail(total_steps_error(), "max_total_steps")?;
    assert_budget_detail(total_slots_error(), "max_total_slots")?;
    assert_budget_detail(fanout_error(), "max_fanout")?;
    assert_budget_detail(nesting_depth_error(), "max_nesting_depth")?;
    assert_budget_detail(parallel_error(), "max_parallel_in_flight")?;
    assert_budget_detail(action_tickets_error(), "max_action_tickets")?;
    assert_budget_detail(run_time_error(), "max_run_time_seconds")?;
    assert_budget_detail(result_bytes_error(), "max_result_bytes")?;
    assert_budget_detail(steps_executable_error(), "max_steps_executable")?;
    Ok(())
}

#[test]
fn workflow_budget_validation_reports_total_steps_detail() -> Result<(), String> {
    assert_workflow_budget_detail(total_steps_budget_parts(), "max_total_steps")
}

#[test]
fn workflow_budget_validation_reports_fanout_detail() -> Result<(), String> {
    assert_workflow_budget_detail(fanout_budget_parts(), "max_fanout")
}

#[test]
fn workflow_budget_validation_reports_nesting_depth_detail() -> Result<(), String> {
    assert_workflow_budget_detail(nesting_depth_budget_parts(), "max_nesting_depth")
}

#[test]
fn workflow_budget_validation_reports_result_bytes_detail() -> Result<(), String> {
    assert_workflow_budget_detail(result_bytes_budget_parts(), "max_result_bytes")
}

fn total_steps_error() -> BudgetError {
    BudgetError::TotalStepsExceeded {
        actual: 2,
        limit: 1,
    }
}

fn total_slots_error() -> BudgetError {
    BudgetError::TotalSlotsExceeded {
        actual: 2,
        limit: 1,
    }
}

fn fanout_error() -> BudgetError {
    BudgetError::FanoutExceeded {
        actual: 2,
        limit: 1,
    }
}

fn nesting_depth_error() -> BudgetError {
    BudgetError::NestingDepthExceeded {
        actual: 2,
        limit: 1,
    }
}

fn parallel_error() -> BudgetError {
    BudgetError::ParallelExceeded {
        actual: 2,
        limit: 1,
    }
}

fn action_tickets_error() -> BudgetError {
    BudgetError::ActionTicketsExceeded {
        actual: 2,
        limit: 1,
    }
}

fn run_time_error() -> BudgetError {
    BudgetError::RunTimeExceeded {
        actual: 2,
        limit: 1,
    }
}

fn result_bytes_error() -> BudgetError {
    BudgetError::ResultBytesExceeded {
        actual: 2,
        limit: 1,
    }
}

fn steps_executable_error() -> BudgetError {
    BudgetError::StepsExecutableExceeded {
        actual: 2,
        limit: 1,
    }
}

fn assert_budget_detail(error: BudgetError, detail: &'static str) -> Result<(), String> {
    match validate_budget_result(Err(error)) {
        Err(WorkflowError::BudgetPolicyExceeded { detail: actual }) if actual == detail => Ok(()),
        other => Err(format!("unexpected budget validation result: {other:?}")),
    }
}

fn assert_workflow_budget_detail(parts: WorkflowParts, detail: &'static str) -> Result<(), String> {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BudgetPolicyExceeded { detail: actual }) if actual == detail => Ok(()),
        other => Err(format!("unexpected workflow validation result: {other:?}")),
    }
}

fn total_steps_budget_parts() -> WorkflowParts {
    budget_parts(
        vec![
            budget_node(
                0,
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(0),
                    limit: 1_000_001,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            ),
            budget_node(1, CompiledNodeKind::Nop),
            budget_node(2, CompiledNodeKind::Nop),
        ],
        1,
        budget_contract(3, 1),
    )
}

fn fanout_budget_parts() -> WorkflowParts {
    budget_parts(
        vec![
            budget_node(
                0,
                CompiledNodeKind::ChooseSlot {
                    branches: fanout_branches(),
                    otherwise: None,
                },
            ),
            budget_node(1, CompiledNodeKind::Nop),
        ],
        1,
        budget_contract(2, 1),
    )
}

fn nesting_depth_budget_parts() -> WorkflowParts {
    budget_parts(nesting_nodes(), 1, budget_contract(10, 1))
}

fn result_bytes_budget_parts() -> WorkflowParts {
    budget_parts(
        vec![budget_node(
            0,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        )],
        1,
        ResourceContract {
            max_output_bytes: 262_145,
            ..budget_contract(1, 1)
        },
    )
}

fn fanout_branches() -> Box<[SlotBranch]> {
    (0..65)
        .map(|_| SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn nesting_nodes() -> Vec<CompiledNode> {
    (0..9)
        .map(|index| {
            budget_node(
                index,
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(0),
                    limit: 1,
                    body: StepIdx::new(
                        u16::try_from(index.saturating_add(1)).map_or(u16::MAX, |v| v),
                    ),
                    done: StepIdx::new(9),
                },
            )
        })
        .chain(std::iter::once(budget_node(9, CompiledNodeKind::Nop)))
        .collect()
}

fn budget_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        ..ResourceContract::DEFAULT
    }
}

fn budget_parts(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    resource_contract: ResourceContract,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from("budget_validation"),
        digest: WorkflowDigest::from_bytes([0x71; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract,
        step_names: Box::default(),
        input_slots: Box::default(),    }
}

fn budget_node(index: u16, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind,
    }
}

#[cfg(test)]
mod proptests {
    use super::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ResourceContract,
        WorkflowError, WorkflowParts,
    };
    use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn resource_contract_max_steps_is_positive(_unused in 0u8..1u8) {
            let contract = ResourceContract::DEFAULT;
            prop_assert!(contract.max_steps > 0);
        }
    }

    proptest! {
        #[test]
        fn resource_contract_max_slots_is_positive(_unused in 0u8..1u8) {
            let contract = ResourceContract::DEFAULT;
            prop_assert!(contract.max_slots > 0);
        }
    }

    // =========================================================================
    // Property A: Valid minimal workflow always passes validation
    //
    // Generate random (but structurally valid) workflows with 2-10 steps,
    // each forming a SetConst -> ... -> Finish chain.
    // =========================================================================

    /// Builds a valid linear workflow with `step_count` nodes.
    /// Nodes 0..N-2 are SetConst, node N-1 is Finish.
    /// slot_count = 1 (slot 0 is used throughout).
    fn build_valid_chain(step_count: usize) -> WorkflowParts {
        let last = step_count.saturating_sub(1);
        let mut nodes: Vec<CompiledNode> = (0..last)
            .map(|i| {
                let next_step = u16::try_from(i.saturating_add(1)).map_or(u16::MAX, |v| v);
                CompiledNode {
                    id: StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v)),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(next_step)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                }
            })
            .collect();
        nodes.push(CompiledNode {
            id: StepIdx::new(u16::try_from(last).map_or(u16::MAX, |v| v)),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
        WorkflowParts {
            name: Box::<str>::from("proptest_valid_chain"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
            step_names: Box::new([]),
        input_slots: Box::new([]),        }
    }

    fn chain_node(index: usize, total: usize) -> CompiledNode {
        let is_last = index == total.saturating_sub(1);
        let next = if is_last {
            None
        } else {
            Some(StepIdx::new(
                u16::try_from(index.saturating_add(1)).map_or(u16::MAX, |v| v),
            ))
        };
        let kind = if is_last {
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            }
        } else {
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            }
        };
        CompiledNode {
            id: StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v)),
            output: Some(SlotIdx::new(0)),
            next,
            on_error: None,
            error_slot: None,
            kind,
        }
    }

    fn duplicate_step_node(index: usize, total: usize, duplicate_position: usize) -> CompiledNode {
        let claimed_id = if index == duplicate_position {
            StepIdx::new(0)
        } else {
            StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v))
        };
        CompiledNode {
            id: claimed_id,
            ..chain_node(index, total)
        }
    }

    fn unreachable_finish_node(index: usize) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v)),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }
    }

    proptest! {
        #[test]
        fn prop_a_valid_chain_workflow_passes_validation(step_count in 2u16..10u16) {
            let parts = build_valid_chain(usize::from(step_count));
            let result = CompiledWorkflow::try_from_parts(parts);
            prop_assert!(
                result.is_ok(),
                "valid chain with {} steps should pass validation, got {:?}",
                step_count,
                result
            );
        }
    }

    // =========================================================================
    // Property B: SlotIdx out of bounds always rejected
    //
    // Generate workflows where Finish or SetConst references a slot >= slot_count.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_b_finish_slot_out_of_bounds_rejected(
            slot_count in 1u16..10u16,
            bad_slot_delta in 1u16..50u16
        ) {
            let bad_slot = slot_count.saturating_add(bad_slot_delta);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_b_finish_oob"),
                digest: WorkflowDigest::from_bytes([0xBB; 32]),
                nodes: vec![CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(bad_slot),
                    },
                }]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(1, slot_count, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::SlotOutOfBounds { slot }) => {
                    prop_assert_eq!(slot, SlotIdx::new(bad_slot));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected SlotOutOfBounds, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_b_setconst_output_slot_out_of_bounds_rejected(
            slot_count in 1u16..10u16,
            bad_slot_delta in 1u16..50u16
        ) {
            let bad_slot = slot_count.saturating_add(bad_slot_delta);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_b_output_oob"),
                digest: WorkflowDigest::from_bytes([0xBC; 32]),
                nodes: vec![
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: Some(SlotIdx::new(bad_slot)),
                        next: Some(StepIdx::new(1)),
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::SetConst {
                            value: ConstIdx::new(0),
                        },
                    },
                    CompiledNode {
                        id: StepIdx::new(1),
                        output: None,
                        next: None,
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Finish {
                            result: SlotIdx::new(0),
                        },
                    },
                ]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(2, slot_count, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::SlotOutOfBounds { slot }) => {
                    prop_assert_eq!(slot, SlotIdx::new(bad_slot));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected SlotOutOfBounds, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property C: Duplicate StepIdx always rejected
    //
    // Generate workflows with two nodes claiming the same StepIdx.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_c_duplicate_step_idx_rejected(
            step_count in 3u16..10u16,
            duplicate_id_pos in 1u16..9u16
        ) {
            let n = usize::from(step_count);
            let dup_pos = usize::from(duplicate_id_pos.min(step_count.saturating_sub(1)));
            let nodes = (0..n)
                .map(|index| duplicate_step_node(index, n, dup_pos))
                .collect::<Vec<_>>();
            let max_steps = u16::try_from(n).map_or(u16::MAX, |v| v);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_c_dup"),
                digest: WorkflowDigest::from_bytes([0xCC; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: 1,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::NodeIdMismatch { expected, actual }) => {
                    prop_assert_eq!(expected, StepIdx::new(u16::try_from(dup_pos).map_or(u16::MAX, |v| v)));
                    prop_assert_eq!(actual, StepIdx::new(0));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected NodeIdMismatch, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property D: Unreachable nodes always rejected
    //
    // Generate workflows where an extra node exists but no other node points to it.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_d_unreachable_node_rejected(
            chain_len in 2u16..8u16,
            unreachable_count in 1u16..3u16
        ) {
            let chain_n = usize::from(chain_len);
            let extra_n = usize::from(unreachable_count);
            let total = chain_n.saturating_add(extra_n);
            let nodes = (0..chain_n)
                .map(|index| chain_node(index, chain_n))
                .chain((chain_n..total).map(unreachable_finish_node))
                .collect::<Vec<_>>();

            let max_steps = u16::try_from(total).map_or(u16::MAX, |v| v);
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_d_unreachable"),
                digest: WorkflowDigest::from_bytes([0xDD; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: 1,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::UnreachableNode { step }) => {
                    // The first unreachable node should be at index chain_len.
                    prop_assert_eq!(step, StepIdx::new(u16::try_from(chain_n).map_or(u16::MAX, |v| v)));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected UnreachableNode, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property E: Resource contract bounds respected
    //
    // Workflows with step_count > max_steps fail with ResourceContractExceeded.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_e_resource_contract_max_steps_violated(
            actual_steps in 2u16..10u16,
            shortfall in 1u16..5u16
        ) {
            let max_steps_declared = actual_steps.saturating_sub(shortfall);
            // Build a valid chain but with a contract that doesn't cover it.
            let valid_parts = build_valid_chain(usize::from(actual_steps));
            let parts = WorkflowParts {
                resource_contract: resource_contract(max_steps_declared, 1, 1, 0, 0),
        step_names: Box::new([]),
                ..valid_parts
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::ResourceContractExceeded { resource }) => {
                    prop_assert_eq!(resource, "max_steps");
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected ResourceContractExceeded for max_steps, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_e_resource_contract_max_slots_violated(
            actual_slots in 1u16..10u16,
            shortfall in 1u16..5u16
        ) {
            let declared_slots = actual_slots.saturating_sub(shortfall);
            // Single node that uses a slot at the boundary.
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_e_slots"),
                digest: WorkflowDigest::from_bytes([0xEE; 32]),
                nodes: vec![CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                }]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: actual_slots,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(1, declared_slots, 1, 0, 0),
        step_names: Box::new([]),
        input_slots: Box::new([]),            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::ResourceContractExceeded { resource }) => {
                    prop_assert_eq!(resource, "max_slots");
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected ResourceContractExceeded for max_slots, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    fn resource_contract(
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

    // =========================================================================
    // Phase 45 tests — ResourceContract default values
    // =========================================================================

    #[test]
    fn resource_contract_default_has_reasonable_max_steps() {
        assert_eq!(ResourceContract::DEFAULT.max_steps, 10_000);
    }

    #[test]
    fn resource_contract_default_has_reasonable_max_slots() {
        assert_eq!(ResourceContract::DEFAULT.max_slots, 1_024);
    }

    #[test]
    fn resource_contract_default_has_reasonable_max_fanout() {
        assert_eq!(ResourceContract::DEFAULT.max_fanout, 64);
    }

    #[test]
    fn resource_contract_default_has_reasonable_step_budget_per_tick() {
        assert_eq!(ResourceContract::DEFAULT.max_step_budget_per_tick, 10_000);
    }

    #[test]
    fn resource_contract_default_max_steps_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_steps, u16::MAX);
    }

    #[test]
    fn resource_contract_default_max_slots_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_slots, u16::MAX);
    }

    #[test]
    fn resource_contract_default_max_fanout_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_fanout, u16::MAX);
    }

    #[test]
    fn resource_contract_default_max_retry_attempts_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_retry_attempts, u16::MAX);
    }
}

// =========================================================================
// Edge-case tests — CompiledNode variants, CompiledNodeKind, ExprOp,
// ExprProgram, ExprBranch/SlotBranch
// =========================================================================

#[test]
fn compiled_node_nop_construction_equality_and_debug() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let node2 = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    assert_eq!(node, node2, "identical Nop nodes must be equal");
    let debug_str = format!("{node:?}");
    assert!(
        debug_str.contains("Nop"),
        "Debug output for Nop node must contain 'Nop': {debug_str}"
    );
}

#[test]
fn compiled_node_kind_set_const_field_access() -> Result<(), String> {
    let const_idx = ConstIdx::new(42);
    let kind = CompiledNodeKind::SetConst { value: const_idx };
    let CompiledNodeKind::SetConst { value } = kind else {
        return Err(String::from("expected SetConst variant"));
    };
    if value != const_idx {
        return Err(String::from("SetConst value field mismatch"));
    }
    Ok(())
}

#[test]
fn compiled_node_kind_copy_field_access() -> Result<(), String> {
    let slot = SlotIdx::new(7);
    let kind = CompiledNodeKind::Copy { source: slot };
    let CompiledNodeKind::Copy { source } = kind else {
        return Err(String::from("expected Copy variant"));
    };
    if source != slot {
        return Err(String::from("Copy source field mismatch"));
    }
    Ok(())
}

#[test]
fn compiled_node_kind_do_field_access() -> Result<(), String> {
    let action = ActionId::new(3);
    let input = SlotIdx::new(5);
    let kind = CompiledNodeKind::Do { action, input };
    let CompiledNodeKind::Do {
        action: a,
        input: i,
    } = kind
    else {
        return Err(String::from("expected Do variant"));
    };
    if a != action {
        return Err(String::from("Do action field mismatch"));
    }
    if i != input {
        return Err(String::from("Do input field mismatch"));
    }
    Ok(())
}

#[test]
fn compiled_node_kind_finish_equality_and_debug() {
    let kind_a = CompiledNodeKind::Finish {
        result: SlotIdx::new(99),
    };
    let kind_b = CompiledNodeKind::Finish {
        result: SlotIdx::new(99),
    };
    assert_eq!(kind_a, kind_b, "identical Finish variants must be equal");
    let debug_str = format!("{kind_a:?}");
    assert!(
        debug_str.contains("Finish"),
        "Debug output for Finish must contain 'Finish': {debug_str}"
    );
}

#[test]
fn compiled_node_kind_variant_inequality() {
    let nop = CompiledNodeKind::Nop;
    let set_const = CompiledNodeKind::SetConst {
        value: ConstIdx::new(0),
    };
    let finish = CompiledNodeKind::Finish {
        result: SlotIdx::new(0),
    };
    assert_ne!(nop, set_const, "Nop and SetConst must not be equal");
    assert_ne!(nop, finish, "Nop and Finish must not be equal");
    assert_ne!(set_const, finish, "SetConst and Finish must not be equal");
}

#[test]
fn expr_op_comparison_variants_pairwise_distinct() {
    let ops = [
        ExprOp::Eq,
        ExprOp::NotEq,
        ExprOp::Gt,
        ExprOp::Gte,
        ExprOp::Lt,
        ExprOp::Lte,
    ];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_arithmetic_variants_pairwise_distinct() {
    let ops = [ExprOp::Add, ExprOp::Sub, ExprOp::Mul, ExprOp::Div];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_load_variants_with_max_indices() -> Result<(), String> {
    let load_slot = ExprOp::LoadSlot(SlotIdx::new(u16::MAX));
    let load_const = ExprOp::LoadConst(ConstIdx::new(u16::MAX));
    let load_accessor = ExprOp::LoadAccessor(AccessorIdx::new(u16::MAX));

    match load_slot {
        ExprOp::LoadSlot(idx) if idx.get() == u16::MAX => {}
        _ => return Err(String::from("LoadSlot max index mismatch")),
    }
    match load_const {
        ExprOp::LoadConst(idx) if idx.get() == u16::MAX => {}
        _ => return Err(String::from("LoadConst max index mismatch")),
    }
    match load_accessor {
        ExprOp::LoadAccessor(idx) if idx.get() == u16::MAX => {}
        _ => return Err(String::from("LoadAccessor max index mismatch")),
    }

    assert_ne!(load_slot, load_const);
    assert_ne!(load_slot, load_accessor);
    assert_ne!(load_const, load_accessor);
    Ok(())
}

#[test]
fn expr_program_construction_and_field_access() -> Result<(), String> {
    let ops: Box<[ExprOp]> = vec![load(0)].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops.clone()).map_err(|e| e.to_string())?;
    if program.ops.len() != 1 {
        return Err(format!("expected 1 op, got {}", program.ops.len()));
    }
    if program.max_stack != 1 {
        return Err(format!("expected max_stack=1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_branch_zero_offset_construction_and_equality() {
    let branch_a = ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(0),
    };
    let branch_b = ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(0),
    };
    assert_eq!(
        branch_a, branch_b,
        "identical ExprBranch with zero offsets must be equal"
    );
    assert_eq!(branch_a.condition.get(), 0);
    assert_eq!(branch_a.target.get(), 0);

    let debug_str = format!("{branch_a:?}");
    assert!(
        debug_str.contains("ExprBranch"),
        "Debug output must contain 'ExprBranch': {debug_str}"
    );
}

#[test]
fn expr_branch_max_values() {
    let branch = ExprBranch {
        condition: ExprIdx::new(u16::MAX),
        target: StepIdx::new(u16::MAX),
    };
    assert_eq!(branch.condition.get(), u16::MAX);
    assert_eq!(branch.target.get(), u16::MAX);

    let branch_zero = ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(0),
    };
    assert_ne!(branch, branch_zero, "max and zero ExprBranch must differ");
}

#[test]
fn slot_branch_zero_and_max_values() {
    let branch_zero = SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(0),
    };
    let branch_max = SlotBranch {
        condition: SlotIdx::new(u16::MAX),
        target: StepIdx::new(u16::MAX),
    };
    assert_eq!(branch_zero.condition.get(), 0);
    assert_eq!(branch_zero.target.get(), 0);
    assert_eq!(branch_max.condition.get(), u16::MAX);
    assert_eq!(branch_max.target.get(), u16::MAX);
    assert_ne!(branch_zero, branch_max);
}

#[test]
fn expr_op_boolean_and_helper_variants_equality() {
    assert_eq!(ExprOp::And, ExprOp::And);
    assert_eq!(ExprOp::Or, ExprOp::Or);
    assert_eq!(ExprOp::Not, ExprOp::Not);
    assert_ne!(ExprOp::And, ExprOp::Or);
    assert_ne!(ExprOp::Not, ExprOp::And);

    assert_eq!(ExprOp::Contains, ExprOp::Contains);
    assert_eq!(ExprOp::StartsWith, ExprOp::StartsWith);
    assert_eq!(ExprOp::EndsWith, ExprOp::EndsWith);
    assert_ne!(ExprOp::Contains, ExprOp::StartsWith);
    assert_ne!(ExprOp::StartsWith, ExprOp::EndsWith);

    assert_eq!(ExprOp::Has, ExprOp::Has);
    assert_eq!(ExprOp::Exists, ExprOp::Exists);
    assert_eq!(ExprOp::Length, ExprOp::Length);
    assert_eq!(ExprOp::Empty, ExprOp::Empty);
    assert_ne!(ExprOp::Has, ExprOp::Exists);
    assert_ne!(ExprOp::Length, ExprOp::Empty);

    assert_eq!(ExprOp::Append, ExprOp::Append);
    assert_eq!(ExprOp::AppendIf, ExprOp::AppendIf);
    assert_eq!(ExprOp::Merge, ExprOp::Merge);
    assert_eq!(ExprOp::Sum, ExprOp::Sum);
    assert_eq!(ExprOp::Count, ExprOp::Count);
    assert_eq!(ExprOp::Unique, ExprOp::Unique);
    assert_ne!(ExprOp::Append, ExprOp::AppendIf);
    assert_ne!(ExprOp::Sum, ExprOp::Count);
    assert_ne!(ExprOp::Merge, ExprOp::Unique);
}

#[test]
fn compiled_node_kind_jump_and_eval_expr_field_access() -> Result<(), String> {
    let jump = CompiledNodeKind::Jump {
        target: StepIdx::new(10),
    };
    let CompiledNodeKind::Jump { target } = jump else {
        return Err(String::from("expected Jump variant"));
    };
    if target.get() != 10 {
        return Err(String::from("Jump target mismatch"));
    }

    let eval_expr = CompiledNodeKind::EvalExpr {
        expr: ExprIdx::new(5),
    };
    let CompiledNodeKind::EvalExpr { expr } = eval_expr else {
        return Err(String::from("expected EvalExpr variant"));
    };
    if expr.get() != 5 {
        return Err(String::from("EvalExpr expr mismatch"));
    }
    Ok(())
}

// =========================================================================
// vb-hbav B25-B27: blake3 digest coherence tests.
// =========================================================================

fn make_minimal_workflow_parts(name: &str, entry: StepIdx, slot_count: u16) -> WorkflowParts {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    WorkflowParts {
        name: name.into(),
        digest,
        nodes: Box::new([CompiledNode {
            id: entry,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    }
}

#[test]
fn blake3_digest_is_deterministic_for_identical_parts() {
    let parts1 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts2 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let bytes1 = postcard::to_allocvec(&parts1).expect("serialize should succeed");
    let bytes2 = postcard::to_allocvec(&parts2).expect("serialize should succeed");
    let hash1 = blake3::hash(&bytes1);
    let hash2 = blake3::hash(&bytes2);
    assert_eq!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "identical WorkflowParts must produce identical digests"
    );
}

#[test]
fn blake3_digest_differs_when_name_differs() {
    let parts_alpha = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts_beta = make_minimal_workflow_parts("beta", StepIdx::ZERO, 1);
    let bytes_alpha = postcard::to_allocvec(&parts_alpha).expect("serialize should succeed");
    let bytes_beta = postcard::to_allocvec(&parts_beta).expect("serialize should succeed");
    let hash_alpha = blake3::hash(&bytes_alpha);
    let hash_beta = blake3::hash(&bytes_beta);
    assert_ne!(
        hash_alpha.as_bytes(),
        hash_beta.as_bytes(),
        "different name must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_node_count_differs() {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts1 = WorkflowParts {
        name: "test".into(),
        digest,
        nodes: Box::new([node.clone()]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    let node2 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts2 = WorkflowParts {
        name: "test".into(),
        digest,
        nodes: Box::new([node, node2]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    };
    let hash1 = blake3::hash(&postcard::to_allocvec(&parts1).expect("serialize should succeed"));
    let hash2 = blake3::hash(&postcard::to_allocvec(&parts2).expect("serialize should succeed"));
    assert_ne!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "different node_count must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_entry_step_differs() {
    let parts_entry0 = make_minimal_workflow_parts("test", StepIdx::ZERO, 1);
    let parts_entry1 = make_minimal_workflow_parts("test", StepIdx::new(1), 1);
    let hash0 =
        blake3::hash(&postcard::to_allocvec(&parts_entry0).expect("serialize should succeed"));
    let hash1 =
        blake3::hash(&postcard::to_allocvec(&parts_entry1).expect("serialize should succeed"));
    assert_ne!(
        hash0.as_bytes(),
        hash1.as_bytes(),
        "different entry step must produce different digest"
    );
}

#[test]
fn blake3_digest_valid_for_zero_slot_workflow() {
    let parts = make_minimal_workflow_parts("zero_slot", StepIdx::ZERO, 0);
    let bytes = postcard::to_allocvec(&parts).expect("serialize should succeed");
    let hash = blake3::hash(&bytes);
    let hash_bytes = hash.as_bytes();
    assert_eq!(hash_bytes.len(), 32, "blake3 must produce 32-byte hash");
    assert_ne!(hash_bytes, &[0u8; 32], "hash must not be all zeros");
}

//! Workflow tests.

use crate::errors::CoreError;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};
use crate::value::ConstValue;

use super::{
    AccessorProgram, check_expr_stack_bound, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ExprBranch, ExprOp, ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError,
    WorkflowParts,
};

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

    let workflow =
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;

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
    let fields =
        vec![(crate::ids::SymbolId::new(0), SlotIdx::new(0)); MAX_OBJECT_FIELDS_PER_VALUE]
            .into_boxed_slice();
    let parts = construction_parts(CompiledNodeKind::BuildObject { fields }, 1, 1);

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
    let parts = construction_parts(CompiledNodeKind::BuildObject { fields }, 1, 1);

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "object_fields",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn workflow_parts_reject_build_object_field_slot_out_of_bounds() -> Result<(), String> {
    let parts = construction_parts(
        CompiledNodeKind::BuildObject {
            fields: vec![
                (crate::ids::SymbolId::new(1), SlotIdx::new(0)),
                (crate::ids::SymbolId::new(2), SlotIdx::new(3)),
            ]
            .into_boxed_slice(),
        },
        2,
        2,
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
    let parts = construction_parts(CompiledNodeKind::BuildObject { fields }, 2, 2);

    let workflow =
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
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

// Helper functions

fn load(index: u16) -> ExprOp {
    ExprOp::LoadConst(ConstIdx::new(index))
}

fn construction_parts(
    kind: CompiledNodeKind,
    slot_count: u16,
    max_slots: u16,
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, max_slots, 0, 0, 0),
    }
}

fn expect_resource_error(
    contract: ResourceContract,
    resource: &'static str,
) -> Result<(), String> {
    let parts = finish_const_parts_with(contract, Box::new([]));

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded { resource: found })
            if found == resource =>
        {
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
        entry: StepIdx::new(0),
        resource_contract,
    }
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(3, validated_slot_count, 1, 1, 1),
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
        max_input_bytes: 1,
        max_output_bytes: 1,
        max_blob_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_retry_attempts: 0,
        max_fanout: 0,
        max_collect_items: 0,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
    }
}

// WorkflowError exact variant assertions

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
    };

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
        entry: StepIdx::new(5),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
    };

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 0, 1, 0, 0),
    };

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(0, 0, 0, 0, 0),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EmptyNodes) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

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
        entry: StepIdx::new(1),
        resource_contract: resource_contract(1, 0, 0, 0, 0),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

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

#[test]
fn workflow_zero_max_steps_with_one_node_returns_resource_contract_exceeded(
) -> Result<(), String> {
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(0, 0, 0, 0, 0),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_steps",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

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

#[test]
fn workflow_together_start_branch_out_of_bounds_returns_step_out_of_bounds(
) -> Result<(), String> {
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
            id: StepIdx::new(5),
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

#[test]
fn compiled_workflow_to_parts_roundtrip_preserves_fields() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let original = finish_const_parts_with(
        resource_contract(1, 0, 1, 1, 1),
        vec![expression].into_boxed_slice(),
    );
    let workflow =
        CompiledWorkflow::try_from_parts(original).map_err(|error| error.to_string())?;

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

// Phase 46 IR structural validation tests

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(max_steps, slot_count, 1, 0, 0),
    }
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
    assert!(result.is_err(), "backward jump should be rejected as cycle");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("jump cycle"),
        "error should mention jump cycle"
    );
    Ok(())
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(4, 3, 1, 0, 0),
    };
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(6, 5, 1, 0, 0),
    };
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(6, 5, 1, 0, 0),
    };
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
        entry: StepIdx::new(0),
        resource_contract: contract,
    };
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
        entry: StepIdx::new(0),
        resource_contract: contract,
    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// Phase 46 adversarial tests

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
    };

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
    };

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
    };

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 2, 0, 0, 0),
    };

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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_unreachable_node_from_entry() -> Result<(), String> {
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
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ResourceContract default value tests

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

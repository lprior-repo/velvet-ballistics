//! Basic workflow validation tests.

use super::super::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, CoreError, ExprBranch, ExprIdx, ExprOp,
    ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx, WorkflowError, WorkflowParts,
};
use super::tests::{
    choose_expr_parts, choose_slot_parts, construction_parts, construction_parts_with_symbols,
    expect_resource_error, expect_step_out_of_bounds, finish_const_parts_with, load,
    resource_contract,
};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};

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

    match super::super::check_expr_stack_bound(&ops, 1) {
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

// =========================================================================
// CW-009 regression tests: ResourceContract enforcement on node-local fields
// =========================================================================

fn make_for_each_parts(
    limit: u32,
    max_collect_items: u32,
    max_steps: u16,
    max_slots: u16,
) -> WorkflowParts {
    let contract = ResourceContract {
        max_collect_items,
        ..resource_contract(max_steps, max_slots, 0, 0, 0)
    };
    WorkflowParts {
        name: Box::<str>::from("cw009_for_each"),
        digest: crate::ids::WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit,
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: max_slots,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    }
}

#[test]
fn workflow_parts_reject_for_each_limit_over_max_collect_items() -> Result<(), String> {
    // CW-009: ForEachStart.limit must not exceed max_collect_items.
    let parts = make_for_each_parts(200, 100, 3, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("limit over max_collect_items must be rejected")),
    }
}

#[test]
fn workflow_parts_reject_for_each_zero_limit() -> Result<(), String> {
    // CW-009: zero limit means no items are ever processed; treat it as
    // rejected at the contract boundary.
    let parts = make_for_each_parts(0, 100, 3, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("zero ForEachStart limit must be rejected")),
    }
}

#[test]
fn workflow_parts_accept_for_each_limit_at_max_collect_items() -> Result<(), String> {
    // CW-009: limit == max_collect_items is the exact-bounds accepted case.
    let parts = make_for_each_parts(100, 100, 3, 2);
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn make_collect_start_parts(
    limit: u32,
    page_size: u32,
    max_collect_items: u32,
    max_steps: u16,
    max_slots: u16,
) -> WorkflowParts {
    let contract = ResourceContract {
        max_collect_items,
        ..resource_contract(max_steps, max_slots, 0, 0, 0)
    };
    WorkflowParts {
        name: Box::<str>::from("cw009_collect"),
        digest: crate::ids::WorkflowDigest::from_bytes([0xEF; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit,
                    page_size,
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: max_slots,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    }
}

#[test]
fn workflow_parts_reject_collect_start_limit_over_max_collect_items() -> Result<(), String> {
    // CW-009: CollectStart.limit must not exceed max_collect_items.
    let parts = make_collect_start_parts(2_000, 10, 1_000, 3, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("collect limit over max_collect_items must be rejected")),
    }
}

#[test]
fn workflow_parts_reject_collect_start_zero_limit() -> Result<(), String> {
    // CW-009: zero limit is not a valid collect cap.
    let parts = make_collect_start_parts(0, 10, 1_000, 3, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("zero collect limit must be rejected")),
    }
}

#[test]
fn workflow_parts_reject_collect_start_zero_page_size() -> Result<(), String> {
    // CW-009: zero page_size means no items can ever be paged in.
    let parts = make_collect_start_parts(100, 0, 1_000, 3, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("zero collect page_size must be rejected")),
    }
}

#[test]
fn workflow_parts_reject_collect_start_page_size_over_max_collect_items() -> Result<(), String> {
    // CW-009: page_size must also fit under max_collect_items.
    let parts = make_collect_start_parts(2_000, 2_000, 1_000, 3, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_collect_items",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("collect page_size over max_collect_items must be rejected")),
    }
}

fn make_together_join_parts(
    branch_count: u16,
    max_fanout: u16,
    max_steps: u16,
    max_slots: u16,
) -> WorkflowParts {
    let contract = ResourceContract {
        max_fanout,
        ..resource_contract(max_steps, max_slots, 0, 0, 0)
    };
    WorkflowParts {
        name: Box::<str>::from("cw009_together"),
        digest: crate::ids::WorkflowDigest::from_bytes([0xF0; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherJoin {
                    branch_count,
                    accumulator: SlotIdx::new(0),
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
        constants: Box::new([]),
        slot_count: max_slots,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    }
}

#[test]
fn workflow_parts_reject_together_join_branch_count_over_max_fanout() -> Result<(), String> {
    // CW-009: TogetherJoin.branch_count must not exceed max_fanout.
    let parts = make_together_join_parts(65, 64, 2, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_fanout",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("branch_count over max_fanout must be rejected")),
    }
}

#[test]
fn workflow_parts_reject_together_join_zero_branch_count() -> Result<(), String> {
    // CW-009: zero branch_count is not a valid fanout.
    let parts = make_together_join_parts(0, 64, 2, 2);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_fanout",
        }) => Ok(()),
        Err(other) => Err(format!("unexpected error: {other:?}")),
        Ok(_) => Err(String::from("zero branch_count must be rejected")),
    }
}

#[test]
fn workflow_parts_accept_together_join_branch_count_at_max_fanout() -> Result<(), String> {
    // CW-009: branch_count == max_fanout is the exact-bounds accepted case.
    let parts = make_together_join_parts(64, 64, 2, 2);
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

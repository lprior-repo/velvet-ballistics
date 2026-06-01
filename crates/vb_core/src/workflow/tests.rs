//! Shared test helpers for workflow validation tests.
//!
//! This module contains only helper functions used by the test modules.
//! All actual tests have been extracted to separate files.

use crate::budget::BudgetError;
use crate::ids::{
    AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_PATH_DEPTH};
use crate::value::ConstValue;
use crate::errors::CoreError;
use crate::workflow::{
    check_expr_stack_bound, AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ExprBranch, ExprOp, ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError,
    WorkflowParts,
};
use crate::workflow::validation::validate_budget_result;
use std::fmt::Debug;

pub(crate) fn assert_pairwise_distinct<T>(values: &[T])
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

pub(crate) fn load(index: u16) -> ExprOp {
    ExprOp::LoadConst(ConstIdx::new(index))
}

pub(crate) fn construction_parts(
    kind: CompiledNodeKind,
    slot_count: u16,
    max_slots: u16,
) -> WorkflowParts {
    construction_parts_with_symbols(kind, slot_count, max_slots, 0)
}

pub(crate) fn construction_parts_with_symbols(
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
    }
}

pub(crate) fn expect_resource_error(
    contract: ResourceContract,
    resource: &'static str,
) -> Result<(), String> {
    let parts = finish_const_parts_with(contract, Box::new([]));

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ResourceContractExceeded { resource: found }) if found == resource => {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

pub(crate) fn expect_step_out_of_bounds(parts: WorkflowParts, step: StepIdx) -> Result<(), String> {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::StepOutOfBounds { step: found }) if found == step => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

pub(crate) fn finish_const_parts_with(
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
    }
}

pub(crate) fn choose_slot_parts(
    branches: Box<[SlotBranch]>,
    otherwise: Option<StepIdx>,
) -> WorkflowParts {
    branch_parts(
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        },
        Box::new([]),
        1,
    )
}

pub(crate) fn choose_expr_parts(
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
    }
}

pub(crate) const fn resource_contract(
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
// Budget validation helpers
// =========================================================================

pub(crate) fn total_steps_error() -> BudgetError {
    BudgetError::TotalStepsExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn total_slots_error() -> BudgetError {
    BudgetError::TotalSlotsExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn fanout_error() -> BudgetError {
    BudgetError::FanoutExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn nesting_depth_error() -> BudgetError {
    BudgetError::NestingDepthExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn parallel_error() -> BudgetError {
    BudgetError::ParallelExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn action_tickets_error() -> BudgetError {
    BudgetError::ActionTicketsExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn run_time_error() -> BudgetError {
    BudgetError::RunTimeExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn result_bytes_error() -> BudgetError {
    BudgetError::ResultBytesExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn steps_executable_error() -> BudgetError {
    BudgetError::StepsExecutableExceeded {
        actual: 2,
        limit: 1,
    }
}

pub(crate) fn assert_budget_detail(error: BudgetError, detail: &'static str) -> Result<(), String> {
    match validate_budget_result(Err(error)) {
        Err(WorkflowError::BudgetPolicyExceeded { detail: actual }) if actual == detail => Ok(()),
        other => Err(format!("unexpected budget validation result: {other:?}")),
    }
}

pub(crate) fn assert_workflow_budget_detail(
    parts: WorkflowParts,
    detail: &'static str,
) -> Result<(), String> {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BudgetPolicyExceeded { detail: actual }) if actual == detail => Ok(()),
        other => Err(format!("unexpected workflow validation result: {other:?}")),
    }
}

pub(crate) fn total_steps_budget_parts() -> WorkflowParts {
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

pub(crate) fn fanout_budget_parts() -> WorkflowParts {
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

pub(crate) fn nesting_depth_budget_parts() -> WorkflowParts {
    budget_parts(nesting_nodes(), 1, budget_contract(10, 1))
}

pub(crate) fn result_bytes_budget_parts() -> WorkflowParts {
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
    }
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

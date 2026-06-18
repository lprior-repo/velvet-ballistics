#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

//! Shared test helpers for workflow validation tests.
//!
//! This module contains only helper functions used by the test modules.
//! All actual tests have been extracted to separate files.

use crate::budget::BudgetError;
use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::validation::validate_budget_result;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram,
    ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
};

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

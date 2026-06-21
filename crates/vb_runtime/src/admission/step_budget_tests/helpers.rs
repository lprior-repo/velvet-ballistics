//! Shared test helpers for the step-budget gate tests.
//!
//! These helpers were extracted from the original `step_budget_tests.rs`
//! (which exceeded the 300-line source cap) so the test files can stay
//! under the cap. All helpers are public to the parent test module.

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
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
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
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
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
    clippy::useless_asref,
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

use std::num::NonZeroUsize;

use vb_core::ids::{StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

use crate::Runtime;
use crate::shard::ShardConfig;

/// Builds a `CompiledWorkflow` with `max_steps` declared in the resource
/// contract. The compiled node graph is a single `Nop` node so this helper
/// isolates declared-contract admission behavior.
pub(crate) fn workflow_with_max_steps(max_steps: u16) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("max_steps_{max_steps}").into(),
        digest: WorkflowDigest::from_bytes([0xA0; 32]),
        nodes: Box::from([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract {
            max_steps,
            max_slots: 1,
            ..vb_core::workflow::ResourceContract::DEFAULT
        },
        step_names: linear_step_names(1),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

pub(crate) fn linear_workflow_with_declared_steps(
    node_count: u16,
    declared_max_steps: u16,
) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("actual_nodes_{node_count}_declared_{declared_max_steps}").into(),
        digest: WorkflowDigest::from_bytes([0xB0; 32]),
        nodes: linear_nodes(node_count),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract {
            max_steps: declared_max_steps,
            max_slots: node_count,
            ..vb_core::workflow::ResourceContract::DEFAULT
        },
        step_names: linear_step_names(node_count),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

pub(crate) fn linear_nodes(node_count: u16) -> Box<[CompiledNode]> {
    let mut nodes: Vec<CompiledNode> = Vec::with_capacity(usize::from(node_count));
    for index in 0..node_count {
        let kind = if next_linear_step(index, node_count).is_none() {
            CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            }
        } else {
            CompiledNodeKind::Nop
        };
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: next_linear_step(index, node_count),
            on_error: None,
            error_slot: None,
            kind,
        });
    }
    nodes.into_boxed_slice()
}

pub(crate) fn next_linear_step(index: u16, node_count: u16) -> Option<StepIdx> {
    match index.checked_add(1) {
        Some(next) if next < node_count => Some(StepIdx::new(next)),
        _ => None,
    }
}

pub(crate) fn linear_step_names(node_count: u16) -> Box<[Box<str>]> {
    let mut names = Vec::with_capacity(usize::from(node_count));
    let mut index = 0u16;
    while index < node_count {
        names.push(format!("s{index}").into_boxed_str());
        index = index.saturating_add(1);
    }
    names.into_boxed_slice()
}

pub(crate) fn master_step_limit_u16() -> u16 {
    match u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW) {
        Ok(value) => value,
        Err(_) => u16::MAX,
    }
}

pub(crate) fn first_step_count_over_master_limit() -> u16 {
    match vb_core::limits::MAX_STEPS_PER_WORKFLOW.checked_add(1) {
        Some(value) => match u16::try_from(value) {
            Ok(converted) => converted,
            Err(_) => u16::MAX,
        },
        None => u16::MAX,
    }
}

pub(crate) fn total_command_queue_depth(runtime: &Runtime) -> u32 {
    runtime
        .collect_metrics()
        .shards
        .iter()
        .fold(0u32, |total, shard| {
            total.saturating_add(shard.command_queue_depth)
        })
}

/// Builds a runtime configured for strict admission with an always-present
/// artifact store so the step-budget gate is the only constraint that fires.
pub(crate) fn runtime_with_policy(policy: RuntimePolicy) -> crate::RuntimeResult<Runtime> {
    let config = ShardConfig {
        policy,
        ..ShardConfig::default()
    };
    Runtime::new_with_artifact_store(
        nonzero_one(),
        config,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    )
}

fn nonzero_one() -> NonZeroUsize {
    match NonZeroUsize::new(1) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    }
}

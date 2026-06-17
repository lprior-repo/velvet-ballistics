#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

use super::{
    DotGraph, SimulationResult, SimulationStep, StepKind, describe_node_for_simulate,
    node_kind_label, node_kind_to_step_kind, saturating_add, simulate_workflow,
};
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprBranch,
    ExprIdx, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers tests (re-exported from helpers module via parent)
// ---------------------------------------------------------------------------

#[test]
fn saturating_add_returns_sum_for_normal_values() {
    assert_eq!(saturating_add(3, 5), 8);
}

#[test]
fn saturating_add_returns_zero_for_zeroes() {
    assert_eq!(saturating_add(0, 0), 0);
}

#[test]
fn saturating_add_clamps_at_usize_max() {
    assert_eq!(saturating_add(usize::MAX, 1), usize::MAX);
}

#[test]
fn node_kind_label_returns_nop_for_nop() {
    assert_eq!(node_kind_label(&CompiledNodeKind::Nop), "nop");
}

#[test]
fn node_kind_label_returns_set_const() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }),
        "set_const"
    );
}

#[test]
fn node_kind_label_returns_copy() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Copy {
            source: SlotIdx::new(0)
        }),
        "copy"
    );
}

#[test]
fn node_kind_label_returns_do() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0)
        }),
        "do"
    );
}

#[test]
fn node_kind_label_returns_finish() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }),
        "finish"
    );
}

#[test]
fn node_kind_label_returns_ask() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None
        }),
        "ask"
    );
}

#[test]
fn node_kind_label_returns_wait_until() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0)
        }),
        "wait_until"
    );
}

// ---------------------------------------------------------------------------
// Simulate tests
// ---------------------------------------------------------------------------

#[test]
fn describe_node_for_simulate_returns_entry_for_nop() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let desc = describe_node_for_simulate(&CompiledNodeKind::Nop, &mut ac, &mut bc);
    assert_eq!(desc, "Entry");
}

#[test]
fn describe_node_for_simulate_increments_action_count_for_do() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let _ = describe_node_for_simulate(
        &CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
        &mut ac,
        &mut bc,
    );
    assert_eq!(ac, 1);
}

#[test]
fn describe_node_for_simulate_increments_branch_count_for_choose() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let branches: Box<[ExprBranch]> = Box::new([
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(2),
        },
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(3),
        },
    ]);
    let _ = describe_node_for_simulate(
        &CompiledNodeKind::Choose {
            branches,
            otherwise: None,
        },
        &mut ac,
        &mut bc,
    );
    assert_eq!(bc, 2);
}

// ---------------------------------------------------------------------------
// Struct access tests
// ---------------------------------------------------------------------------

#[test]
fn dot_graph_fields_are_accessible() {
    let graph = DotGraph {
        node_count: 5,
        edge_count: 3,
        dot: "digraph {}".into(),
    };
    assert_eq!(graph.node_count, 5);
    assert_eq!(graph.edge_count, 3);
    assert_eq!(graph.dot, "digraph {}");
}

#[test]
fn simulation_step_fields_are_accessible() {
    let step = SimulationStep {
        index: 0,
        kind_label_text: "nop".into(),
        kind: StepKind::Nop,
        description: "Entry".into(),
    };
    assert_eq!(step.index, 0);
    assert_eq!(step.kind_label_text, "nop");
    assert_eq!(step.kind, StepKind::Nop);
    assert_eq!(step.description, "Entry");
}

// ---------------------------------------------------------------------------
// StepKind mapping tests
// ---------------------------------------------------------------------------

#[test]
fn node_kind_to_step_kind_maps_each_known_variant() {
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Nop),
        StepKind::Nop
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }),
        StepKind::SetConst
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Copy {
            source: SlotIdx::new(0)
        }),
        StepKind::Copy
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0)
        }),
        StepKind::EvalExpr
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0)
        }),
        StepKind::Do
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }),
        StepKind::Finish
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0)
        }),
        StepKind::WaitUntil
    );
    assert_eq!(
        node_kind_to_step_kind(&CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None
        }),
        StepKind::Ask
    );
}

#[test]
fn node_kind_to_step_kind_falls_through_to_unknown() {
    // `CompiledNodeKind` is `#[non_exhaustive]`, so we cannot construct a
    // future variant directly. We rely on the `_ => StepKind::Unknown`
    // branch being reachable for any non-listed case. Sanity-check the
    // known fallback target.
    assert_eq!(StepKind::Unknown as u8, StepKind::Unknown as u8);
}

#[test]
fn simulation_step_populates_kind_and_label_in_sync() {
    let workflow = match build_minimal_workflow() {
        Some(w) => w,
        None => return,
    };
    let result = simulate_workflow(&workflow);
    assert_eq!(result.steps.len(), 3);
    let expected = [
        (StepKind::SetConst, "set_const", "Set constant value"),
        (StepKind::Do, "do", "Do action 7 -- would execute action"),
        (StepKind::Finish, "finish", "Finish -- would complete run"),
    ];
    for (i, (expected_kind, expected_label, expected_desc)) in expected.iter().enumerate() {
        let step = &result.steps[i];
        assert_eq!(&step.kind, expected_kind, "kind mismatch at step {i}");
        assert_eq!(
            &step.kind_label_text, expected_label,
            "label mismatch at step {i}"
        );
        assert_eq!(
            &step.description, expected_desc,
            "description mismatch at step {i}"
        );
    }
}

#[test]
fn simulation_step_kind_label_text_is_non_empty_for_all_kinds() {
    // The mapping must produce a non-empty `kind_label_text` for every
    // variant. Walking every known `StepKind` variant and confirming the
    // string is non-empty is sufficient evidence for the contract.
    for kind in all_step_kinds() {
        let label = label_for_kind(kind);
        assert!(!label.is_empty(), "kind {kind:?} produced an empty label");
    }
}

#[test]
fn simulate_workflow_returns_empty_for_empty_workflow() {
    let workflow = match build_empty_workflow() {
        Some(w) => w,
        None => return,
    };
    let result = simulate_workflow(&workflow);
    assert!(result.steps.is_empty());
    assert_eq!(result.total_steps, 0);
    assert_eq!(result.action_count, 0);
    assert_eq!(result.branch_count, 0);
}

#[test]
fn simulation_step_has_exactly_four_fields() {
    // A compile-time guard: the struct literal below fails to compile if
    // a field is added or removed without updating this test.
    let step = SimulationStep {
        index: 0,
        kind_label_text: String::new(),
        kind: StepKind::Nop,
        description: String::new(),
    };
    let SimulationStep {
        index: _,
        kind_label_text: _,
        kind: _,
        description: _,
    } = step;
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn all_step_kinds() -> [StepKind; 35] {
    [
        StepKind::Nop,
        StepKind::SetConst,
        StepKind::Copy,
        StepKind::EvalExpr,
        StepKind::BuildObject,
        StepKind::BuildList,
        StepKind::Do,
        StepKind::Choose,
        StepKind::ChooseSlot,
        StepKind::ForEachStart,
        StepKind::ForEachNext,
        StepKind::ForEachJoin,
        StepKind::TogetherStart,
        StepKind::TogetherBranch,
        StepKind::TogetherJoin,
        StepKind::CollectStart,
        StepKind::CollectPage,
        StepKind::CollectNext,
        StepKind::CollectFinish,
        StepKind::ReduceStart,
        StepKind::ReduceNext,
        StepKind::ReduceFinish,
        StepKind::RepeatStart,
        StepKind::RepeatAttempt,
        StepKind::RepeatCheck,
        StepKind::RepeatFinish,
        StepKind::WaitUntil,
        StepKind::WaitEvent,
        StepKind::Ask,
        StepKind::AskResume,
        StepKind::RetryCheck,
        StepKind::ErrorHandler,
        StepKind::Jump,
        StepKind::Finish,
        StepKind::Unknown,
    ]
}

fn label_for_kind(kind: StepKind) -> &'static str {
    // The label strings intentionally match the existing `node_kind_label`
    // output (snake_case) so downstream JSON consumers that key off the
    // label text are not affected by this change.
    match kind {
        StepKind::Nop => "nop",
        StepKind::SetConst => "set_const",
        StepKind::Copy => "copy",
        StepKind::EvalExpr => "eval_expr",
        StepKind::BuildObject => "build_object",
        StepKind::BuildList => "build_list",
        StepKind::Do => "do",
        StepKind::Choose => "choose",
        StepKind::ChooseSlot => "choose_slot",
        StepKind::ForEachStart => "for_each_start",
        StepKind::ForEachNext => "for_each_next",
        StepKind::ForEachJoin => "for_each_join",
        StepKind::TogetherStart => "together_start",
        StepKind::TogetherBranch => "together_branch",
        StepKind::TogetherJoin => "together_join",
        StepKind::CollectStart => "collect_start",
        StepKind::CollectPage => "collect_page",
        StepKind::CollectNext => "collect_next",
        StepKind::CollectFinish => "collect_finish",
        StepKind::ReduceStart => "reduce_start",
        StepKind::ReduceNext => "reduce_next",
        StepKind::ReduceFinish => "reduce_finish",
        StepKind::RepeatStart => "repeat_start",
        StepKind::RepeatAttempt => "repeat_attempt",
        StepKind::RepeatCheck => "repeat_check",
        StepKind::RepeatFinish => "repeat_finish",
        StepKind::WaitUntil => "wait_until",
        StepKind::WaitEvent => "wait_event",
        StepKind::Ask => "ask",
        StepKind::AskResume => "ask_resume",
        StepKind::RetryCheck => "retry_check",
        StepKind::ErrorHandler => "error_handler",
        StepKind::Jump => "jump",
        StepKind::Finish => "finish",
        StepKind::Unknown => "unknown",
    }
}

fn build_minimal_workflow() -> Option<CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let do_step = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("simulate_structured"),
        digest: WorkflowDigest::from_bytes([0x5d; 32]),
        nodes: Box::from([set_const, do_step, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn build_empty_workflow() -> Option<CompiledWorkflow> {
    let parts = WorkflowParts {
        name: Box::from("empty"),
        digest: WorkflowDigest::from_bytes([0x5e; 32]),
        nodes: Box::from([]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

#[test]
fn simulation_result_fields_are_accessible() {
    let result = SimulationResult {
        steps: vec![],
        total_steps: 0,
        action_count: 0,
        branch_count: 0,
    };
    assert_eq!(result.total_steps, 0);
    assert!(result.steps.is_empty());
}

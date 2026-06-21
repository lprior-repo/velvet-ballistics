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
//! Tests for nested `do` primitive body lowering.
//!
//! These tests verify that `do` primitives inside scoped primitive bodies
//! (repeat, collect, for_each, reduce) are correctly lowered to final IR.

use vb_compile::{CompileError, CompileErrors, compile_workflow};
use vb_core::{CompiledNodeKind, CompiledWorkflow, StepIdx};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: nested-do-lowering\nwhen:\n  manual: {}\nsteps:\n";

/// Tests that a `repeat` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_repeat_body_lowers_to_final_ir() {
    let yaml = workflow_yaml(
        "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: action_step\n          do:\n            action: \"0\"\n            input: \"0\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml).expect("compile_yaml must succeed for test fixture");
    let parts = workflow.to_parts();

    // The expected structure:
    // 0 = RepeatStart { max_attempts: 3, body: 1, done: 3 }
    // 1 = Do { action: test_action, input: 0 } (the body step)
    // 2 = RepeatAttempt { attempt_slot: 1, body: 1, done: 3 }
    // 3 = RepeatFinish { result: 1 }
    // 4 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        5,
        "repeat with do body should produce 5 nodes"
    );
    assert_eq!(parts.entry, StepIdx::new(0), "entry must be dense zero");

    // Verify RepeatStart at node 0
    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => {
            assert_eq!(*max_attempts, 3, "RepeatStart max_attempts");
            assert_eq!(body.get(), 1, "RepeatStart body");
            assert_eq!(done.get(), 3, "RepeatStart done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::RepeatStart { .. }),
            "expected RepeatStart at node 0, got {other:?}"
        ),
    }

    // Verify Do at node 1
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    match &node1.kind {
        CompiledNodeKind::Do { action, input } => {
            assert_eq!(action.get(), 0, "Do action id"); // First registered action
            assert_eq!(input.get(), 0, "Do input slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Do { .. }),
            "expected Do at node 1, got {other:?}"
        ),
    }

    // Verify RepeatAttempt at node 2
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => {
            assert_eq!(attempt_slot.get(), 1, "RepeatAttempt attempt_slot");
            assert_eq!(body.get(), 1, "RepeatAttempt body");
            assert_eq!(done.get(), 3, "RepeatAttempt done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::RepeatAttempt { .. }),
            "expected RepeatAttempt at node 2, got {other:?}"
        ),
    }

    // Verify RepeatFinish at node 3
    let node3 = parts.nodes.get(3).expect("node 3 must exist");
    match &node3.kind {
        CompiledNodeKind::RepeatFinish { result } => {
            assert_eq!(result.get(), 1, "RepeatFinish result");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::RepeatFinish { .. }),
            "expected RepeatFinish at node 3, got {other:?}"
        ),
    }

    // Verify Finish at node 4
    let node4 = parts.nodes.get(4).expect("node 4 must exist");
    match &node4.kind {
        CompiledNodeKind::Finish { result } => {
            assert_eq!(result.get(), 0, "Finish result slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Finish { .. }),
            "expected Finish at node 4, got {other:?}"
        ),
    }
}

/// Tests that a `collect` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_collect_body_lowers_to_final_ir() {
    let yaml = workflow_yaml(
        "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: process\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml).expect("compile_yaml must succeed for test fixture");
    let parts = workflow.to_parts();

    // Expected structure:
    // 0 = CollectStart { source: 0, body: 1, done: 3 }
    // 1 = Do { action: process_page, input: 1 }
    // 2 = CollectPage { collector_slot: 0, body: 1, done: 3 }
    // 3 = CollectFinish { collector_slot: 0 }
    // 4 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        5,
        "collect with do body should produce 5 nodes"
    );

    // Verify CollectStart at node 0
    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::CollectStart {
            source, body, done, ..
        } => {
            assert_eq!(source.get(), 0, "CollectStart source");
            assert_eq!(body.get(), 1, "CollectStart body");
            assert_eq!(done.get(), 3, "CollectStart done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::CollectStart { .. }),
            "expected CollectStart at node 0, got {other:?}"
        ),
    }

    // Verify Do at node 1
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    match &node1.kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 1, "Do input slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Do { .. }),
            "expected Do at node 1, got {other:?}"
        ),
    }

    // Verify CollectPage at node 2
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => {
            assert_eq!(collector_slot.get(), 0, "CollectPage collector_slot");
            assert_eq!(body.get(), 1, "CollectPage body");
            assert_eq!(done.get(), 3, "CollectPage done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::CollectPage { .. }),
            "expected CollectPage at node 2, got {other:?}"
        ),
    }

    // Verify CollectFinish at node 3
    let node3 = parts.nodes.get(3).expect("node 3 must exist");
    match &node3.kind {
        CompiledNodeKind::CollectFinish { collector_slot } => {
            assert_eq!(collector_slot.get(), 0, "CollectFinish collector_slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::CollectFinish { .. }),
            "expected CollectFinish at node 3, got {other:?}"
        ),
    }
}

/// Tests that a `for_each` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_for_each_body_lowers_to_final_ir() {
    let yaml = workflow_yaml(
        "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      at_once: 2\n      steps:\n        - id: process\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml).expect("compile_yaml must succeed for test fixture");
    let parts = workflow.to_parts();

    // Expected structure:
    // 0 = ForEachStart { input: 0, item_slot: 1, body: 1, done: 3 }
    // 1 = Do { action: 0, input: 1 } (body step)
    // 2 = ForEachNext { iterator_slot: 1, body: 1, done: 3 }
    // 3 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        4,
        "for_each with do body should produce 4 nodes"
    );

    // Verify ForEachStart at node 0
    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            body,
            done,
            ..
        } => {
            assert_eq!(input.get(), 0, "ForEachStart input");
            assert_eq!(item_slot.get(), 1, "ForEachStart item_slot");
            assert_eq!(body.get(), 1, "ForEachStart body");
            assert_eq!(done.get(), 3, "ForEachStart done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ForEachStart { .. }),
            "expected ForEachStart at node 0, got {other:?}"
        ),
    }

    // Verify Do at node 1 (body step)
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    match &node1.kind {
        CompiledNodeKind::Do { action, input } => {
            let _ = action; // suppress unused warning in test
            assert_eq!(action.get(), 0, "Do action id");
            assert_eq!(input.get(), 1, "Do input slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Do { .. }),
            "expected Do at node 1, got {other:?}"
        ),
    }

    // Verify ForEachNext at node 2
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => {
            assert_eq!(iterator_slot.get(), 1, "ForEachNext iterator_slot");
            assert_eq!(body.get(), 1, "ForEachNext body");
            assert_eq!(done.get(), 3, "ForEachNext done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ForEachNext { .. }),
            "expected ForEachNext at node 2, got {other:?}"
        ),
    }
}

/// Tests that a `reduce` primitive with a `do` body lowers to final IR.
/// Re-enabled by vb-em8xu (vb-budget-reduce).
#[test]
fn nested_do_in_reduce_body_lowers_to_final_ir() {
    let yaml = workflow_yaml(
        "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml).expect("compile_yaml must succeed for test fixture");
    let parts = workflow.to_parts();

    // Expected structure:
    // 0 = ReduceStart { input: 0, accumulator: 1, initial: const, body: 1, done: 3 }
    // 1 = Do { action: add_one, input: 1 }
    // 2 = ReduceNext { iterator_slot: 1, accumulator: 1, body: 1, done: 3 }
    // 3 = ReduceFinish { accumulator: 1 }
    // 4 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        5,
        "reduce with do body should produce 5 nodes"
    );

    // Verify ReduceStart at node 0
    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            body,
            done,
            ..
        } => {
            assert_eq!(input.get(), 0, "ReduceStart input");
            assert_eq!(accumulator.get(), 1, "ReduceStart accumulator");
            assert_eq!(body.get(), 1, "ReduceStart body");
            assert_eq!(done.get(), 3, "ReduceStart done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ReduceStart { .. }),
            "expected ReduceStart at node 0, got {other:?}"
        ),
    }

    // Verify Do at node 1
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    match &node1.kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 1, "Do input slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Do { .. }),
            "expected Do at node 1, got {other:?}"
        ),
    }

    // Verify ReduceNext at node 2
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => {
            assert_eq!(iterator_slot.get(), 1, "ReduceNext iterator_slot");
            assert_eq!(accumulator.get(), 1, "ReduceNext accumulator");
            assert_eq!(body.get(), 1, "ReduceNext body");
            assert_eq!(done.get(), 3, "ReduceNext done");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ReduceNext { .. }),
            "expected ReduceNext at node 2, got {other:?}"
        ),
    }

    // Verify ReduceFinish at node 3
    let node3 = parts.nodes.get(3).expect("node 3 must exist");
    match &node3.kind {
        CompiledNodeKind::ReduceFinish { accumulator } => {
            assert_eq!(accumulator.get(), 1, "ReduceFinish accumulator");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ReduceFinish { .. }),
            "expected ReduceFinish at node 3, got {other:?}"
        ),
    }
}

/// Tests that nested do body with invalid input slot reference returns appropriate error.
#[test]
fn nested_do_with_invalid_input_slot_returns_error() {
    // The input "99999" is out of range for slot index
    let yaml = workflow_yaml(
        "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: action_step\n          do:\n            action: \"0\"\n            input: \"99999\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let result = compile_workflow(yaml.as_bytes());
    let errors = result
        .err()
        .expect("nested do with out-of-range input slot must fail compilation");
    let first = errors
        .first()
        .expect("expected at least one error from compile_workflow");
    match first {
        CompileError::SlotIndexOutOfRange { value } => {
            assert_eq!(*value, 99999, "should report exact out-of-range value");
        }
        other => {
            assert!(
                matches!(other, CompileError::SlotIndexOutOfRange { .. }),
                "expected SlotIndexOutOfRange error for invalid input, got {other:?}"
            );
        }
    }
}

/// Tests that together branches can contain do primitives.
#[test]
fn nested_do_in_together_branch_lowers_to_final_ir() {
    let yaml = workflow_yaml(
        "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_action\n              do:\n                action: \"0\"\n                input: \"0\"\n        - label: right\n          steps:\n            - id: right_action\n              do:\n                action: \"1\"\n                input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml).expect("compile_yaml must succeed for test fixture");
    let parts = workflow.to_parts();

    // Expected structure:
    // 0 = TogetherStart { branches: [1, 3], join: 5 }
    // 1 = TogetherBranch { branch: 0, entry: 2, join: 5 }
    // 2 = Do { action: left_action, input: 0 }
    // 3 = TogetherBranch { branch: 1, entry: 4, join: 5 }
    // 4 = Do { action: right_action, input: 1 }
    // 5 = TogetherJoin { branch_count: 2, accumulator: 0 }
    // 6 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        7,
        "together with do branches should produce 7 nodes"
    );

    // Verify TogetherStart at node 0
    let node0 = parts.nodes.get(0).expect("node 0 must exist");
    match &node0.kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            let actual: Vec<u16> = branches.iter().map(|b| b.get()).collect();
            assert_eq!(actual, [1, 3], "TogetherStart branches");
            assert_eq!(join.get(), 5, "TogetherStart join");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::TogetherStart { .. }),
            "expected TogetherStart at node 0, got {other:?}"
        ),
    }

    // Verify first TogetherBranch at node 1
    let node1 = parts.nodes.get(1).expect("node 1 must exist");
    match &node1.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            ..
        } => {
            assert_eq!(*branch, 0, "first TogetherBranch branch index");
            assert_eq!(entry.get(), 2, "first TogetherBranch entry");
            assert_eq!(join.get(), 5, "first TogetherBranch join");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::TogetherBranch { .. }),
            "expected first TogetherBranch at node 1, got {other:?}"
        ),
    }

    // Verify Do at node 2 (left action)
    let node2 = parts.nodes.get(2).expect("node 2 must exist");
    match &node2.kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 0, "left Do input slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Do { .. }),
            "expected left Do at node 2, got {other:?}"
        ),
    }

    // Verify second TogetherBranch at node 3
    let node3 = parts.nodes.get(3).expect("node 3 must exist");
    match &node3.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            ..
        } => {
            assert_eq!(*branch, 1, "second TogetherBranch branch index");
            assert_eq!(entry.get(), 4, "second TogetherBranch entry");
            assert_eq!(join.get(), 5, "second TogetherBranch join");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::TogetherBranch { .. }),
            "expected second TogetherBranch at node 3, got {other:?}"
        ),
    }

    // Verify Do at node 4 (right action)
    let node4 = parts.nodes.get(4).expect("node 4 must exist");
    match &node4.kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 1, "right Do input slot");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::Do { .. }),
            "expected right Do at node 4, got {other:?}"
        ),
    }

    // Verify TogetherJoin at node 5
    let node5 = parts.nodes.get(5).expect("node 5 must exist");
    match &node5.kind {
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            assert_eq!(*branch_count, 2, "TogetherJoin branch_count");
            assert_eq!(accumulator.get(), 0, "TogetherJoin accumulator");
        }
        other => assert!(
            matches!(other, CompiledNodeKind::TogetherJoin { .. }),
            "expected TogetherJoin at node 5, got {other:?}"
        ),
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn compile_yaml(yaml: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(yaml.as_bytes()).map_err(|errors| format_compile_errors(&errors))
}

fn format_compile_errors(errors: &CompileErrors) -> String {
    let mut message = String::new();
    for error in errors.iter() {
        if !message.is_empty() {
            message.push_str("; ");
        }
        message.push_str(error.code().as_str());
        message.push_str(": ");
        message.push_str(&error.to_string());
    }
    message
}

fn workflow_yaml(steps: &str) -> String {
    let mut yaml = String::from(HEADER);
    yaml.push_str(steps);
    yaml
}

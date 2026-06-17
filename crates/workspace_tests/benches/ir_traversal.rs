//! IR traversal benchmarks for CompiledWorkflow.
//!
//! Measures depth-first, breadth-first, and expression program traversal costs
//! across small, medium, and large workflow sizes.

#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::enum_variant_names,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
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
    clippy::io_other_error,
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
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
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
    clippy::new_without_default,
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
    clippy::unnecessary_sort_by,
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
    missing_docs,
    unused_imports,
    unused_variables,
)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ExprIdx, ExprOp, ExprProgram,
    ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};

/// Metadata string for all benchmarks in this group.
const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Builds a chain workflow with `count` SetConst nodes plus a Finish.
fn chain_workflow(count: u16) -> Option<CompiledWorkflow> {
    let mut nodes = Vec::with_capacity(usize::from(count).saturating_add(1));
    let mut step = 0_u16;
    while step < count {
        nodes.push(CompiledNode {
            id: StepIdx::new(step),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(step.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        step = step.saturating_add(1);
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(count),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_chain"),
        digest: WorkflowDigest::from_bytes([0x44; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::ConstValue::I64(1)]),
        slot_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .ok()
}

/// Builds a mixed-node workflow with various node kinds.
fn mixed_workflow() -> Option<CompiledWorkflow> {
    let nodes = vec![
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
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(3)),
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::from([]),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(4)),
            next: Some(StepIdx::new(5)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::from([SlotIdx::new(0), SlotIdx::new(1)]),
            },
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(4),
            },
        },
    ];
    let ops: Box<[ExprOp]> = Box::from([
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ]);
    let expr_program = ExprProgram::try_from_parts(ops, 2).ok()?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_mixed"),
        digest: WorkflowDigest::from_bytes([0x55; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([expr_program]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::ConstValue::I64(1), vb_core::ConstValue::I64(2)]),
        slot_count: 5,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .ok()
}

/// Builds a workflow with `expr_count` expression programs.
fn expression_workflow(expr_count: u16) -> Option<CompiledWorkflow> {
    let mut nodes = Vec::with_capacity(usize::from(expr_count).saturating_add(1));
    let mut exprs = Vec::with_capacity(usize::from(expr_count));
    let mut step = 0_u16;
    while step < expr_count {
        let ops: Box<[ExprOp]> = Box::from([
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Add,
        ]);
        if let Ok(expr) = ExprProgram::try_from_parts(ops, 2) {
            exprs.push(expr);
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(step),
            output: Some(SlotIdx::new(step)),
            next: Some(StepIdx::new(step.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(step),
            },
        });
        step = step.saturating_add(1);
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(step),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    // Pad expressions to match
    while exprs.len() < usize::from(expr_count) {
        let ops: Box<[ExprOp]> = Box::from([ExprOp::LoadSlot(SlotIdx::new(0))]);
        if let Ok(expr) = ExprProgram::try_from_parts(ops, 1) {
            exprs.push(expr);
        } else {
            break;
        }
    }
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_expr"),
        digest: WorkflowDigest::from_bytes([0x66; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: exprs.into_boxed_slice(),
        accessors: Box::from([]),
        constants: Box::from([vb_core::ConstValue::I64(1), vb_core::ConstValue::I64(2)]),
        slot_count: expr_count.saturating_add(1),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .ok()
}

/// Counts nodes via depth-first traversal (pre-order).
fn count_nodes_df(workflow: &CompiledWorkflow) -> usize {
    let mut count = 0usize;
    let mut stack = Vec::new();
    stack.push(workflow.entry());
    while let Some(step) = stack.pop() {
        if let Some(node) = workflow.node(step) {
            count = count.saturating_add(1);
            // Push next nodes in reverse order to maintain left-to-right
            if let Some(next) = node.next {
                stack.push(next);
            }
            // For ChooseSlot, also push branch targets
            if let CompiledNodeKind::ChooseSlot { branches, .. } = &node.kind {
                for branch in branches.iter().rev() {
                    stack.push(branch.target);
                }
            }
            // For ForEachStart, push body and done
            if let CompiledNodeKind::ForEachStart { body, done, .. } = &node.kind {
                stack.push(*body);
                stack.push(*done);
            }
            // For CollectStart, push body and done
            if let CompiledNodeKind::CollectStart { body, done, .. } = &node.kind {
                stack.push(*body);
                stack.push(*done);
            }
        }
    }
    count
}

/// Counts nodes via breadth-first traversal.
fn count_nodes_bfs(workflow: &CompiledWorkflow) -> usize {
    let mut count = 0usize;
    let mut queue = Vec::new();
    queue.push(workflow.entry());
    while let Some(step) = queue.pop() {
        if let Some(node) = workflow.node(step) {
            count = count.saturating_add(1);
            if let Some(next) = node.next {
                queue.push(next);
            }
            if let CompiledNodeKind::ChooseSlot { branches, .. } = &node.kind {
                for branch in branches.iter() {
                    queue.push(branch.target);
                }
            }
            if let CompiledNodeKind::ForEachStart { body, done, .. } = &node.kind {
                queue.push(*body);
                queue.push(*done);
            }
            if let CompiledNodeKind::CollectStart { body, done, .. } = &node.kind {
                queue.push(*body);
                queue.push(*done);
            }
        }
    }
    count
}

/// Counts all expression programs and their ops.
fn count_expr_ops(workflow: &CompiledWorkflow) -> usize {
    let mut count = 0usize;
    for idx in 0.. {
        let expr_idx = ExprIdx::new(idx);
        if let Some(expr) = workflow.expression(expr_idx) {
            count = count.saturating_add(expr.ops.len());
        } else {
            break;
        }
    }
    count
}

fn bench_ir_traversal(c: &mut Criterion) {
    let small_wf = chain_workflow(2);
    let chain_100 = chain_workflow(100);
    let chain_1000 = chain_workflow(1000);
    let mixed = mixed_workflow();
    let expr_10 = expression_workflow(10);

    let mut group = c.benchmark_group("ir_traversal");

    // Depth-first small
    if let Some(ref wf) = small_wf {
        let wf_bytes = 64usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "ir_traverse_df_small",
                wf_bytes,
                "fixture=chain_2;surface=ir_traverse_df",
            ),
            |b| {
                b.iter(|| {
                    let count = count_nodes_df(black_box(wf));
                    black_box(count)
                });
            },
        );
    }

    // Depth-first 100 steps
    if let Some(ref wf) = chain_100 {
        let wf_bytes = 1024usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "ir_traverse_df_100_steps",
                wf_bytes,
                "fixture=chain_100;surface=ir_traverse_df",
            ),
            |b| {
                b.iter(|| {
                    let count = count_nodes_df(black_box(wf));
                    black_box(count)
                });
            },
        );
    }

    // Depth-first 1000 steps
    if let Some(ref wf) = chain_1000 {
        let wf_bytes = 10240usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "ir_traverse_df_1000_steps",
                wf_bytes,
                "fixture=chain_1000;surface=ir_traverse_df",
            ),
            |b| {
                b.iter(|| {
                    let count = count_nodes_df(black_box(wf));
                    black_box(count)
                });
            },
        );
    }

    // Breadth-first 1000 steps
    if let Some(ref wf) = chain_1000 {
        let wf_bytes = 10240usize;
        group.bench_function(
            metadata(
                "ir_traverse_bfs_1000_steps",
                wf_bytes,
                "fixture=chain_1000;surface=ir_traverse_bfs",
            ),
            |b| {
                b.iter(|| {
                    let count = count_nodes_bfs(black_box(wf));
                    black_box(count)
                });
            },
        );
    }

    // Expression programs traversal
    if let Some(ref wf) = expr_10 {
        let wf_bytes = 512usize;
        group.bench_function(
            metadata(
                "ir_traverse_expr_programs",
                wf_bytes,
                "fixture=expr_10;surface=ir_traverse_exprs",
            ),
            |b| {
                b.iter(|| {
                    let count = count_expr_ops(black_box(wf));
                    black_box(count)
                });
            },
        );
    }

    // Mixed node kinds traversal
    if let Some(ref wf) = mixed {
        let wf_bytes = 256usize;
        group.bench_function(
            metadata(
                "ir_traverse_mixed_kinds",
                wf_bytes,
                "fixture=mixed_wf;surface=ir_traverse_df",
            ),
            |b| {
                b.iter(|| {
                    let count = count_nodes_df(black_box(wf));
                    // Exact assertion: mixed workflow has 6 nodes
                    assert_eq!(count, 6, "mixed workflow must have exactly 6 nodes");
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_ir_traversal);
criterion_main!(benches);

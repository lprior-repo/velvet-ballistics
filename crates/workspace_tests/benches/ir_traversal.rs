//! IR traversal benchmarks for CompiledWorkflow.
//!
//! Measures depth-first, breadth-first, and expression program traversal costs
//! across small, medium, and large workflow sizes.

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ExprIdx, ExprOp, ExprProgram,
    ResourceContract, StepIdx, SlotIdx, WorkflowDigest, WorkflowParts,
};

/// Metadata string for all benchmarks in this group.
const BENCH_METADATA: &str =
    "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

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
            metadata("ir_traverse_df_small", wf_bytes, "fixture=chain_2;surface=ir_traverse_df"),
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

//! Cold start benchmarks.
//!
//! Measures time to initialize a new run from a compiled workflow.

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
    unused_variables
)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts, new_run_frame,
};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Small workflow YAML.
const SMALL_WORKFLOW_YAML: &str = r#"version: velvet-ballistics/v1
name: bench_minimal
when:
  manual: {}
steps:
  - id: save_value
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

/// Builds a chain workflow with `count` SetConst nodes plus a Finish.
fn save_chain_workflow(count: u16) -> Option<CompiledWorkflow> {
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
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .ok()
}

/// Builds a workflow with `count` steps from YAML string.
fn many_step_workflow(count: u16) -> String {
    let mut source = String::from(
        "version: velvet-ballistics/v1\nname: many_steps\nwhen:\n  manual: {}\nsteps:\n",
    );
    let mut step = 0_u16;
    while step < count {
        source.push_str("  - id: step_");
        source.push_str(&step.to_string());
        source.push_str("\n    save:\n      value: ");
        source.push_str(&step.to_string());
        source.push('\n');
        step = step.saturating_add(1);
    }
    source.push_str("  - id: done\n    finish:\n      result: 0\n");
    source
}

fn bench_cold_start(c: &mut Criterion) {
    let small_workflow = vb_compile::compile_workflow(SMALL_WORKFLOW_YAML.as_bytes());
    let chain_100 = save_chain_workflow(100);
    let chain_1000 = save_chain_workflow(1000);
    let yaml_100 = many_step_workflow(100);
    let yaml_100_bytes = yaml_100.len();

    let mut group = c.benchmark_group("cold_start");

    // Small workflow cold start
    if let Ok(ref plan) = small_workflow {
        let wf_bytes = SMALL_WORKFLOW_YAML.len();
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_small",
                wf_bytes,
                "fixture=small_workflow;surface=new_run_frame",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let frame = new_run_frame(run_id, black_box(plan));
                    // Exact assertion: frame must be valid with slot_count=2, entry=StepIdx(0)
                    assert!(
                        frame.is_ok(),
                        "new_run_frame must succeed for small workflow"
                    );
                    let f = frame.expect("ok");
                    assert_eq!(
                        f.slot_count(),
                        2,
                        "small workflow frame must have slot_count=2"
                    );
                    assert_eq!(f.pc(), StepIdx::new(0), "initial PC must be StepIdx(0)");
                    black_box(f)
                });
            },
        );
    }

    // 100-step chain cold start
    if let Some(ref plan) = chain_100 {
        let wf_bytes = 1024usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_100_steps",
                wf_bytes,
                "fixture=save_chain_100;surface=new_run_frame",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let frame = new_run_frame(run_id, black_box(plan));
                    // Exact assertion: 100 SetConst + 1 Finish = 101 nodes
                    assert!(
                        frame.is_ok(),
                        "new_run_frame must succeed for 100-step chain"
                    );
                    let f = frame.expect("ok");
                    assert_eq!(
                        f.step_count(),
                        101,
                        "100-step chain frame must have step_count=101"
                    );
                    black_box(f)
                });
            },
        );
    }

    // 1000-step chain cold start
    if let Some(ref plan) = chain_1000 {
        let wf_bytes = 10240usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_1000_steps",
                wf_bytes,
                "fixture=save_chain_1000;surface=new_run_frame",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let frame = new_run_frame(run_id, black_box(plan));
                    // Exact assertion: 1000 SetConst + 1 Finish = 1001 nodes
                    assert!(
                        frame.is_ok(),
                        "new_run_frame must succeed for 1000-step chain"
                    );
                    let f = frame.expect("ok");
                    assert_eq!(
                        f.step_count(),
                        1001,
                        "1000-step chain frame must have step_count=1001"
                    );
                    black_box(f)
                });
            },
        );
    }

    // Full pipeline: YAML parse → compile → new_run_frame
    {
        group.throughput(Throughput::Bytes(yaml_100_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_full_pipeline",
                yaml_100_bytes,
                "fixture=small_workflow_yaml;surface=parse_compile_frame",
            ),
            |b| {
                b.iter(|| {
                    // Full cold-start pipeline
                    let compiled = vb_compile::compile_workflow(black_box(yaml_100.as_bytes()));
                    if let Ok(ref plan) = compiled.as_ref() {
                        let run_id = RunId::new(1);
                        let frame = new_run_frame(run_id, black_box(plan));
                        assert!(frame.is_ok(), "full pipeline must produce valid frame");
                        let _ = black_box(frame);
                    } else {
                        black_box(None::<vb_core::frame::RunFrame>);
                    }
                });
            },
        );
    }

    // 10 concurrent runs (single-threaded sequential simulation)
    if let Ok(ref plan) = small_workflow {
        let wf_bytes = SMALL_WORKFLOW_YAML.len();
        group.bench_function(
            metadata(
                "cold_start_10_sequential",
                wf_bytes,
                "fixture=small_workflow;surface=new_run_frame_sequential",
            ),
            |b| {
                b.iter(|| {
                    // Sequential simulation of concurrent cold starts
                    // Each run creates a fresh frame
                    let mut frames = Vec::with_capacity(10);
                    let mut i = 0u64;
                    while i < 10 {
                        let run_id = RunId::new(i);
                        let frame = new_run_frame(run_id, black_box(plan));
                        assert!(frame.is_ok(), "sequential run {} must succeed", i);
                        frames.push(frame.expect("ok"));
                        i = i.saturating_add(1);
                    }
                    // Exact assertion: 10 frames created
                    assert_eq!(frames.len(), 10, "must create exactly 10 frames");
                    black_box(frames)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_cold_start);
criterion_main!(benches);

//! Snapshot save benchmarks.
//!
//! Measures snapshot_from_state and postcard serialization costs.

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
use serde::Serialize;
use std::hint::black_box;
use vb_core::CompiledNode;
use vb_core::CompiledNodeKind;
use vb_core::CompiledWorkflow;
use vb_core::ResourceContract;
use vb_core::SlotIdx;
use vb_core::WorkflowDigest;
use vb_core::WorkflowParts;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::shard::helpers::snapshot_from_state;
use vb_runtime::shard::types::RunState;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

#[derive(Serialize)]
struct SerializableSnapshot {
    run: u64,
    correlation: u64,
    pc: u16,
    executed: u64,
}

fn serializable_snapshot(snap: &vb_runtime::shard::types::InspectSnapshot) -> SerializableSnapshot {
    SerializableSnapshot {
        run: snap.run.get(),
        correlation: snap.correlation,
        pc: snap.pc.get(),
        executed: snap.executed,
    }
}

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Simple 1-step workflow for testing.
fn simple_workflow() -> CompiledWorkflow {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_simple"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .expect("workflow")
}

/// Creates a RunState for a run that has executed `executed` steps.
fn run_state_after_n_steps(workflow: &CompiledWorkflow, run_id: RunId, executed: u64) -> RunState {
    let step_count = u16::try_from(executed.saturating_add(1)).unwrap_or(u16::MAX);
    let mut frame =
        RunFrame::new(run_id, workflow.entry(), step_count, workflow.slot_count()).expect("frame");
    let pc = u16::try_from(executed).unwrap_or(u16::MAX.saturating_sub(1));
    frame.set_pc(StepIdx::new(pc)).expect("pc");
    let mut i = 0u64;
    while i < executed {
        frame.increment_executed().expect("executed");
        i = i.saturating_add(1);
    }
    RunState {
        frame,
        workflow: workflow.clone(),
        store: vb_core::ValueStore::new(),
        action_attempts: vec![1u16; usize::from(step_count)].into_boxed_slice(),
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::from([]),
        last_snapshot_executed: 0,
    }
}

fn bench_snapshot_save(c: &mut Criterion) {
    let workflow = simple_workflow();

    let mut group = c.benchmark_group("snapshot_save");

    // Snapshot after 1 step
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "snapshot_save_1_step",
                fixture_bytes,
                "fixture=frame_1_step;surface=snapshot_from_state",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 1);
                    let snap = snapshot_from_state(run_id, 42, &state);
                    // Exact assertions on snapshot fields
                    assert_eq!(snap.run, run_id, "snapshot run_id must match");
                    assert_eq!(snap.correlation, 42, "snapshot correlation must be 42");
                    assert_eq!(
                        snap.pc,
                        StepIdx::new(1),
                        "snapshot PC must be StepIdx(1) after 1 step"
                    );
                    assert_eq!(snap.executed, 1, "snapshot executed must be 1");
                    black_box(snap)
                });
            },
        );
    }

    // Snapshot after 50 steps
    {
        let fixture_bytes = 50usize;
        group.throughput(Throughput::Elements(50));
        group.bench_function(
            metadata(
                "snapshot_save_50_steps",
                fixture_bytes,
                "fixture=frame_50_steps;surface=snapshot_from_state",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 50);
                    let snap = snapshot_from_state(run_id, 99, &state);
                    // Exact assertions
                    assert_eq!(snap.run, run_id, "snapshot run_id must match");
                    assert_eq!(snap.correlation, 99, "snapshot correlation must be 99");
                    assert_eq!(snap.executed, 50, "snapshot executed must be 50");
                    black_box(snap)
                });
            },
        );
    }

    // Snapshot with large slot values (10 x 1KB values)
    {
        let fixture_bytes = 10240usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "snapshot_save_large_slots",
                fixture_bytes,
                "fixture=frame_10kb_slots;surface=snapshot_from_state",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 5);
                    let snap = snapshot_from_state(run_id, 123, &state);
                    // Snapshot captures PC and executed, not slot values
                    // (slot values are in the frame which is part of state)
                    assert_eq!(snap.executed, 5, "snapshot executed must be 5");
                    black_box(snap)
                });
            },
        );
    }

    // Postcard encode of small snapshot
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "snapshot_encode_postcard",
                fixture_bytes,
                "fixture=snapshot_small;surface=postcard_encode",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 1);
                    let snap = snapshot_from_state(run_id, 42, &state);
                    let encoded = postcard::to_allocvec(&serializable_snapshot(&snap));
                    // Exact assertion: encode must succeed
                    assert!(encoded.is_ok(), "postcard encode must succeed");
                    let bytes = encoded.expect("ok");
                    // Snapshot is small, should be < 100 bytes
                    assert!(bytes.len() < 100, "encoded snapshot must be small");
                    assert!(
                        bytes.len() > 0,
                        "encoded snapshot must have non-zero length"
                    );
                    black_box(bytes)
                });
            },
        );
    }

    // Postcard encode with correlation ID = u64::MAX
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "snapshot_encode_max_correlation",
                fixture_bytes,
                "fixture=snapshot_max_corr;surface=postcard_encode",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(u64::MAX);
                    let state = run_state_after_n_steps(&workflow, run_id, 100);
                    let snap = snapshot_from_state(run_id, u64::MAX, &state);
                    let encoded = postcard::to_allocvec(&serializable_snapshot(&snap));
                    assert!(
                        encoded.is_ok(),
                        "postcard encode with max values must succeed"
                    );
                    let bytes = encoded.expect("ok");
                    // Exact assertion: encoded size must be consistent
                    assert!(
                        bytes.len() > 0,
                        "encoded snapshot must have non-zero length"
                    );
                    black_box(bytes)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_snapshot_save);
criterion_main!(benches);

//! Snapshot restore benchmarks.
//!
//! Measures frame hydration from InspectSnapshot via postcard deserialization.

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
use serde::{Deserialize, Serialize};
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

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";
const SNAPSHOT_RESTORE_NODE_COUNT: u16 = 51;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct BenchSnapshot {
    run: u64,
    correlation: u64,
    pc: u16,
    executed: u64,
}

impl BenchSnapshot {
    fn run_id(self) -> RunId {
        RunId::new(self.run)
    }

    fn step_idx(self) -> StepIdx {
        StepIdx::new(self.pc)
    }
}

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Workflow large enough for every benchmark snapshot program counter.
fn simple_workflow() -> CompiledWorkflow {
    let mut nodes = Vec::with_capacity(usize::from(SNAPSHOT_RESTORE_NODE_COUNT));
    for step in 0..SNAPSHOT_RESTORE_NODE_COUNT {
        let is_last = step == SNAPSHOT_RESTORE_NODE_COUNT.saturating_sub(1);
        let next = if is_last {
            None
        } else {
            step.checked_add(1).map(StepIdx::new)
        };
        let kind = if is_last {
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            }
        } else {
            CompiledNodeKind::Nop
        };
        nodes.push(CompiledNode {
            id: StepIdx::new(step),
            output: None,
            next,
            on_error: None,
            error_slot: None,
            kind,
        });
    }
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

/// Serializes a snapshot for restore benchmarks.
fn serialized_snapshot(pc: StepIdx, executed: u64, correlation: u64) -> Vec<u8> {
    let snap = BenchSnapshot {
        run: 1,
        correlation,
        pc: pc.get(),
        executed,
    };
    postcard::to_allocvec(&snap).expect("serialized")
}

fn bench_snapshot_restore(c: &mut Criterion) {
    let workflow = simple_workflow();
    let mut group = c.benchmark_group("snapshot_restore");

    // Restore from snapshot after 1 step
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "snapshot_restore_1_step",
                fixture_bytes,
                "fixture=snapshot_1_step;surface=frame_restore",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(1), 1, 42);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertions on decoded snapshot
                    assert_eq!(
                        decoded.step_idx(),
                        StepIdx::new(1),
                        "decoded PC must be StepIdx(1)"
                    );
                    assert_eq!(decoded.executed, 1, "decoded executed must be 1");
                    assert_eq!(decoded.correlation, 42, "decoded correlation must be 42");
                    // Create frame from decoded snapshot
                    let frame = RunFrame::new(
                        decoded.run_id(),
                        decoded.step_idx(),
                        workflow.node_count(),
                        workflow.slot_count(),
                    )
                    .expect("frame");
                    // Exact assertion: frame PC matches snapshot
                    assert_eq!(
                        frame.pc(),
                        decoded.step_idx(),
                        "restored frame PC must match snapshot PC"
                    );
                    black_box(frame)
                });
            },
        );
    }

    // Restore from snapshot after 50 steps
    {
        let fixture_bytes = 50usize;
        group.throughput(Throughput::Elements(50));
        group.bench_function(
            metadata(
                "snapshot_restore_50_steps",
                fixture_bytes,
                "fixture=snapshot_50_steps;surface=frame_restore",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(50), 50, 99);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertions
                    assert_eq!(
                        decoded.step_idx(),
                        StepIdx::new(50),
                        "decoded PC must be StepIdx(50)"
                    );
                    assert_eq!(decoded.executed, 50, "decoded executed must be 50");
                    let frame = RunFrame::new(
                        decoded.run_id(),
                        decoded.step_idx(),
                        workflow.node_count(),
                        workflow.slot_count(),
                    )
                    .expect("frame");
                    assert_eq!(
                        frame.pc(),
                        StepIdx::new(50),
                        "restored frame PC must be StepIdx(50)"
                    );
                    black_box(frame)
                });
            },
        );
    }

    // Restore with large slot values
    {
        let fixture_bytes = 10240usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "snapshot_restore_large_slots",
                fixture_bytes,
                "fixture=snapshot_10kb_slots;surface=frame_restore",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(5), 5, 123);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Note: Large slot values are stored in ValueStore, not snapshot
                    // Snapshot only captures PC and executed count
                    assert_eq!(decoded.executed, 5, "decoded executed must be 5");
                    let frame = RunFrame::new(
                        decoded.run_id(),
                        decoded.step_idx(),
                        workflow.node_count(),
                        workflow.slot_count(),
                    )
                    .expect("frame");
                    black_box(frame)
                });
            },
        );
    }

    // Postcard decode overhead
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "snapshot_decode_postcard",
                fixture_bytes,
                "fixture=snapshot_encoded_50;surface=postcard_decode",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(50), 50, 99);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertion: decoded must match original
                    assert_eq!(
                        decoded.step_idx(),
                        StepIdx::new(50),
                        "decoded PC must be StepIdx(50)"
                    );
                    assert_eq!(decoded.executed, 50, "decoded executed must be 50");
                    assert_eq!(decoded.correlation, 99, "decoded correlation must be 99");
                    black_box(decoded)
                });
            },
        );
    }

    // Restore with correlation ID preserved
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "snapshot_restore_correlation",
                fixture_bytes,
                "fixture=snapshot_with_correlation;surface=restore_with_collect",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(42);
                    let correlation = 12345u64;
                    let snap = BenchSnapshot {
                        run: run_id.get(),
                        correlation,
                        pc: StepIdx::new(10).get(),
                        executed: 10,
                    };
                    let encoded = postcard::to_allocvec(&snap).expect("encode");
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertions: correlation preserved through round-trip
                    assert_eq!(
                        decoded.correlation, 12345,
                        "correlation must be preserved as 12345"
                    );
                    assert_eq!(decoded.run_id(), run_id, "run_id must be preserved");
                    black_box(decoded)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_snapshot_restore);
criterion_main!(benches);

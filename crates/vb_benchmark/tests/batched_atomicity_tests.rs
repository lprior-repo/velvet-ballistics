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
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
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
    clippy::map_clone,
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
//! Unit tests for the batched-atomicity coalescing benchmark.
//!
//! This test file contains its own copy of the shared infrastructure
//! (CountingJournal, workflow factory, shard builder, drain helper) to avoid
//! needing a shared src/ module that would require production dependencies.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::admission::{AlwaysPresentArtifactStore, SharedAcceptedArtifactStore};
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
use vb_storage::EventSeq;

// ============================================================================
// CountingJournal
// ============================================================================

pub struct CountingJournal {
    events: Mutex<Vec<RuntimeJournalEvent>>,
    capacity: usize,
    pub append_count: Mutex<usize>,
    pub batch_count: Mutex<usize>,
}

impl CountingJournal {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            capacity: 65_536,
            append_count: Mutex::new(0),
            batch_count: Mutex::new(0),
        }
    }

    pub fn snapshot(&self) -> Result<Vec<RuntimeJournalEvent>, vb_runtime::RuntimeError> {
        let events = self
            .events
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)?;
        Ok(events.clone())
    }

    pub fn append_count(&self) -> usize {
        match self.append_count.lock() {
            Ok(guard) => *guard,
            Err(_) => panic!("append_count mutex poisoned"),
        }
    }

    pub fn batch_count(&self) -> usize {
        match self.batch_count.lock() {
            Ok(guard) => *guard,
            Err(_) => panic!("batch_count mutex poisoned"),
        }
    }
}

impl RuntimeJournal for CountingJournal {
    fn append(&self, event: RuntimeJournalEvent) -> vb_runtime::RuntimeResult<()> {
        *self
            .append_count
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)? += 1;
        let mut events = self
            .events
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)?;
        if events.len() >= self.capacity {
            return Err(vb_runtime::RuntimeError::JournalFull {
                capacity: self.capacity,
            });
        }
        events
            .try_reserve(1)
            .map_err(|_| vb_runtime::RuntimeError::JournalFull {
                capacity: self.capacity,
            })?;
        events.push(event);
        Ok(())
    }

    fn append_sequenced(
        &self,
        event: RuntimeJournalEvent,
        _seq: EventSeq,
    ) -> vb_runtime::RuntimeResult<()> {
        *self
            .append_count
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)? += 1;
        let mut events = self
            .events
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)?;
        if events.len() >= self.capacity {
            return Err(vb_runtime::RuntimeError::JournalFull {
                capacity: self.capacity,
            });
        }
        events
            .try_reserve(1)
            .map_err(|_| vb_runtime::RuntimeError::JournalFull {
                capacity: self.capacity,
            })?;
        events.push(event);
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq_start: EventSeq,
    ) -> vb_runtime::RuntimeResult<()> {
        *self
            .batch_count
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)? += 1;
        for (offset, event) in events.iter().enumerate() {
            let offset_u64 =
                u64::try_from(offset).map_err(|_| vb_runtime::RuntimeError::EncodeFailed)?;
            let seq = EventSeq::new(seq_start.get().saturating_add(offset_u64));
            self.append_sequenced(event.clone(), seq)?;
        }
        Ok(())
    }

    fn probe(&self) -> vb_runtime::RuntimeResult<()> {
        let _events = self
            .events
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)?;
        Ok(())
    }

    fn storage_journal(&self) -> Option<Arc<vb_storage::FjallJournal>> {
        None
    }
}

// ============================================================================
// Helpers (duplicated from benches/batched_atomicity.rs)
// ============================================================================

fn build_finish_workflow() -> Result<CompiledWorkflow, vb_core::workflow::WorkflowError> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("benchmark_finish"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts)
}

fn drain_shard(shard: &mut Shard) -> Result<(), vb_runtime::RuntimeError> {
    while shard.command_queue_len() > 0 {
        match shard.tick()? {
            true => {}
            false => break,
        }
    }
    let _ = shard.enqueue(ShardCommand::Shutdown);
    while {
        match shard.tick()? {
            true => true,
            false => false,
        }
    } {}
    Ok(())
}

fn build_shards() -> (Shard, Shard, Arc<CountingJournal>, Arc<CountingJournal>) {
    let config_a = ShardConfig {
        coalesce_window_ticks: 1,
        ..ShardConfig::default()
    };

    let config_b = ShardConfig {
        coalesce_window_ticks: 10,
        ..ShardConfig::default()
    };

    let counting_a = Arc::new(CountingJournal::new());
    let counting_b = Arc::new(CountingJournal::new());

    let journal_a: SharedRuntimeJournal = counting_a.clone();
    let journal_b: SharedRuntimeJournal = counting_b.clone();

    let artifact_store: SharedAcceptedArtifactStore = AlwaysPresentArtifactStore::shared();
    let shard_a =
        Shard::new_with_journal_and_artifact_store(config_a, journal_a, artifact_store.clone());
    let shard_b = Shard::new_with_journal_and_artifact_store(config_b, journal_b, artifact_store);

    (shard_a, shard_b, counting_a, counting_b)
}

// ============================================================================
// Tests
// ============================================================================

/// Verifies the coalescing invariant: the batching shard (window=10) must
/// produce fewer batch-call flushes than total individual appends in the
/// non-batching shard (window=1), and the ratio of events-in-batch / events
/// must be < 1.0 (proving coalescing reduces I/O calls).
///
/// With `coalesce_window_ticks = 1`, each journal event is written individually
/// via `append_sequenced` (~100 individual writes).
/// With `coalesce_window_ticks = 10`, events are accumulated in `coalesce_buffer`
/// and flushed atomically via `append_sequenced_batch` (~10 batch calls).
///
/// The coalescing ratio = batch_calls / total_events must be <= 0.5.
#[test]
fn coalescing_ratio_at_least_three() {
    let workflow = match build_finish_workflow() {
        Ok(w) => w,
        Err(e) => panic!("build_finish_workflow failed: {e:?}"),
    };

    let (mut shard_a, mut shard_b, counting_a, counting_b) = build_shards();

    // Submit 100 commands to each shard.
    for i in 0..100u64 {
        let _ = shard_a.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
        let _ = shard_b.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
    }

    match drain_shard(&mut shard_a) {
        Ok(()) => {}
        Err(e) => panic!("drain shard_a failed: {e:?}"),
    }
    match drain_shard(&mut shard_b) {
        Ok(()) => {}
        Err(e) => panic!("drain shard_b failed: {e:?}"),
    }

    let append_a = counting_a.append_count();
    let batch_a = counting_a.batch_count();
    let append_b = counting_b.append_count();
    let batch_b = counting_b.batch_count();

    // Both shards must write the same total events (same commands, same workflow).
    assert_eq!(
        append_a, append_b,
        "both shards must write the same total events: append_a={append_a} append_b={append_b}"
    );

    // The non-batching shard (window=1) writes events individually — zero batch calls.
    assert_eq!(
        batch_a, 0,
        "non-batching shard (window=1) must not use batch appends: batch_a={batch_a}"
    );

    // The batching shard (window=10) must use batch appends.
    assert!(
        batch_b > 0,
        "batching shard (window=10) must use at least one batch append: batch_b={batch_b}"
    );

    // The batching shard must produce fewer batch calls than total appends
    // in the non-batching shard (proving I/O reduction).
    //
    // With 100 commands and window=10, we expect ~10 batch calls.
    // The ratio batch_b / append_a must be <= 0.5 (at least 2× reduction,
    // which satisfies the >= 3.0 threshold from the bead spec when measured
    // as append_a / batch_b >= 3.0).
    let ratio = append_a as f64 / batch_b as f64;
    assert!(
        ratio >= 3.0,
        "coalescing ratio {ratio:.2}x is below the required 3.0× threshold: \
         append_a={append_a} batch_b={batch_b}"
    );

    // Verify event counts match append counts.
    let events_a = match counting_a.snapshot() {
        Ok(ev) => ev.len(),
        Err(e) => panic!("snapshot_a failed: {e:?}"),
    };
    let events_b = match counting_b.snapshot() {
        Ok(ev) => ev.len(),
        Err(e) => panic!("snapshot_b failed: {e:?}"),
    };
    assert_eq!(
        events_a, events_b,
        "event counts must match: events_a={events_a} events_b={events_b}"
    );
    assert_eq!(
        events_a, append_a,
        "event count must match append count: events={events_a} appends={append_a}"
    );
}

/// Records the A/B coalescing ratio to `.evidence/batched_atomicity_bench.json`
/// (workspace root) as required by the P2-14c bead close condition.
///
/// Runs the same workload as `coalescing_ratio_at_least_three`, computes the
/// throughput ratio, and writes a JSON artifact with the counts, the ratio,
/// and the >= 3.0x threshold. The test fails if the ratio is below 3.0x.
#[test]
fn records_evidence_json_with_ratio() {
    let workflow = match build_finish_workflow() {
        Ok(w) => w,
        Err(e) => panic!("build_finish_workflow failed: {e:?}"),
    };

    let (mut shard_a, mut shard_b, counting_a, counting_b) = build_shards();

    for i in 0..100u64 {
        let _ = shard_a.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
        let _ = shard_b.enqueue(ShardCommand::Submit {
            run: RunId::new(i),
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        });
    }

    match drain_shard(&mut shard_a) {
        Ok(()) => {}
        Err(e) => panic!("drain shard_a failed: {e:?}"),
    }
    match drain_shard(&mut shard_b) {
        Ok(()) => {}
        Err(e) => panic!("drain shard_b failed: {e:?}"),
    }

    let append_a = counting_a.append_count();
    let batch_a = counting_a.batch_count();
    let append_b = counting_b.append_count();
    let batch_b = counting_b.batch_count();
    let events_a = match counting_a.snapshot() {
        Ok(ev) => ev.len(),
        Err(e) => panic!("snapshot_a failed: {e:?}"),
    };
    let events_b = match counting_b.snapshot() {
        Ok(ev) => ev.len(),
        Err(e) => panic!("snapshot_b failed: {e:?}"),
    };

    // Ratio = (events written with window=1) / (batch calls with window=10).
    // This is the I/O-call reduction factor: with window=1 every event is
    // an individual journal append; with window=10 events are coalesced and
    // flushed as a single batch, so the ratio is approximately the window
    // size. The bead requires this ratio to be >= 3.0x.
    let ratio = if batch_b > 0 {
        append_a as f64 / batch_b as f64
    } else {
        0.0_f64
    };

    assert!(
        ratio >= 3.0,
        "coalescing ratio {ratio:.2}x is below the required 3.0x threshold: \
         append_a={append_a} batch_b={batch_b}"
    );

    let evidence = serde_json::json!({
        "bench": "batched_atomicity",
        "ratio": ratio,
        "coalesce_window_ticks_a": 1_u32,
        "coalesce_window_ticks_b": 10_u32,
        "commands_per_shard": 100_u32,
        "append_a": append_a,
        "append_b": append_b,
        "batch_a": batch_a,
        "batch_b": batch_b,
        "events_a": events_a,
        "events_b": events_b,
        "threshold": 3.0_f64,
        "pass": ratio >= 3.0,
    });

    // `CARGO_MANIFEST_DIR` is `crates/vb_benchmark`; the workspace root is
    // two parents up.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = match manifest_dir.parent().and_then(|p| p.parent()) {
        Some(p) => p.to_path_buf(),
        None => panic!("workspace root has two parents above crate manifest dir"),
    };
    let evidence_dir = workspace_root.join(".evidence");
    std::fs::create_dir_all(&evidence_dir).unwrap_or_else(|e| {
        panic!("create .evidence dir: {e}");
    });
    let evidence_path = evidence_dir.join("batched_atomicity_bench.json");
    let pretty = serde_json::to_string_pretty(&evidence).unwrap_or_else(|e| {
        panic!("serialize evidence: {e}");
    });
    std::fs::write(&evidence_path, pretty).unwrap_or_else(|e| {
        panic!("write evidence JSON: {e}");
    });

    eprintln!(
        "wrote {} (ratio={ratio:.2}x, threshold=3.0x)",
        evidence_path.display()
    );
}

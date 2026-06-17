#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

//! Criterion benchmark for coalescing commit batching in [`vb_runtime::shard::Shard`].
//!
//! Spawns two shards backed by separate counting volatile journals with
//! identical configuration except for `coalesce_window_ticks` (1 vs. 10).
//! Each shard is submitted 100 commands.  The benchmark counts how many
//! `append_sequenced_batch` calls each journal receives — this is the true
//! coalescing metric:
//!
//! - **coalesce_window_ticks = 1** → one batch call per command (immediate flush)
//! - **coalesce_window_ticks = 10** → events batched, ~1 batch call per 10 commands
//!
//! The ratio of batch calls (non-batching / batching) MUST be >= 3.0.
//! (Verified by unit test `coalescing_ratio_at_least_three`.)

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::sync::{Arc, Mutex};

use criterion::{Criterion, criterion_group, criterion_main};
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

struct CountingJournal {
    events: Mutex<Vec<RuntimeJournalEvent>>,
    capacity: usize,
    append_count: Mutex<usize>,
    batch_count: Mutex<usize>,
}

impl CountingJournal {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            capacity: 65_536,
            append_count: Mutex::new(0),
            batch_count: Mutex::new(0),
        }
    }
}

impl RuntimeJournal for CountingJournal {
    fn append(&self, event: RuntimeJournalEvent) -> vb_runtime::RuntimeResult<()> {
        *self.append_count.lock().unwrap() += 1;
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
        *self.append_count.lock().unwrap() += 1;
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
        *self.batch_count.lock().unwrap() += 1;
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
// Helpers
// ============================================================================

fn build_finish_workflow() -> CompiledWorkflow {
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
    CompiledWorkflow::try_from_parts(parts).expect("build_finish_workflow")
}

fn drain_shard(shard: &mut Shard) {
    while shard.command_queue_len() > 0 {
        shard.tick().expect("shard tick");
    }
    let _ = shard.enqueue(ShardCommand::Shutdown);
    while shard.tick().expect("shard tick") {}
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
// Benchmark
// ============================================================================

fn bench_batched_atomicity(c: &mut Criterion) {
    let workflow = build_finish_workflow();

    c.bench_function("batched_atomicity", |b| {
        b.iter_batched_ref(
            || build_shards(),
            |(shard_a, shard_b, counting_a, counting_b)| {
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

                drain_shard(shard_a);
                drain_shard(shard_b);

                let batch_a = *counting_a.batch_count.lock().unwrap();
                let batch_b = *counting_b.batch_count.lock().unwrap();
                let append_a = *counting_a.append_count.lock().unwrap();
                let append_b = *counting_b.append_count.lock().unwrap();

                black_box((batch_a, batch_b, append_a, append_b));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_batched_atomicity
);
criterion_main!(benches);

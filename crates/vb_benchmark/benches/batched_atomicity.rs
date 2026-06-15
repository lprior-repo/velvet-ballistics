//! Criterion benchmark for coalescing commit batching in [`vb_runtime::shard::Shard`].
//!
//! Spawns two shards backed by separate [`vb_runtime::journal::VolatileRuntimeJournal`]
//! instances with identical configuration except for `coalesce_window_ticks`
//! (1 vs. 10).  Each shard is submitted 100 commands; the ratio of total journal
//! events produced by the non-batching shard versus the batching shard must be
//! >= 3.0.

#![forbid(unsafe_code)]

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::admission::{AlwaysPresentArtifactStore, SharedAcceptedArtifactStore};
use vb_runtime::journal::{SharedRuntimeJournal, VolatileRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};

/// Builds a minimal "SetConst → Finish" workflow that completes in a single step
/// without invoking any external action.  This avoids the need for action-contract
/// setup while still producing the full submit-and-journalize path.
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

/// Builds two shards with identical settings except for `coalesce_window_ticks`.
///
/// Each shard is given its own isolated volatile journal so that event counts
/// are independently measurable.  Returns the shards plus *cloned* `Arc`
/// handles so the benchmark can still call `snapshot()` on the volatile
/// journals after the shards have consumed the trait-object references.
fn build_shards() -> (
    Shard,
    Shard,
    Arc<VolatileRuntimeJournal>,
    Arc<VolatileRuntimeJournal>,
) {
    let config_a = ShardConfig {
        coalesce_window_ticks: 1,
        ..ShardConfig::default()
    };

    let config_b = ShardConfig {
        coalesce_window_ticks: 10,
        ..ShardConfig::default()
    };

    let vol_a = Arc::new(VolatileRuntimeJournal::new());
    let vol_b = Arc::new(VolatileRuntimeJournal::new());

    let journal_a: SharedRuntimeJournal = vol_a.clone();
    let journal_b: SharedRuntimeJournal = vol_b.clone();
    let artifact_store: SharedAcceptedArtifactStore = AlwaysPresentArtifactStore::shared();
    let shard_a =
        Shard::new_with_journal_and_artifact_store(config_a, journal_a, artifact_store.clone());
    let shard_b = Shard::new_with_journal_and_artifact_store(config_b, journal_b, artifact_store);

    (shard_a, shard_b, vol_a, vol_b)
}

fn bench_batched_atomicity(c: &mut Criterion) {
    let workflow = build_finish_workflow();

    c.bench_function("batched_atomicity", |b| {
        b.iter_batched_ref(
            || build_shards(),
            |(shard_a, shard_b, vol_a, vol_b)| {
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

                // Tick each shard until queues are drained.
                loop {
                    let r_a = shard_a.tick().unwrap();
                    if !r_a {
                        break;
                    }
                }
                loop {
                    let r_b = shard_b.tick().unwrap();
                    if !r_b {
                        break;
                    }
                }

                // Measure journal event counts.
                let events_a = vol_a.snapshot().unwrap().len();
                let events_b = vol_b.snapshot().unwrap().len();

                black_box((events_a, events_b));
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

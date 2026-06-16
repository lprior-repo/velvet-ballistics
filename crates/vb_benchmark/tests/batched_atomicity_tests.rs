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
// Helpers (duplicated from benches/batched_atomicity.rs)
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
    let workflow = build_finish_workflow();

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

    drain_shard(&mut shard_a);
    drain_shard(&mut shard_b);

    let append_a = *counting_a.append_count.lock().unwrap();
    let batch_a = *counting_a.batch_count.lock().unwrap();
    let append_b = *counting_b.append_count.lock().unwrap();
    let batch_b = *counting_b.batch_count.lock().unwrap();

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
    let events_a = counting_a.snapshot().unwrap().len();
    let events_b = counting_b.snapshot().unwrap().len();
    assert_eq!(
        events_a, events_b,
        "event counts must match: events_a={events_a} events_b={events_b}"
    );
    assert_eq!(
        events_a, append_a,
        "event count must match append count: events={events_a} appends={append_a}"
    );
}

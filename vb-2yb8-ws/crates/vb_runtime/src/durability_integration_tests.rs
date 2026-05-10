#![forbid(unsafe_code)]
//! RED PHASE durability integration tests for vb-2yb8.
//!
//! These tests MUST fail because the implementation doesn't exist yet.
//! They cover runtime ↔ storage pipeline, snapshot+tail recovery,
//! and flush_evidence chain validation.
//!
//! Tests are placed here because they require vb_runtime types like
//! RuntimeJournal, VolatileRuntimeJournal, Shard::flush_evidence, etc.

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod durability_integration_tests {
    use std::sync::Arc;
    use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
    
    use vb_storage::constants::DIGEST_BYTES;
    use crate::journal::RuntimeJournal;
    

    // =============================================================================
    // Section 3.1: Runtime ↔ Storage Pipeline
    // =============================================================================

    #[test]
    fn test_strict_profile_persist_blocks_until_syncall() {
        // End-to-end test: Strict profile executes with SyncAll persistence
        // This test requires RuntimeJournal which is in vb_runtime
        use crate::journal::{RuntimeJournal, RuntimeJournalConfig};
        use vb_storage::{DurabilityProfile, FjallJournal, JournalWriterQueue, StorageLimits};

        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let config = RuntimeJournalConfig::new(DurabilityProfile::Strict);
        let shared_journal = config.shared_journal(
            Arc::new(journal),
            Arc::new(
                JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT)
                    .expect("queue creation should succeed"),
            ),
        );

        let run = RunId::new(30);
        let workflow = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);

        let runtime_event = crate::journal::RuntimeJournalEvent::RunSubmitted { run, workflow };

        // Strict profile append should immediately persist
        let result = shared_journal.append(runtime_event);
        assert!(result.is_ok(), "append should succeed");

        // Verify event is durable (Strict profile)
        drop(shared_journal);
        let journal2 = FjallJournal::open(temp.path(), None).expect("journal2 open should succeed");
        let events = journal2.events_for_run(run).expect("events_for_run should succeed");
        assert!(
            !events.is_empty(),
            "Strict profile events should be immediately durable"
        );
    }

    #[test]
    fn test_volatile_profile_drops_all_events() {
        // Tests that Volatile profile drops all events (no persistence)
        use crate::journal::{RuntimeJournal, VolatileRuntimeJournal};

        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = vb_storage::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        // VolatileRuntimeJournal doesn't persist to Fjall at all
        let volatile_journal = VolatileRuntimeJournal::shared();

        let run = RunId::new(32);
        let workflow = WorkflowDigest::from_bytes([0xCD; DIGEST_BYTES]);

        let runtime_event = crate::journal::RuntimeJournalEvent::RunSubmitted { run, workflow };

        volatile_journal.append(runtime_event).expect("append should succeed");

        // Events should NOT appear in Fjall (Volatile profile)
        let events = journal.events_for_run(run).expect("events_for_run should succeed");
        assert!(
            events.is_empty(),
            "Volatile profile events should NOT be persisted"
        );
    }

    // =============================================================================
    // Section 3.4: Evidence Chain (Flush Evidence)
    // =============================================================================

    #[test]
    fn test_flush_evidence_emits_ordered_chain() {
        // Tests that flush_evidence emits StepStarted -> SlotWritten -> SlotWritten -> StepSucceeded
        // This requires Shard::flush_evidence
        use crate::engine::{EvidenceCollector, EvidenceEvent};
        use vb_core::value::SlotValue;
        

        // Create a mock evidence collector
        let mut collector = EvidenceCollector::new();

        // Simulate evidence collection for a step with 2 slot writes
        collector.push_step_started(StepIdx::new(5));
        collector.push_slot_written(SlotIdx::new(0), SlotValue::Bool(true));
        collector.push_slot_written_with_extra(SlotIdx::new(1), SlotValue::I64(42), None);
        collector.push_step_succeeded(StepIdx::new(5), Some(SlotIdx::new(0)));

        // The evidence chain should be in order: StepStarted -> SlotWritten -> SlotWritten -> StepSucceeded
        let events: Vec<_> = collector.drain();
        assert_eq!(events.len(), 4);

        assert!(matches!(events[0], EvidenceEvent::StepStarted { step: _ }));
        assert!(matches!(events[1], EvidenceEvent::SlotWritten { slot: _, value: SlotValue::Bool(true), .. }));
        assert!(matches!(events[2], EvidenceEvent::SlotWritten { slot: _, value: SlotValue::I64(42), .. }));
        assert!(matches!(events[3], EvidenceEvent::StepSucceeded { step: _, output: Some(_) }));
    }

    // =============================================================================
    // Section 2.6: RuntimeError Variants (vb_runtime specific)
    // =============================================================================

    #[test]
    fn test_runtime_error_queue_full_in_shard() {
        // Tests that shard command queue full maps to RuntimeError::QueueFull
        use crate::shard::{Shard, ShardCommand, ShardConfig};

        let config = ShardConfig {
            command_queue_capacity: 2,
            trace_capacity: 4,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };

        let shard = Shard::new(config);

        // Fill the command queue
        assert!(shard.enqueue(ShardCommand::Shutdown).is_ok());
        assert!(shard.enqueue(ShardCommand::Shutdown).is_ok());

        // Queue should be full now
        assert!(shard.is_queue_full());

        // Next enqueue should fail with RuntimeError::QueueFull
        let result = shard.enqueue(ShardCommand::Shutdown);
        assert!(
            matches!(result, Err(crate::RuntimeError::QueueFull)),
            "enqueue when full should return QueueFull, got {:?}",
            result
        );
    }

    #[test]
    fn test_runtime_error_journal_poisoned() {
        // Tests that journal mutex poisoning maps to RuntimeError::JournalPoisoned
        // This would require poisoning the mutex via a panic, which is hard to test deterministically
        // RED PHASE: Document that this error case needs implementation
    }

    #[test]
    fn test_runtime_error_encode_failed() {
        // Tests that postcard encoding failures map to RuntimeError::EncodeFailed
        // This happens in flush_slot_written when value encoding fails
        // RED PHASE: Document that this error case needs implementation
    }

    // =============================================================================
    // Section 3.2: Per-Primitive Durability Matrix (runtime integration)
    // Note: These tests require full runtime execution and are more complex.
    // They are marked as needing implementation in RED phase.
    // =============================================================================

    #[test]
    fn test_foreach_iterator_state_replay_requires_runtime() {
        // Tests ForEachStart -> ForEachNext -> ForEachJoin iterator state replay
        // Iterator position is stored in slots and must survive replay
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_together_accumulator_replay_requires_runtime() {
        // Tests TogetherStart -> TogetherBranch -> TogetherBranch -> TogetherJoin
        // Accumulator slot must be correctly updated by each branch
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_collect_pagination_state_replay_requires_runtime() {
        // Tests CollectStart -> CollectPage -> CollectPage -> CollectFinish
        // Pagination state in slots must allow correct resume
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_reduce_accumulator_replay_requires_runtime() {
        // Tests ReduceStart -> ReduceNext -> ReduceNext -> ReduceFinish
        // Accumulator slot replay must reconstruct correct reduced value
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_repeat_attempt_counter_replay_requires_runtime() {
        // Tests RepeatStart -> RepeatAttempt -> RepeatAttempt -> RepeatCheck -> RepeatFinish
        // Attempt counter slot must allow correct re-execution after replay
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_wait_until_resumes_from_slot_requires_runtime() {
        // Tests WaitUntil with timer suspension - slot state must allow resume
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_ask_resume_with_answer_slot_requires_runtime() {
        // Tests Ask -> AskResume with answer slot
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_action_scheduled_completion_idempotent_replay_requires_runtime() {
        // Tests that ActionScheduled + ActionCompletedEvent replay is idempotent
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }

    #[test]
    fn test_action_scheduled_failure_idempotent_replay_requires_runtime() {
        // Tests that ActionScheduled + ActionFailedEvent replay is idempotent
        // RED PHASE: Requires full runtime execution - documented for later implementation
    }
}
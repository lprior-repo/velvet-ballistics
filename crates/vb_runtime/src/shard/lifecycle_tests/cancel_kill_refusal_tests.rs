// Bounded in-memory append refusal. Retained terminal identity is synthetic.
// This does not simulate allocation failure or exercise durable recovery.
#[cfg(not(kani))]
#[test]
fn cancel_kill_append_refusal_preserves_ticket_and_retained_identity() -> Result<(), String> {
    struct RejectCancelKill {
        accepted: crate::journal::VolatileRuntimeJournal,
        attempted: std::sync::Mutex<Option<(Vec<RuntimeJournalEvent>, vb_storage::EventSeq)>>,
    }
    impl crate::journal::RuntimeJournal for RejectCancelKill {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced_batch(&[event], vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.append_sequenced_batch(&[event], seq)
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if events.iter().any(|event| matches!(event, RuntimeJournalEvent::RunCancelled { .. } | RuntimeJournalEvent::RunKilled { .. })) {
                let mut attempted = self.attempted.lock()
                    .map_err(|_| RuntimeError::JournalPoisoned)?;
                *attempted = Some((events.to_vec(), seq));
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "test_cancel_kill_refusal",
                });
            }
            crate::journal::RuntimeJournal::append_sequenced_batch(&self.accepted, events, seq)
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    for kill in [false, true] {
        let journal = std::sync::Arc::new(RejectCancelKill {
            accepted: crate::journal::VolatileRuntimeJournal::new(),
            attempted: std::sync::Mutex::new(None),
        });
        let shared: SharedRuntimeJournal = journal.clone();
        let mut config = small_config();
        config.max_active_runs = 1;
        let mut shard = Shard::new_with_journal(config, shared)
            .map_err(|error| error.to_string())?;
        let retained = RunId::new(50_141);
        let run = RunId::new(50_142);
        shard.checked_out_run_insert(retained).map_err(|error| error.to_string())?;
        shard.terminal_runs_insert(retained).map_err(|error| error.to_string())?;
        let workflow = require_workflow("cancel-kill-refusal", suspended_workflow())?;
        submit_run(&mut shard, run, workflow, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
        let ticket = shard.pending_action_get(run).ok_or("missing issued pending ticket")?;
        assert_eq!(shard.require_pending_action_ownership(ticket), Ok(()));
        let state = shard.run_state_get(run).ok_or("missing before state")?;
        let attempt = state.run_attempt.get();
        let terminal = if kill {
            RuntimeJournalEvent::RunKilled { run, attempt }
        } else {
            RuntimeJournalEvent::RunCancelled {
                run,
                reason: Some(String::from("operator cancellation")),
                attempt,
            }
        };
        let expected_events = vec![RuntimeJournalEvent::ActionAbandoned { ticket }, terminal];
        let before = state.clone();
        let pending = shard.pending_action_clone();
        let seq = shard.journal_sequence_for(run);
        let quota = shard.charged_quotas.clone();
        let usage = shard.usage_snapshot();
        let global = shard.global_cost_usage();
        let counters = shard.counters().snapshot();
        let trace = shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity());
        let runtime = shard.runtime_state_get(run);
        let accepted = journal.accepted.snapshot().map_err(|error| error.to_string())?;
        assert!(journal.attempted.lock().map_err(|_| "attempt lock poisoned")?.is_none());

        assert_eq!(
            if kill {
                shard.handle_kill(run, None)
            } else {
                shard.handle_cancel(run, Some(String::from("operator cancellation")))
            },
            Err(RuntimeError::UnsupportedOperation {
                operation: "test_cancel_kill_refusal",
            })
        );
        assert_eq!(
            journal.attempted.lock().map_err(|_| "attempt lock poisoned")?.as_ref(),
            Some(&(expected_events, seq))
        );
        assert_eq!(journal.accepted.snapshot().map_err(|error| error.to_string())?, accepted);
        assert_eq!(shard.run_state_get(run), Some(&before));
        assert_eq!(shard.pending_action_clone(), pending);
        assert_eq!(shard.require_pending_action_ownership(ticket), Ok(()));
        assert_eq!(shard.journal_seq_get(run), Some(seq));
        assert_eq!(shard.charged_quotas, quota);
        assert_eq!(shard.usage_snapshot(), usage);
        assert_eq!(shard.global_cost_usage(), global);
        assert_eq!(shard.counters().snapshot(), counters);
        assert_eq!(
            shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity()),
            trace
        );
        assert_eq!(shard.runtime_state_get(run), runtime);
        assert!(!shard.checked_out_run_contains(run));
        assert_eq!(shard.active_run_count(), 1);
        assert!(shard.terminal_runs_contains(retained));
        assert!(!shard.terminal_runs_contains(run));
    }
    Ok(())
}

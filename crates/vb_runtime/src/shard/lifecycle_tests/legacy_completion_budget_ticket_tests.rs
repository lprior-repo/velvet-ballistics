// Synthetic journal budget forces the legacy fallback; retained identity is synthetic.
// This does not simulate allocation failure or exercise durable recovery.
#[cfg(not(kani))]
#[test]
fn legacy_completion_budget_fallback_preserves_ticket_until_commit() -> Result<(), String> {
    struct LegacyCompletionBudgetJournal {
        refuse: bool,
        accepted: crate::journal::VolatileRuntimeJournal,
        attempted: std::sync::Mutex<Option<(Vec<RuntimeJournalEvent>, vb_storage::EventSeq)>>,
    }
    impl crate::journal::RuntimeJournal for LegacyCompletionBudgetJournal {
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
            if events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::RunFailed { .. }))
            {
                let mut attempted = self
                    .attempted
                    .lock()
                    .map_err(|_| RuntimeError::JournalPoisoned)?;
                *attempted = Some((events.to_vec(), seq));
                if self.refuse {
                    return Err(RuntimeError::UnsupportedOperation {
                        operation: "test_legacy_completion_budget_refusal",
                    });
                }
            }
            crate::journal::RuntimeJournal::append_sequenced_batch(&self.accepted, events, seq)
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    for refuse in [true, false] {
        let journal = std::sync::Arc::new(LegacyCompletionBudgetJournal {
            refuse,
            accepted: crate::journal::VolatileRuntimeJournal::new(),
            attempted: std::sync::Mutex::new(None),
        });
        let shared: SharedRuntimeJournal = journal.clone();
        let mut config = small_config();
        config.max_active_runs = 1;
        let mut shard =
            Shard::new_with_journal(config, shared).map_err(|error| error.to_string())?;
        let retained = RunId::new(50_161);
        let run = RunId::new(50_162);
        shard
            .checked_out_run_insert(retained)
            .map_err(|error| error.to_string())?;
        shard
            .terminal_runs_insert(retained)
            .map_err(|error| error.to_string())?;
        let workflow = require_workflow("legacy-completion-budget-ticket", suspended_workflow())?;
        submit_run(
            &mut shard,
            run,
            workflow,
            &[(SlotIdx::ZERO, SlotValue::I64(0))],
        );
        let ticket = shard
            .pending_action_get(run)
            .ok_or("missing issued pending ticket")?;
        assert_eq!(shard.require_pending_action_ownership(ticket), Ok(()));
        {
            let state = shard.run_state_get_mut(run).ok_or("missing active run")?;
            state.admission = Some(crate::admission::RunAdmission::with_budget(
                state.workflow.digest(),
                run,
                CapabilitySet::empty(),
                vb_core::RuntimePolicy::Relaxed,
                journal_bytes_budget(1),
            ));
            state.journal_bytes_written = 0;
        }
        let state = shard.run_state_get(run).ok_or("missing before state")?;
        assert!(
            shard
                .command_journal_bytes_exceeds(
                    state,
                    &[RuntimeJournalEvent::StepSucceeded {
                        run,
                        step: ticket.step,
                        output: SlotIdx::ZERO,
                        attempt: ticket.attempt,
                    },]
                )
                .map_err(|error| error.to_string())?
        );
        let attempt = state.run_attempt.get();
        let expected_events = vec![RuntimeJournalEvent::RunFailed { run, attempt }];
        let before = state.clone();
        let pending = shard.pending_action_clone();
        let seq = shard.journal_sequence_for(run);
        let quota = shard.charged_quotas.clone();
        let usage = shard.usage_snapshot();
        let global = shard.global_cost_usage();
        let counters = shard.counters().snapshot();
        let trace = shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity());
        let runtime = shard.runtime_state_get(run);
        let accepted = journal
            .accepted
            .snapshot()
            .map_err(|error| error.to_string())?;
        assert!(
            journal
                .attempted
                .lock()
                .map_err(|_| "attempt lock poisoned")?
                .is_none()
        );

        let result = shard.handle_legacy_action_completion(run, ticket.step);
        if !refuse {
            assert_eq!(result, Ok(()));
            assert_eq!(shard.pending_action_get(run), None);
            assert!(!shard.run_state_contains(run));
            assert!(!shard.checked_out_run_contains(run));
            assert!(shard.terminal_runs_contains(run));
            assert_eq!(shard.runtime_state_get(run), None);
            assert!(!shard.charged_quotas.contains_key(&run));
            assert_eq!(shard.journal_seq_get(run), None);
            assert_eq!(
                journal
                    .attempted
                    .lock()
                    .map_err(|_| "attempt lock poisoned")?
                    .as_ref(),
                Some(&(expected_events, seq))
            );
            continue;
        }
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "test_legacy_completion_budget_refusal",
            })
        );
        assert_eq!(
            journal
                .attempted
                .lock()
                .map_err(|_| "attempt lock poisoned")?
                .as_ref(),
            Some(&(expected_events, seq))
        );
        assert_eq!(
            journal
                .accepted
                .snapshot()
                .map_err(|error| error.to_string())?,
            accepted
        );
        assert_eq!(shard.run_state_get(run), Some(&before));
        assert_eq!(shard.pending_action_clone(), pending);
        assert_eq!(shard.require_pending_action_ownership(ticket), Ok(()));
        assert_eq!(shard.journal_seq_get(run), Some(seq));
        assert_eq!(shard.charged_quotas, quota);
        assert_eq!(shard.usage_snapshot(), usage);
        assert_eq!(shard.global_cost_usage(), global);
        assert_eq!(shard.counters().snapshot(), counters);
        assert_eq!(
            shard
                .trace_ring()
                .snapshot_for_run(run, shard.trace_ring().capacity()),
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

#[test]
fn future_attempt_completion_rejected_when_current_attempt_exists() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(40_001);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(state) = shard.run_state_get_mut(run) else {
        assert_eq!(None::<()>, Some(()), "run should remain active");
        return;
    };
    assert_eq!(state.action_attempts.get(0).copied(), Some(1));
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 3,
                ..make_ticket(run, StepIdx::ZERO, 2)
            },
            output,
        }),
        Ok(())
    );
    // G005 fixed: future-attempt completion must be rejected
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn future_attempt_completion_mismatch_is_action_failed_code() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(40_002);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    let error = RuntimeError::InvalidActionCompletion;
    assert_eq!(error.runtime_code(), Some("ACTION_FAILED"));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 3,
                ..make_ticket(run, StepIdx::ZERO, 4)
            },
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(error));
}

#[test]
fn stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged() {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(41);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(state) = shard.run_state_get_mut(run) else {
        assert_eq!(None::<()>, Some(()), "run should remain active");
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let frame_before = state.frame.clone();
    let step_state_before = state.frame.step_state(StepIdx::ZERO);
    let attempts_before = state.action_attempts.clone();
    let counters_before = shard.counters().snapshot();
    let journal_before = journal.snapshot();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: ActionTicket {
                capacity: 3,
                ..make_ticket(run, StepIdx::ZERO, 2)
            },
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    let Some(state_after) = shard.run_state_get_mut(run) else {
        assert_eq!(
            None::<()>,
            Some(()),
            "run should remain active after rejection"
        );
        return;
    };
    assert_eq!(state_after.frame.pc(), frame_before.pc());
    assert_eq!(
        state_after.frame.step_state(StepIdx::ZERO),
        step_state_before
    );
    assert_eq!(state_after.frame, frame_before);
    assert_eq!(state_after.action_attempts, attempts_before);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(journal.snapshot(), journal_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before
    );
}

#[test]
fn scheduling_propagates_zero_retry_policy_error() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = zero_retry_policy_workflow() else {
        assert_eq!(
            None::<()>,
            Some(()),
            "missing zero retry policy workflow fixture"
        );
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run: RunId::new(42),
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(
                vb_core::ActionId::new(0)
            ),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(
                vb_core::ActionId::new(0)
            )
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_zero",
        })
    );
}

#[test]
fn legacy_action_completed_on_suspended_run_succeeds() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let found = shard.trace_ring_mut().drain().iter().any(|e| {
        *e == TraceEvent::ActionCompleted {
            run,
            step: StepIdx::ZERO,
        }
    });
    assert_eq!(found, true);
}

#[test]
fn legacy_action_completed_unknown_run_returns_run_not_found() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: RunId::new(9999),
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn legacy_action_completion_journal_first() -> Result<(), String> {
    // Regression test for RS-010:
    // handle_legacy_action_completion must append the journal event BEFORE mutating
    // the run frame. On journal append failure, the frame MUST remain unchanged.
    struct LegacyStepFailsJournal;
    impl crate::journal::RuntimeJournal for LegacyStepFailsJournal {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }

        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            // Reject the legacy completion's StepSucceeded append with a typed
            // JournalError so the shard surfaces a StorageJournalAppend failure.
            if matches!(event, RuntimeJournalEvent::StepSucceeded { .. }) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::StepSucceeded { .. }))
            {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                });
            }
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    let shared: SharedRuntimeJournal = std::sync::Arc::new(LegacyStepFailsJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let wf = require_workflow("suspended_workflow", suspended_workflow())?;
    let run = RunId::new(50_010);
    assert_eq!(
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow: wf,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0
            )),
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))])
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Capture frame invariants BEFORE the failing legacy completion.
    let frame_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .frame
        .clone();
    let pc_before = frame_before.pc();
    let step_state_before = frame_before.step_state(StepIdx::ZERO);
    let executed_before = frame_before.executed();
    let counters_before = shard.counters().snapshot();
    // Pre-existing journal sequence value (if any) for the run.
    let seq_before = shard.journal_sequences.get(&run).copied();

    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run,
            step: StepIdx::ZERO,
        }),
        Ok(())
    );
    // The typed StorageJournalAppend error MUST surface (no swallowed map_err(|_| ...)).
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );

    // Frame MUST remain unchanged — the journal-first ordering guarantees that
    // a failed append does not diverge the frame from the journal.
    let frame_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected legacy completion")
        .frame
        .clone();
    assert_eq!(frame_after.pc(), pc_before, "pc must be unchanged");
    assert_eq!(
        frame_after.step_state(StepIdx::ZERO),
        step_state_before,
        "step state must be unchanged"
    );
    assert_eq!(
        frame_after.executed(),
        executed_before,
        "executed count must be unchanged"
    );
    assert_eq!(frame_after, frame_before, "frame must be byte-equal");
    // Counters and journal sequence MUST remain unchanged.
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    Ok(())
}

#[test]
fn handle_ask_answer_batch_append_failures_leave_no_partial_rows() -> Result<(), String> {
    struct AskAnswerBatchFailsOnce {
        run: RunId,
        fail_index: usize,
        failed: std::sync::atomic::AtomicBool,
        events: std::sync::Mutex<Vec<RuntimeJournalEvent>>,
    }

    impl AskAnswerBatchFailsOnce {
        fn snapshot(&self) -> Result<Vec<RuntimeJournalEvent>, String> {
            self.events
                .lock()
                .map_err(|_| String::from("test journal mutex poisoned"))
                .map(|events| events.clone())
        }

        fn is_target_answer_event(&self, index: usize, event: &RuntimeJournalEvent) -> bool {
            match index {
                0 => self.is_target_slot_written(event),
                1 => self.is_target_ask_answered(event),
                2 => self.is_target_step_succeeded(event),
                _ => false,
            }
        }

        fn is_target_slot_written(&self, event: &RuntimeJournalEvent) -> bool {
            matches!(
                event,
                RuntimeJournalEvent::SlotWritten { run, slot, .. }
                    if *run == self.run && *slot == SlotIdx::new(2)
            )
        }

        fn is_target_ask_answered(&self, event: &RuntimeJournalEvent) -> bool {
            matches!(
                event,
                RuntimeJournalEvent::AskAnswered { run, step, slot }
                    if *run == self.run && *step == StepIdx::new(2) && *slot == SlotIdx::new(2)
            )
        }

        fn is_target_step_succeeded(&self, event: &RuntimeJournalEvent) -> bool {
            matches!(
                event,
                RuntimeJournalEvent::StepSucceeded {
                    run, step, output, ..
                } if *run == self.run && *step == StepIdx::new(2) && *output == SlotIdx::new(2)
            )
        }
    }

    impl crate::journal::RuntimeJournal for AskAnswerBatchFailsOnce {
        fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
            Some(vb_storage::DurabilityProfile::Volatile)
        }

        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }

        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .push(event);
            Ok(())
        }

        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _start_seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            for (index, event) in events.iter().enumerate() {
                if self.fail_index == index
                    && self.is_target_answer_event(index, event)
                    && !self.failed.swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    return Err(RuntimeError::StorageJournalAppend {
                        source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                    });
                }
            }
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .extend(events.iter().cloned());
            Ok(())
        }

        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    for (fail_index, run) in [RunId::new(50_041), RunId::new(50_042), RunId::new(50_043)]
        .into_iter()
        .enumerate()
    {
        let journal = std::sync::Arc::new(AskAnswerBatchFailsOnce {
            run,
            fail_index,
            failed: std::sync::atomic::AtomicBool::new(false),
            events: std::sync::Mutex::new(Vec::new()),
        });
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        submit_run(
            &mut shard,
            run,
            require_workflow("ask", ask_workflow())?,
            &[],
        );
        let ticket = shard
            .lookup_pending_ask_ticket(run, StepIdx::new(2))
            .ok_or("suspended ask must issue a ticket")?;
        let answer = AskAnswer {
            ticket,
            answer_slot: SlotIdx::new(2),
            value: SlotValue::I64(77),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        let frame_before = shard
            .run_state_get(run)
            .ok_or("run should remain active after ask suspension")?
            .frame
            .clone();
        let timers_before = shard.pending_timer_clone();
        let counters_before = shard.counters().snapshot();
        let seq_before = shard.journal_sequences.get(&run).copied();
        let journal_before = journal.snapshot()?;
        let trace_before = shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity());

        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        let result = shard.tick();
        assert!(
            matches!(
                &result,
                Err(RuntimeError::StorageJournalAppend { source })
                    if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
            ),
            "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
        );
        let frame_after = shard
            .run_state_get(run)
            .ok_or("run must remain active after rejected ask answer")?
            .frame
            .clone();
        assert_eq!(frame_after, frame_before, "frame must be unchanged");
        assert_eq!(shard.pending_timer_clone(), timers_before);
        assert_eq!(shard.counters().snapshot(), counters_before);
        assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
        assert_eq!(
            journal.snapshot()?,
            journal_before,
            "failed batch index {fail_index} must not append partial answer rows"
        );
        assert_eq!(
            shard
                .trace_ring()
                .snapshot_for_run(run, shard.trace_ring().capacity()),
            trace_before,
            "trace ring must be unchanged"
        );

        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 0);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        let journal_after_retry = journal.snapshot()?;
        assert_eq!(
            journal_after_retry
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        RuntimeJournalEvent::SlotWritten { run: event_run, slot, .. }
                            if *event_run == run && *slot == SlotIdx::new(2)
                    )
                })
                .count(),
            1,
            "retry must append exactly one SlotWritten answer row"
        );
        assert_eq!(
            journal_after_retry
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        RuntimeJournalEvent::AskAnswered { run: event_run, step, slot }
                            if *event_run == run
                                && *step == StepIdx::new(2)
                                && *slot == SlotIdx::new(2)
                    )
                })
                .count(),
            1,
            "retry must append exactly one AskAnswered row"
        );
        assert_eq!(
            journal_after_retry
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        RuntimeJournalEvent::StepSucceeded { run: event_run, step, output, .. }
                            if *event_run == run
                                && *step == StepIdx::new(2)
                                && *output == SlotIdx::new(2)
                    )
                })
                .count(),
            1,
            "retry must append exactly one StepSucceeded answer row"
        );
    }
    Ok(())
}


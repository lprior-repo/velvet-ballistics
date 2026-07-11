#[test]
fn future_attempt_completion_rejected_when_current_attempt_exists() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(40_001);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
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
fn future_attempt_completion_beyond_max_is_action_failed_code() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
        return;
    };
    let run = RunId::new(40_002);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
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
    let error = RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 };
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
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
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
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: 2,
            current: 3,
        })
    );
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
        shard.enqueue(ShardCommand::Submit {
            run: RunId::new(42),
            workflow: wf,
            caps: CapabilitySet::empty(),
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
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
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
fn legacy_action_completion_journal_first() {
    // Regression test for RS-010:
    // handle_legacy_action_completion must append the journal event BEFORE mutating
    // the run frame. On journal append failure, the frame MUST remain unchanged.
    struct LegacyStepFailsJournal;
    impl crate::journal::RuntimeJournal for LegacyStepFailsJournal {
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
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50_010);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
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
                    && !self
                        .failed
                        .swap(true, std::sync::atomic::Ordering::SeqCst)
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
        submit_run(&mut shard, run, require_workflow("ask", ask_workflow())?);
        let answer = AskAnswer {
            ticket: AskTicket {
                run,
                ask_step: StepIdx::new(2),
                resume_step: StepIdx::new(3),
            },
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

#[test]
fn handle_action_completion_journal_first() {
    // Regression test for RS-010 (non-legacy path):
    // handle_action_completion must append the ActionCompletedEnvelope journal
    // event BEFORE mutating the run frame. On journal append failure, the
    // frame, action_attempts, counters, and journal sequence MUST remain
    // unchanged so a failed append does not diverge memory-only state from
    // durable evidence.
    struct EnvelopeFailsJournal;
    impl crate::journal::RuntimeJournal for EnvelopeFailsJournal {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            // Reject the non-legacy completion's ActionCompletedEnvelope append
            // with a typed JournalError so the shard surfaces a
            // StorageJournalAppend failure.
            if matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. }) {
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
            if events.iter().any(|event| {
                matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. })
            }) {
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

    let shared: SharedRuntimeJournal = std::sync::Arc::new(EnvelopeFailsJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50_020);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Capture frame invariants BEFORE the failing completion.
    let frame_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .frame
        .clone();
    let pc_before = frame_before.pc();
    let step_state_before = frame_before.step_state(StepIdx::ZERO);
    let executed_before = frame_before.executed();
    let attempts_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .action_attempts
        .clone();
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
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
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    // The typed StorageJournalAppend error MUST surface (no swallowed map_err).
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );

    // Frame MUST remain unchanged — the journal-first ordering guarantees
    // that a failed append does not diverge the frame from the journal.
    let frame_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected completion")
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
    // action_attempts, counters, journal sequence, and trace MUST remain
    // unchanged.
    let attempts_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected completion")
        .action_attempts
        .clone();
    assert_eq!(attempts_after, attempts_before, "action_attempts unchanged");
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );
}

#[test]
fn handle_action_failure_journal_first() {
    // Regression test for RS-010 (failure path):
    // handle_action_failure must append the ActionFailed journal event BEFORE
    // mutating the run frame. On journal append failure, the frame,
    // action_attempts, counters, and journal sequence MUST remain unchanged
    // so a failed append does not diverge memory-only state from durable
    // evidence. The retry-policy and error-handler decisions in
    // apply_action_failure_to_state are part of the same critical section and
    // must not be persisted to memory without durable evidence.
    struct ActionFailedFailsJournal;
    impl crate::journal::RuntimeJournal for ActionFailedFailsJournal {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            // Reject the failure path's ActionFailed append with a typed
            // JournalError so the shard surfaces a StorageJournalAppend
            // failure.
            if matches!(event, RuntimeJournalEvent::ActionFailed { .. }) {
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
                .any(|event| matches!(event, RuntimeJournalEvent::ActionFailed { .. }))
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

    let shared: SharedRuntimeJournal = std::sync::Arc::new(ActionFailedFailsJournal);
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50_030);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Capture frame invariants BEFORE the failing failure-path.
    let frame_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .frame
        .clone();
    let pc_before = frame_before.pc();
    let step_state_before = frame_before.step_state(StepIdx::ZERO);
    let executed_before = frame_before.executed();
    let attempts_before = shard
        .run_state_get(run)
        .expect("run should remain active after submit")
        .action_attempts
        .clone();
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    // The typed StorageJournalAppend error MUST surface (no swallowed map_err).
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );

    // Frame MUST remain unchanged — the journal-first ordering guarantees
    // that a failed append does not diverge the frame from the journal.
    let frame_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected failure")
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
    // action_attempts, counters, journal sequence, and trace MUST remain
    // unchanged. The trace ring and pending_action state are part of the
    // same critical section as the frame mutation.
    let attempts_after = shard
        .run_state_get(run)
        .expect("run must remain active after rejected failure")
        .action_attempts
        .clone();
    assert_eq!(attempts_after, attempts_before, "action_attempts unchanged");
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );
}

#[test]
fn handle_action_failure_append_failure_keeps_retry_state_and_pending_action() -> Result<(), String>
{
    struct ActionFailedFailsOnce {
        failed: std::sync::atomic::AtomicBool,
    }
    impl crate::journal::RuntimeJournal for ActionFailedFailsOnce {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            let should_fail = matches!(
                event,
                RuntimeJournalEvent::ActionFailed {
                    run: event_run,
                    step: event_step,
                    ..
                } if event_run == RunId::new(50_050) && event_step == StepIdx::new(1)
            );
            if should_fail
                && !self
                    .failed
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
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
            let should_fail = events.iter().any(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::ActionFailed {
                        run: event_run,
                        step: event_step,
                        ..
                    } if *event_run == RunId::new(50_050) && *event_step == StepIdx::new(1)
                )
            });
            if should_fail
                && !self
                    .failed
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
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

    let shared: SharedRuntimeJournal = std::sync::Arc::new(ActionFailedFailsOnce {
        failed: std::sync::atomic::AtomicBool::new(false),
    });
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_050);
    submit_run(&mut shard, run, retry_workflow()?);
    let ticket = make_ticket(run, StepIdx::new(1), 1);
    let frame_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after retry workflow suspension")?
        .frame
        .clone();
    let attempts_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after retry workflow suspension")?
        .action_attempts
        .clone();
    let pending_actions_before = shard.pending_action_clone();
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::WriteLockPoisoned)
        ),
        "expected typed StorageJournalAppend(WriteLockPoisoned), got {result:?}"
    );
    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected action failure")?;
    assert_eq!(state_after.frame, frame_before, "frame must be unchanged");
    assert_eq!(
        state_after.action_attempts, attempts_before,
        "retry attempts must be unchanged"
    );
    assert_eq!(shard.pending_action_clone(), pending_actions_before);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let retried_state = shard
        .run_state_get(run)
        .ok_or("run should remain active after retry is scheduled")?;
    assert_eq!(retried_state.action_attempts.get(1).copied(), Some(2));
    assert_eq!(shard.pending_action_get(run).map(|pending| pending.attempt), Some(2));
    Ok(())
}

#[test]
fn fail_run_action_failure_queue_full_keeps_state_and_allows_retry() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 8;
    const QUEUE_BATCH_SIZE: usize = 8;
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = tempfile::Builder::new()
        .prefix("vb-runtime-fail-run-batch-")
        .tempdir_in(base)
        .map_err(|error| error.to_string())?;
    let journal = std::sync::Arc::new(
        vb_storage::FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?,
    );
    let queue = std::sync::Arc::new(
        vb_storage::JournalWriterQueue::new(
            QUEUE_CAPACITY,
            QUEUE_BATCH_SIZE,
            vb_storage::StorageLimits::DEFAULT,
        )
        .map_err(|error| error.to_string())?,
    );
    let adapter = std::sync::Arc::new(crate::journal::QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let shared: SharedRuntimeJournal = adapter.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_060);
    submit_run(
        &mut shard,
        run,
        require_workflow("suspended", suspended_workflow())?,
    );

    let target_pending = QUEUE_CAPACITY
        .checked_sub(1)
        .ok_or("queue capacity must leave one slot")?;
    let counts_before_fill = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    if counts_before_fill.journaled > target_pending {
        return Err(format!(
            "submit queued {} journaled rows, exceeding target {target_pending}",
            counts_before_fill.journaled
        ));
    }
    let filler_needed = target_pending.saturating_sub(counts_before_fill.journaled);
    let filler_runs = [
        RunId::new(50_061),
        RunId::new(50_062),
        RunId::new(50_063),
        RunId::new(50_064),
        RunId::new(50_065),
        RunId::new(50_066),
        RunId::new(50_067),
        RunId::new(50_068),
    ];
    for filler_run in filler_runs.iter().copied().take(filler_needed) {
        queue
            .enqueue_journaled(vb_storage::JournalEvent::RunCancelled {
                run: filler_run,
                seq: vb_storage::EventSeq::ZERO,
                attempt: 1,
                reason: None,
            })
            .map_err(|error| error.to_string())?;
    }
    let filled_counts = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    assert_eq!(filled_counts.journaled, target_pending);
    assert_eq!(filled_counts.strict, 0);

    let ticket = make_ticket(run, StepIdx::ZERO, 1);
    let frame_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after action suspension")?
        .frame
        .clone();
    let attempts_before = shard
        .run_state_get(run)
        .ok_or("run should remain active after action suspension")?
        .action_attempts
        .clone();
    let pending_actions_before = shard.pending_action_clone();
    let counters_before = shard.counters().snapshot();
    let runtime_state_before = shard.runtime_state_get(run);
    let terminal_before = shard.terminal_runs_contains(run);
    let active_before = shard.active_run_count();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let durable_before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected typed StorageJournalAppend(QueueFull), got {result:?}"
    );

    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected fail-run batch")?;
    assert_eq!(state_after.frame, frame_before, "frame must be unchanged");
    assert_eq!(state_after.action_attempts, attempts_before);
    assert_eq!(shard.pending_action_clone(), pending_actions_before);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.runtime_state_get(run), runtime_state_before);
    assert_eq!(shard.terminal_runs_contains(run), terminal_before);
    assert_eq!(shard.active_run_count(), active_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), seq_before);
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        trace_before,
        "trace ring must be unchanged"
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?,
        filled_counts,
        "failed atomic batch must not consume the one remaining queue slot"
    );
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        durable_before
    );

    let flushed = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(flushed.drained, target_pending);
    assert_eq!(flushed.written, target_pending);
    let durable_after_rejected_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_rejected_flush.iter().any(|event| matches!(
            event,
            vb_storage::JournalEvent::ActionFailedEvent { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
        )),
        false,
        "rejected batch must leave no ActionFailed or RunFailed durable rows"
    );

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.terminal_runs_contains(run), true);
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.counters().snapshot().runs_failed, 1);
    let retry_counts = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    assert_eq!(retry_counts.journaled, 2);
    assert_eq!(retry_counts.strict, 0);
    let retry_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(retry_flush.drained, 2);
    assert_eq!(retry_flush.written, 2);
    let durable_after_retry = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_retry
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_retry
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        1
    );
    Ok(())
}

fn queued_journal_fixture(
    prefix: &str,
    capacity: usize,
    batch_size: usize,
) -> Result<
    (
        std::sync::Arc<vb_storage::FjallJournal>,
        std::sync::Arc<vb_storage::JournalWriterQueue>,
        std::sync::Arc<crate::journal::QueuedStorageRuntimeJournal>,
        SharedRuntimeJournal,
    ),
    String,
> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(base)
        .map_err(|error| error.to_string())?;
    let journal = std::sync::Arc::new(
        vb_storage::FjallJournal::open(dir.keep(), None).map_err(|error| error.to_string())?,
    );
    let queue = std::sync::Arc::new(
        vb_storage::JournalWriterQueue::new(
            capacity,
            batch_size,
            vb_storage::StorageLimits::DEFAULT,
        )
        .map_err(|error| error.to_string())?,
    );
    let adapter = std::sync::Arc::new(crate::journal::QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let shared: SharedRuntimeJournal = adapter.clone();
    Ok((journal, queue, adapter, shared))
}

fn fill_queue_to_one_free_slot(
    queue: &vb_storage::JournalWriterQueue,
    capacity: usize,
    first_filler_run: u64,
) -> Result<usize, String> {
    let target_pending = capacity
        .checked_sub(1)
        .ok_or("queue capacity must leave one slot")?;
    let counts_before_fill = queue
        .pending_profile_counts()
        .map_err(|error| error.to_string())?;
    if counts_before_fill.journaled > target_pending {
        return Err(format!(
            "submit queued {} journaled rows, exceeding target {target_pending}",
            counts_before_fill.journaled
        ));
    }
    let filler_needed = target_pending.saturating_sub(counts_before_fill.journaled);
    for offset in 0..filler_needed {
        let offset = u64::try_from(offset).map_err(|error| error.to_string())?;
        let run_number = first_filler_run
            .checked_add(offset)
            .ok_or("filler run id overflow")?;
        queue
            .enqueue_journaled(vb_storage::JournalEvent::RunCancelled {
                run: RunId::new(run_number),
                seq: vb_storage::EventSeq::ZERO,
                attempt: 1,
                reason: None,
            })
            .map_err(|error| error.to_string())?;
    }
    queue
        .pending_profile_counts()
        .map(|counts| counts.journaled)
        .map_err(|error| error.to_string())
}

fn next_journal_seq(seq: Option<vb_storage::EventSeq>) -> Result<Option<vb_storage::EventSeq>, String> {
    match seq {
        Some(value) => value
            .get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
            .map(Some)
            .ok_or_else(|| "journal sequence overflow".to_owned()),
        None => Ok(Some(vb_storage::EventSeq::new(1))),
    }
}

#[test]
fn submit_drive_queue_full_after_admission_leaves_resumable_retry() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 4;
    const QUEUE_BATCH_SIZE: usize = 4;
    let (journal, queue, adapter, shared) = queued_journal_fixture(
        "vb-runtime-submit-drive-rollback-",
        QUEUE_CAPACITY,
        QUEUE_BATCH_SIZE,
    )?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_100);
    let workflow = require_workflow("suspended", suspended_workflow())?;

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected first drive batch to fail closed on QueueFull, got {result:?}"
    );

    assert_eq!(shard.active_run_count(), 1);
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.terminal_runs_contains(run), false);
    assert_eq!(shard.journal_sequences.get(&run).copied(), Some(vb_storage::EventSeq::new(2)));
    assert_eq!(
        shard.counters().snapshot(),
        crate::counters::CounterSnapshot {
            runs_submitted: 1,
            runs_completed: 0,
            runs_failed: 0,
            steps_executed: 0,
        }
    );
    assert_eq!(
        shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity()),
        vec![TraceEvent::RunSubmitted { run }]
    );
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        2
    );
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));

    let header_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(header_flush.drained, 2);
    assert_eq!(header_flush.written, 2);
    let durable_header = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        durable_header.as_slice(),
        [
            vb_storage::JournalEvent::RunAccepted { seq, .. },
            vb_storage::JournalEvent::RunAdmission { seq: admission_seq, .. },
        ] if *seq == vb_storage::EventSeq::new(0)
            && *admission_seq == vb_storage::EventSeq::new(1)
    ));
    assert_eq!(
        durable_header.iter().any(|event| matches!(
            event,
            vb_storage::JournalEvent::StepStarted { .. }
                | vb_storage::JournalEvent::ActionScheduledTicket { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
        )),
        false
    );

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.runtime_state_get(run),
        Some(super::super::RuntimeState::Resumable)
    );
    assert_eq!(shard.pending_action_get(run).map(|ticket| ticket.attempt), Some(1));
    assert_eq!(
        shard.journal_sequences.get(&run).copied(),
        Some(vb_storage::EventSeq::new(6))
    );

    let resume_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(resume_flush.drained, 4);
    assert_eq!(resume_flush.written, 4);
    let durable_after_resume = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_resume
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunResumed { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_resume
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    assert_eq!(
        durable_after_resume
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn retry_now_action_failure_queue_full_preserves_post_failure_state() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 16;
    const QUEUE_BATCH_SIZE: usize = 16;
    let (journal, queue, adapter, shared) = queued_journal_fixture(
        "vb-runtime-retry-post-failure-",
        QUEUE_CAPACITY,
        QUEUE_BATCH_SIZE,
    )?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_070);
    submit_run(&mut shard, run, retry_workflow()?);
    let filled_counts = fill_queue_to_one_free_slot(&queue, QUEUE_CAPACITY, 50_071)?;
    assert_eq!(filled_counts, QUEUE_CAPACITY - 1);

    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let counters_before = shard.counters().snapshot();
    let active_before = shard.active_run_count();
    let durable_before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    let ticket = make_ticket(run, StepIdx::new(1), 1);

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: retryable_failure(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected QueueFull after accepted ActionFailed, got {result:?}"
    );

    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected retry drive")?;
    assert_eq!(state_after.action_attempts.get(1).copied(), Some(2));
    assert_eq!(state_after.frame.pc(), StepIdx::new(1));
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.runtime_state_get(run), Some(super::super::RuntimeState::Resumable));
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.active_run_count(), active_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), next_journal_seq(seq_before)?);
    let trace_after = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    assert_eq!(trace_after.len(), trace_before.len() + 1);
    assert!(trace_after.iter().any(|event| {
        matches!(event, TraceEvent::ActionFailed { run: event_run, step, .. }
            if *event_run == run && *step == StepIdx::new(1))
    }));
    assert_eq!(
        queue
            .pending_profile_counts()
            .map_err(|error| error.to_string())?
            .journaled,
        QUEUE_CAPACITY
    );
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        durable_before
    );

    let rejected_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(rejected_flush.drained, QUEUE_CAPACITY);
    assert_eq!(rejected_flush.written, QUEUE_CAPACITY);
    let durable_after_rejected_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| {
                matches!(event, vb_storage::JournalEvent::StepStarted { run: event_run, step, .. }
                    if *event_run == run && *step == StepIdx::new(1))
            })
            .count(),
        1,
        "failed retry drive must not append a second StepStarted for the retried action"
    );
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| {
                matches!(event, vb_storage::JournalEvent::ActionScheduledTicket { run: event_run, .. }
                    if *event_run == run)
            })
            .count(),
        1,
        "failed retry drive must not append the attempt-2 ActionScheduledTicket"
    );

    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_action_get(run).map(|pending| pending.attempt), Some(2));
    assert_eq!(
        shard
            .run_state_get(run)
            .and_then(|state| state.action_attempts.get(1).copied()),
        Some(2)
    );
    let retry_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(retry_flush.written, retry_flush.drained);
    Ok(())
}

#[test]
fn drive_handler_action_failure_queue_full_preserves_post_failure_state() -> Result<(), String> {
    const QUEUE_CAPACITY: usize = 16;
    const QUEUE_BATCH_SIZE: usize = 16;
    let (journal, queue, adapter, shared) = queued_journal_fixture(
        "vb-runtime-handler-post-failure-",
        QUEUE_CAPACITY,
        QUEUE_BATCH_SIZE,
    )?;
    let mut shard = Shard::new_with_journal(small_config(), shared);
    let run = RunId::new(50_080);
    submit_run(
        &mut shard,
        run,
        require_workflow("error_handler", error_handler_workflow())?,
    );
    let filled_counts = fill_queue_to_one_free_slot(&queue, QUEUE_CAPACITY, 50_081)?;
    assert_eq!(filled_counts, QUEUE_CAPACITY - 1);
    let counters_before = shard.counters().snapshot();
    let seq_before = shard.journal_sequences.get(&run).copied();
    let trace_before = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    let durable_before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    let ticket = make_ticket(run, StepIdx::new(1), 1);

    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket,
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    let result = shard.tick();
    assert!(
        matches!(
            &result,
            Err(RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ),
        "expected QueueFull after accepted handler ActionFailed, got {result:?}"
    );

    let state_after = shard
        .run_state_get(run)
        .ok_or("run must remain active after rejected handler drive")?;
    assert_eq!(state_after.frame.pc(), StepIdx::new(2));
    assert_eq!(shard.pending_action_get(run), None);
    assert_eq!(shard.terminal_runs_contains(run), false);
    assert_eq!(shard.counters().snapshot(), counters_before);
    assert_eq!(shard.journal_sequences.get(&run).copied(), next_journal_seq(seq_before)?);
    let trace_after = shard
        .trace_ring()
        .snapshot_for_run(run, shard.trace_ring().capacity());
    assert_eq!(trace_after.len(), trace_before.len() + 1);
    assert!(trace_after.iter().any(|event| {
        matches!(event, TraceEvent::ActionFailed { run: event_run, step, .. }
            if *event_run == run && *step == StepIdx::new(1))
    }));
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?,
        durable_before
    );

    let rejected_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(rejected_flush.drained, QUEUE_CAPACITY);
    assert_eq!(rejected_flush.written, QUEUE_CAPACITY);
    let durable_after_rejected_flush = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        durable_after_rejected_flush
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::ActionFailedEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        durable_after_rejected_flush.iter().any(|event| {
            matches!(event, vb_storage::JournalEvent::StepStarted { run: event_run, step, .. }
                if *event_run == run && *step == StepIdx::new(2))
        }),
        false,
        "failed handler drive must not append handler StepStarted"
    );
    assert_eq!(
        durable_after_rejected_flush.iter().any(|event| {
            matches!(event, vb_storage::JournalEvent::RunFinished { run: event_run, .. }
                if *event_run == run)
        }),
        false,
        "failed handler drive must not append RunFinished"
    );

    assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.terminal_runs_contains(run), true);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    let retry_flush = adapter.flush_batch().map_err(|error| format!("{error:?}"))?;
    assert_eq!(retry_flush.written, retry_flush.drained);
    Ok(())
}

#[test]
fn finish_run_append_failure_reports_rollback_failure() -> Result<(), String> {
    struct RejectRunFinishedJournal;
    impl crate::journal::RuntimeJournal for RejectRunFinishedJournal {
        fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            self.append_sequenced(event, vb_storage::EventSeq::ZERO)
        }
        fn append_sequenced(
            &self,
            event: RuntimeJournalEvent,
            _seq: vb_storage::EventSeq,
        ) -> crate::RuntimeResult<()> {
            if matches!(event, RuntimeJournalEvent::RunFinished { .. }) {
                return Err(RuntimeError::StorageJournalAppend {
                    source: std::sync::Arc::new(vb_storage::JournalError::QueueFull),
                });
            }
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    let shared: SharedRuntimeJournal = std::sync::Arc::new(RejectRunFinishedJournal);
    let mut config = small_config();
    config.max_active_runs = 0;
    let mut shard = Shard::new_with_journal(config, shared);
    let run = RunId::new(50_090);
    let workflow = require_workflow("finished", finished_workflow())?;
    let state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or("run state fixture must build")?;
    let result = shard.finish_run(run, state);

    match result {
        Err(RuntimeError::RollbackFailed {
            operation,
            primary,
            rollback,
        }) => {
            assert_eq!(operation, "finish_run");
            assert!(matches!(
                primary.as_ref(),
                RuntimeError::StorageJournalAppend { source }
                    if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
            ));
            assert!(matches!(
                rollback.as_ref(),
                RuntimeError::ActiveRunCapacityExceeded { capacity: 0 }
            ));
            Ok(())
        }
        other => Err(format!("expected rollback-combined error, got {other:?}")),
    }
}

// =======================================================================
// handle_action_completion terminal run fence
// =======================================================================

#[test]
fn handle_action_completion_returns_run_not_found_when_run_missing() {
    let mut shard = Shard::new(small_config());
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(RunId::new(420), StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_completion_returns_run_not_found_when_run_cancelled() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(421);
    submit_run(&mut shard, run, wf);
    // Cancel the run
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Now attempt action completion on cancelled run
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_completion_returns_run_not_found_when_run_finished() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(422);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Finished workflow completes immediately — run should no longer be active
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            output,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

// =======================================================================
// handle_action_failure
// =======================================================================

#[test]
fn handle_action_failure_returns_run_not_found_when_run_missing() {
    let mut shard = Shard::new(small_config());
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(RunId::new(430), StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn handle_action_failure_returns_stale_attempt_when_attempt_mismatch() {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(431);
    submit_run(&mut shard, run, wf);
    // Set current attempt to 3
    let Some(state) = shard.run_state_get_mut(run) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    // Send attempt=1 (stale) failure
    assert_eq!(
        shard.enqueue(ShardCommand::ActionFailed {
            ticket: make_ticket(run, StepIdx::ZERO, 1),
            failure: non_retryable_failure(),
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 3,
        })
    );
}

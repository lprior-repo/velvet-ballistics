
    #[test]
    fn cancel_clears_pending_timer() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = wait_workflow() else {
            return;
        };
        let run = RunId::new(91);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 0);
        Ok(())
    }

    #[test]
    fn inspect_active_run_returns_found() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(100);
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
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 42,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snap)) => {
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 42);
            }
            other => {
                assert_eq!(
                    format!("{other:?}"),
                    "Some(Found(InspectSnapshot { run: RunId(100), correlation: 42 }))"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn inspect_unknown_run_returns_not_found() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(9999),
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run: RunId::new(9999),
                correlation: 1,
            })
        );
        Ok(())
    }

    #[test]
    fn submit_produces_run_submitted_trace() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(110);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let found = shard
            .trace_ring_mut()
            .drain()
            .iter()
            .any(|e| *e == TraceEvent::RunSubmitted { run });
        assert_eq!(found, true);
        Ok(())
    }

    #[test]
    fn cancel_produces_run_cancelled_trace() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(111);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let found = shard
            .trace_ring_mut()
            .drain()
            .iter()
            .any(|e| *e == TraceEvent::RunCancelled { run });
        assert_eq!(found, true);
        Ok(())
    }

    #[test]
    fn cancel_emits_run_cancelled_journal_event() -> Result<(), String> -> Result<(), RuntimeError> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(112);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let events = require_snapshot(&journal)?;
        assert!(
            events.contains(&RuntimeJournalEvent::RunCancelled { run, reason: None}),
            "journal events should contain RunCancelled: {events:?}"
        );
        Ok(())
        Ok(())
    }

    #[test]
    fn finish_produces_run_finished_trace() -> Result<(), String> -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("finished", finished_workflow())?;
        let run = RunId::new(113);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let found = shard
            .trace_ring_mut()
            .drain()
            .iter()
            .any(|e| *e == TraceEvent::RunFinished { run });
        assert_eq!(found, true);
        Ok(())
        Ok(())
    }

    #[test]
    fn finished_workflow_emits_one_slot_written_for_one_output_write() -> Result<(), String> -> Result<(), RuntimeError> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("finished", finished_workflow())?;
        let run = RunId::new(1130);
        submit_run(&mut shard, run, wf);

        let events = require_snapshot(&journal)?;
        let slot_written_count = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::SlotWritten {
                        run: event_run,
                        slot: SlotIdx::ZERO,
                        ..
                    } if *event_run == run
                )
            })
            .count();
        assert_eq!(slot_written_count, 1, "events: {events:?}");
        Ok(())
        Ok(())
    }

    #[test]
    fn resubmit_after_cancel_succeeds() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(300);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf.clone(),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        Ok(())
    }

    #[test]
    fn timer_fire_after_cancel_returns_run_not_found() -> Result<(), RuntimeError> {
        let mut shard = Shard::new(small_config());
        let Some(wf) = wait_workflow() else {
            return;
        };
        let run = RunId::new(400);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(invalid_timer_command(run)), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
        Ok(())
    }

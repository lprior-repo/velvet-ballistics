#[test]
fn storage_runtime_journal_maps_action_wait_and_ask_events() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(44);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::ActionScheduled {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::ActionCompleted {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
            EventSeq::new(1),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::WaitScheduled {
                run,
                step: StepIdx::new(3),
            },
            EventSeq::new(2),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::WaitResolved {
                run,
                step: StepIdx::new(3),
            },
            EventSeq::new(3),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::AskScheduled {
                run,
                step: StepIdx::new(4),
            },
            EventSeq::new(4),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::AskAnswered {
                run,
                step: StepIdx::new(4),
                slot: SlotIdx::new(5),
            },
            EventSeq::new(5),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::SlotWritten {
                run,
                slot: SlotIdx::new(5),
                value: Vec::new(),
                taint: Taint::Clean,
                extra: None,
            },
            EventSeq::new(6),
        ),
        Ok(())
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "action/wait/ask events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(1),
                action: ActionId::new(2),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
                action: ActionId::new(2),
                attempt: 1,
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(3),
                attempt: 1,
            },
            JournalEvent::WaitResolvedEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(3),
                attempt: 1,
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(4),
                attempt: 1,
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(4),
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(6),
                slot: SlotIdx::new(5),
                value: Some(Vec::new()),
                extra: vb_storage::encode_slot_written_extra(Taint::Clean, None).ok(),
                attempt: 1,
            },
        ]
    );
}

/// Regression test for bug-hunt RE-009: `WaitResolved` must map to a distinct
/// `JournalEvent::WaitResolvedEvent` rather than being mis-attributed as a
/// `RetryScheduledEvent`. A wait resolution is a resumption, not a retry.
#[test]
fn re_009_wait_resolved_maps_to_dedicated_journal_event() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(46);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::WaitResolved {
                run,
                step: StepIdx::new(7),
            },
            EventSeq::new(0),
        ),
        Ok(())
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "wait resolved event read",
    ) else {
        return;
    };
    assert_eq!(events.len(), 1);
    let event = match events.first() {
        Some(value) => value,
        None => return,
    };
    // The mis-attribution path produced RetryScheduledEvent; the fix must emit
    // the dedicated WaitResolvedEvent variant.
    assert!(
        matches!(event, JournalEvent::WaitResolvedEvent { step, attempt, .. }
            if *step == StepIdx::new(7) && *attempt == 1),
        "expected WaitResolvedEvent for WaitResolved runtime event, got {event:?}"
    );
    assert!(
        !matches!(event, JournalEvent::RetryScheduledEvent { .. }),
        "RE-009 regression: WaitResolved must not be mis-attributed as RetryScheduledEvent"
    );
    // Record-kind parity: the envelope kind must equal the variant's record kind.
    assert_eq!(event.record_kind(), vb_storage::RecordKind::WaitResolved);
}

#[test]
fn queued_storage_runtime_journal_flushes_mapped_events_to_fjall() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(4, 2), "journal queue opens") else {
        return;
    };
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = RunId::new(45);
    let workflow = WorkflowDigest::from_bytes([9; 32]);

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunSubmitted { run, workflow },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::ActionScheduled {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
            EventSeq::new(1),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunFinished {
                run,
                result: SlotIdx::new(3),
            },
            EventSeq::new(2),
        ),
        Ok(())
    );

    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 2 && report.written == 2)
    );
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "queued events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
                action: ActionId::new(2),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(3),
                attempt: 1,
            },
        ]
    );
}

#[test]
fn runtime_journal_config_maps_profiles_to_volatile_journaled_and_strict_behavior() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(volatile_queue) = require_ok(journal_queue(4, 4), "volatile queue opens") else {
        return;
    };
    let run = RunId::new(47);
    let workflow = WorkflowDigest::from_bytes([10; 32]);

    let volatile = RuntimeJournalConfig::new(DurabilityProfile::Volatile)
        .shared_journal(journal.clone(), volatile_queue.clone());
    assert_eq!(
        volatile.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
    assert!(matches!(
        volatile_queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));

    let Some(journaled_queue) = require_ok(journal_queue(4, 4), "journaled queue opens") else {
        return;
    };
    let journaled = RuntimeJournalConfig::new(DurabilityProfile::Journaled)
        .shared_journal(journal.clone(), journaled_queue.clone());
    assert_eq!(
        journaled.append_sequenced(
            RuntimeJournalEvent::RunCancelled { run, reason: None },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert!(matches!(
        journaled_queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 1 && counts.strict == 0
    ));

    let Some(strict_queue) = require_ok(journal_queue(4, 4), "strict queue opens") else {
        return;
    };
    let strict_run = RunId::new(48);
    let strict = RuntimeJournalConfig::new(DurabilityProfile::Strict)
        .shared_journal(journal.clone(), strict_queue.clone());
    assert_eq!(
        strict.append_sequenced(
            RuntimeJournalEvent::RunFailed { run: strict_run },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert!(matches!(
        strict_queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    assert!(matches!(
        journal.events_for_run(strict_run),
        Ok(events) if matches!(events.as_slice(), [JournalEvent::RunFailedEvent { seq, attempt: 1, ..}] if *seq == EventSeq::new(0))
    ));
}

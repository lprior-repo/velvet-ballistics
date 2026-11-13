use std::error::Error;

fn store_encoded_event(
    journal: &FjallJournal,
    key_run: RunId,
    key_seq: EventSeq,
    envelope_seq: u64,
    event: &JournalEvent,
) -> Result<(), Box<dyn Error>> {
    let key = crate::keys::run_event_key(key_run, key_seq)?;
    let value = crate::codec::encode_record(
        crate::constants::MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        envelope_seq,
        event,
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    journal.events.insert(key.to_vec(), value)?;
    Ok(())
}

fn store_snapshot(journal: &FjallJournal, run: RunId) -> Result<(), Box<dyn Error>> {
    journal.put_snapshot(&RunSnapshot {
        run,
        seq: EventSeq::new(2),
        workflow: WorkflowDigest::from_bytes([0x51; DIGEST_BYTES]),
        slots: vec![],
        taint: vec![],
    })?;
    Ok(())
}

fn store_malformed_event(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
) -> Result<(), Box<dyn Error>> {
    let key = crate::keys::run_event_key(run, seq)?;
    let value = crate::codec::encode_record(
        crate::constants::MAGIC_JOURNAL_EVENT,
        crate::RecordKind::StepStarted,
        seq.get(),
        &(),
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    journal.events.insert(key.to_vec(), value)?;
    Ok(())
}

#[test]
fn retention_rejects_terminal_event_under_mismatched_key_sequence() -> Result<(), Box<dyn Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5001);
    let payload_seq = EventSeq::new(3);
    let key_seq = EventSeq::new(4);
    let event = make_run_finished(run, payload_seq.get());
    store_snapshot(&journal, run)?;
    store_encoded_event(&journal, run, key_seq, payload_seq.get(), &event)?;
    let key = crate::keys::run_event_key(run, key_seq)?;
    let before = journal.events.get(key.as_slice())?.ok_or("event missing")?;

    let error = match journal.trim_events_for_run(run, TrimPolicy::default()) {
        Err(error) => error,
        Ok(result) => return Err(format!("expected corrupt event error, got {result:?}").into()),
    };

    assert!(matches!(
        error,
        TrimError::Journal(JournalError::ReplayKeyMismatch {
            run: actual_run,
            key_seq: 4,
            payload_seq: 3,
        }) if actual_run == run
    ));
    let after = journal.events.get(key.as_slice())?.ok_or("event removed")?;
    assert_eq!(after.as_ref(), before.as_ref());
    assert_eq!(
        journal
            .snapshot(run, EventSeq::new(2))?
            .map(|value| value.seq),
        Some(EventSeq::new(2))
    );
    Ok(())
}

#[test]
fn retention_rejects_payload_from_another_run() -> Result<(), Box<dyn Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5002);
    let other_run = RunId::new(5003);
    let event = make_run_finished(other_run, 1);
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(1),
        workflow: WorkflowDigest::from_bytes([0x52; DIGEST_BYTES]),
        slots: vec![1, 2],
        taint: vec![],
    };
    journal.put_snapshot(&snapshot)?;
    store_encoded_event(&journal, run, EventSeq::new(1), 1, &event)?;
    let key = crate::keys::run_event_key(run, EventSeq::new(1))?;
    let before = journal.events.get(key.as_slice())?.ok_or("event missing")?;

    let error = match journal.trim_events_for_run(run, TrimPolicy::default()) {
        Err(error) => error,
        Ok(result) => return Err(format!("expected run mismatch error, got {result:?}").into()),
    };

    assert!(matches!(
        error,
        TrimError::Journal(JournalError::WrongRun { expected, actual })
            if expected == run && actual == other_run
    ));
    let after = journal.events.get(key.as_slice())?.ok_or("event removed")?;
    assert_eq!(after.as_ref(), before.as_ref());
    assert_eq!(journal.snapshot(run, EventSeq::new(1))?, Some(snapshot));
    Ok(())
}

#[test]
fn retention_rejects_corrupt_nonterminal_before_terminal_event() -> Result<(), Box<dyn Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5004);
    let corrupt = make_step_started(run, 1, 0);
    store_encoded_event(&journal, run, EventSeq::new(0), 1, &corrupt)?;
    let terminal = make_run_finished(run, 2);
    store_encoded_event(&journal, run, EventSeq::new(2), 2, &terminal)?;

    let error = match journal.has_terminal_event(run) {
        Err(error) => error,
        Ok(found) => return Err(format!("expected corrupt event error, got {found:?}").into()),
    };

    assert!(matches!(
        error,
        TrimError::Journal(JournalError::ReplayKeyMismatch {
            run: actual_run,
            key_seq: 0,
            payload_seq: 1,
        }) if actual_run == run
    ));
    Ok(())
}

#[test]
fn retention_preserves_envelope_payload_sequence_error() -> Result<(), Box<dyn Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5005);
    let event = make_step_started(run, 1, 0);
    store_encoded_event(&journal, run, EventSeq::new(1), 9, &event)?;

    let error = match journal.has_terminal_event(run) {
        Err(error) => error,
        Ok(found) => return Err(format!("expected envelope mismatch error, got {found:?}").into()),
    };

    assert!(matches!(
        error,
        TrimError::Journal(JournalError::ReplayEnvelopeSequenceMismatch {
            run: actual_run,
            envelope_seq: 9,
            payload_seq: 1,
        }) if actual_run == run
    ));
    Ok(())
}

#[test]
fn retention_rejects_malformed_payload_before_terminal_event() -> Result<(), Box<dyn Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5006);
    store_malformed_event(&journal, run, EventSeq::new(0))?;
    let terminal = make_run_finished(run, 1);
    store_encoded_event(&journal, run, EventSeq::new(1), 1, &terminal)?;

    let error = match journal.has_terminal_event(run) {
        Err(error) => error,
        Ok(found) => return Err(format!("expected malformed payload error, got {found:?}").into()),
    };

    assert!(matches!(
        error,
        TrimError::Journal(JournalError::PostcardDecodeFailed(
            postcard::Error::DeserializeUnexpectedEnd
        ))
    ));
    Ok(())
}

#[test]
fn zero_retention_short_circuits_populated_corrupt_journal() -> Result<(), Box<dyn Error>> {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(5007);
    write_header(
        &journal,
        run,
        WorkflowDigest::from_bytes([0x57; DIGEST_BYTES]),
    );
    store_malformed_event(&journal, run, EventSeq::new(0))?;
    let zero_policy = TrimPolicy {
        retain_last_n_terminal: 0,
        ..TrimPolicy::default()
    };
    journal.check_retention_policy(run, &zero_policy)?;
    assert_eq!(
        journal.compute_retained_terminal_runs(&zero_policy)?,
        std::collections::HashSet::new()
    );

    let scan_policy = TrimPolicy {
        retain_last_n_terminal: 1,
        ..TrimPolicy::default()
    };
    assert!(matches!(
        journal.compute_retained_terminal_runs(&scan_policy),
        Err(JournalError::PostcardDecodeFailed(
            postcard::Error::DeserializeUnexpectedEnd
        ))
    ));
    Ok(())
}

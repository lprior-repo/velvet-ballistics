use super::support::{Case, TestResult};
use vb_core::{SlotValue, Taint};

pub(super) fn execution_prefix(case: &Case, journal: &vb_storage::FjallJournal) -> TestResult {
    let events = journal.events_for_run_full(case.run)?;
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, vb_storage::JournalEvent::RunAccepted { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, vb_storage::JournalEvent::RunAdmission { .. }))
            .count(),
        1
    );
    assert!(
        matches!(events.get(2), Some(vb_storage::JournalEvent::StepStarted { run, seq, step, attempt })
        if *run == case.run && seq.get() == 2 && *step == case.workflow.entry() && *attempt == 1)
    );
    for (raw, event) in events.iter().enumerate() {
        assert_eq!(event.seq().get(), u64::try_from(raw)?);
    }
    Ok(())
}

pub(super) fn terminal_output(
    case: &Case,
    journal: &vb_storage::FjallJournal,
    expected: SlotValue,
) -> TestResult {
    execution_prefix(case, journal)?;
    let events = journal.events_for_run_full(case.run)?;
    let output = case.output(4)?;
    let writes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            vb_storage::JournalEvent::SlotWrittenEvent {
                slot, value, extra, ..
            } if *slot == output => Some((value, extra)),
            _ => None,
        })
        .collect();
    assert_eq!(writes.len(), 1);
    let (value, extra) = writes.first().ok_or("output write missing")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(value.as_ref().ok_or("value missing")?)?,
        expected
    );
    let decoded = vb_storage::decode_slot_written_extra(extra.as_ref().ok_or("taint missing")?)
        .map_err(|e| format!("{e:?}"))?;
    let vb_storage::DecodedSlotWrittenExtra::Envelope(envelope) = decoded else {
        return Err("legacy extra unexpectedly emitted".into());
    };
    assert_eq!(envelope.taint, Taint::Secret);
    assert_eq!(envelope.frame_extra, None);
    assert!(
        matches!(events.last(), Some(vb_storage::JournalEvent::RunFinished { run, result, attempt, .. })
        if *run == case.run && *result == output && *attempt == 1)
    );
    Ok(())
}

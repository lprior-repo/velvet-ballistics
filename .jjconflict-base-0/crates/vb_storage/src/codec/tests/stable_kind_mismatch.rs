#[test]
fn stable_schema_one_step_succeeded_payload_under_slot_written_envelope_is_rejected()
-> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(501),
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    };
    assert_stable_payload_kind_mismatch(&event, RecordKind::SlotWritten, RecordKind::StepSucceeded)
}

#[test]
fn stable_schema_one_slot_written_payload_under_step_succeeded_envelope_is_rejected()
-> Result<(), JournalError> {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(502),
        seq: EventSeq::new(2),
        slot: SlotIdx::new(3),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_stable_payload_kind_mismatch(&event, RecordKind::StepSucceeded, RecordKind::SlotWritten)
}

#[test]
fn stable_schema_one_action_ticket_payload_under_legacy_action_scheduled_envelope_is_rejected()
-> Result<(), JournalError> {
    let run = RunId::new(503);
    let ticket = codec_action_ticket(run, StepIdx::new(4), ActionId::new(9));
    let event = JournalEvent::ActionScheduledTicket {
        run,
        seq: EventSeq::new(3),
        ticket,
        input: SlotIdx::new(1),
        output: SlotIdx::new(2),
        action_abi_digest: codec_sample_digest(0xA2),
    };
    assert_stable_payload_kind_mismatch(
        &event,
        RecordKind::ActionScheduled,
        RecordKind::ActionScheduledTicket,
    )
}

#[test]
fn stable_schema_one_action_envelope_payload_under_legacy_action_completed_envelope_is_rejected()
-> Result<(), JournalError> {
    let run = RunId::new(504);
    let ticket = codec_action_ticket(run, StepIdx::new(5), ActionId::new(10));
    let value = postcard::to_allocvec(&SlotValue::Bool(true)).map_err(JournalError::Encode)?;
    let encoded_len = u32::try_from(value.len()).map_err(|_| JournalError::PayloadTooLarge {
        len: u32::MAX,
        max: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    })?;
    let event = JournalEvent::ActionCompletedEnvelope {
        run,
        seq: EventSeq::new(4),
        ticket,
        output: SlotIdx::new(2),
        outcome: DurableActionOutcome::Ready,
        value: value.clone(),
        encoded_len,
        taint: Taint::Clean,
        value_digest: *blake3::hash(&value).as_bytes(),
        action_abi_digest: codec_sample_digest(0xA3),
    };
    assert_stable_payload_kind_mismatch(
        &event,
        RecordKind::ActionCompleted,
        RecordKind::ActionCompletedEnvelope,
    )
}

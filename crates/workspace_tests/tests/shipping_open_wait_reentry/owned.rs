use super::*;

pub(super) struct OwnedInput {
    pub(super) slot: vb_core::SlotIdx,
    pub(super) list: vb_core::ListId,
    pub(super) source_digest: vb_core::WorkflowDigest,
}

pub(super) fn commit_owned_input(
    journal: &vb_storage::FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    source: &str,
    run: RunId,
    contracts: &[ActionContract],
) -> TestResult<OwnedInput> {
    let slot = workflow
        .input_slots()
        .first()
        .ok_or("missing retained input")?
        .slot;
    let mut store = vb_core::ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store.insert_list(Box::from([SlotValue::I64(73)]))?;
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::from([(slot, SlotValue::List(list), Taint::Clean)]),
        store.checkpoint_v1(),
        workflow,
    )?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        workflow,
        RuntimePolicy::Strict,
        contracts,
    )?;
    let source_record = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source_digest,
        &bundle,
        workflow,
    )?;
    let submission = vb_storage::RunSubmission::owned(
        source_record,
        artifact,
        inputs,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    Ok(OwnedInput {
        slot,
        list,
        source_digest,
    })
}

pub(super) fn append_corrupt_slot_taint(
    journal: &vb_storage::FjallJournal,
    run: RunId,
    source_events: &[JournalEvent],
    prefix_len: usize,
    slot: vb_core::SlotIdx,
) -> TestResult<(Vec<JournalEvent>, Vec<Vec<u8>>)> {
    let seq = target_slot_write_seq(source_events, slot)
        .ok_or("durable slot write for corruption target missing")?;
    let mut events = Vec::new();
    source_events
        .iter()
        .take(prefix_len)
        .cloned()
        .for_each(|event| events.push(event));
    let mut changed = 0usize;
    for event in source_events.iter().skip(prefix_len) {
        let next = corrupt_taint_extra(event, seq, &mut changed)?;
        journal.append_strict(&next)?;
        events.push(next);
    }
    if changed != 1 {
        return Err("corruption target was not a single durable slot write".into());
    }
    let raw = raw_event_bytes(journal, run, &events)?;
    Ok((events, raw))
}

pub(super) fn raw_event_bytes(
    journal: &vb_storage::FjallJournal,
    run: RunId,
    events: &[JournalEvent],
) -> TestResult<Vec<Vec<u8>>> {
    events
        .iter()
        .map(|event| {
            journal
                .get_event_bytes(run, event.seq())?
                .ok_or_else(|| "raw event bytes missing".into())
        })
        .collect()
}

pub(super) fn slot_written_taint(
    events: &[JournalEvent],
    slot: vb_core::SlotIdx,
) -> TestResult<Taint> {
    let extra = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot: event_slot,
                extra: Some(extra),
                ..
            } if *event_slot == slot => Some(extra.as_slice()),
            _ => None,
        })
        .ok_or("slot taint envelope missing")?;
    match vb_storage::decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))? {
        vb_storage::DecodedSlotWrittenExtra::Envelope(envelope) => Ok(envelope.taint),
        vb_storage::DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
            Err("slot taint extra is legacy".into())
        }
    }
}

fn target_slot_write_seq(
    events: &[JournalEvent],
    slot: vb_core::SlotIdx,
) -> Option<vb_storage::EventSeq> {
    events.iter().rev().find_map(|event| match event {
        JournalEvent::SlotWrittenEvent {
            seq,
            slot: event_slot,
            ..
        } if *event_slot == slot => Some(*seq),
        _ => None,
    })
}

fn corrupt_taint_extra(
    event: &JournalEvent,
    target: vb_storage::EventSeq,
    changed: &mut usize,
) -> TestResult<JournalEvent> {
    match event {
        JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value,
            extra,
            attempt,
            ..
        } if *seq == target => {
            *changed = changed.saturating_add(1);
            Ok(JournalEvent::SlotWrittenEvent {
                run: *run,
                seq: *seq,
                slot: *slot,
                value: value.clone(),
                extra: Some(changed_taint_extra(
                    extra
                        .as_deref()
                        .ok_or("corruption target lacks taint metadata")?,
                )?),
                attempt: *attempt,
            })
        }
        _ => Ok(event.clone()),
    }
}

fn changed_taint_extra(bytes: &[u8]) -> TestResult<Vec<u8>> {
    let vb_storage::DecodedSlotWrittenExtra::Envelope(envelope) =
        vb_storage::decode_slot_written_extra(bytes).map_err(|error| format!("{error:?}"))?
    else {
        return Err("corruption target has legacy taint metadata".into());
    };
    let changed = if envelope.taint == Taint::Clean {
        Taint::Secret
    } else {
        Taint::Clean
    };
    vb_storage::encode_slot_written_extra(changed, envelope.frame_extra)
        .map_err(|error| format!("{error:?}").into())
}

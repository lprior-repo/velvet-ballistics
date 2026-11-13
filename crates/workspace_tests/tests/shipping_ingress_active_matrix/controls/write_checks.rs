use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::JournalEvent;

use super::super::support::TestResult;
use super::helpers::checkpoint_list;

pub(super) fn capture_slot_write_stores(
    events: &[JournalEvent],
    run: RunId,
    slot: SlotIdx,
    checkpoint: &ValueStoreCheckpointV1,
    observations: &mut Vec<(vb_storage::EventSeq, ValueStoreCheckpointV1)>,
) {
    for event in events {
        let JournalEvent::SlotWrittenEvent {
            run: event_run,
            seq,
            slot: event_slot,
            ..
        } = event
        else {
            continue;
        };
        if *event_run == run
            && *event_slot == slot
            && !observations.iter().any(|(observed, _)| observed == seq)
        {
            observations.push((*seq, checkpoint.clone()));
        }
    }
}

pub(super) fn assert_collect_pages(
    observations: &[(vb_storage::EventSeq, ValueStoreCheckpointV1)],
    writes: &[(vb_storage::EventSeq, SlotValue, Taint, u16)],
) -> TestResult {
    assert_eq!(writes.len(), 2, "Collect should write one batch per page");
    for (index, (seq, value, taint, attempt)) in writes.iter().enumerate() {
        assert_eq!(*taint, Taint::Clean, "Collect body taint at {seq:?}");
        assert_eq!(*attempt, 1, "Collect body attempt at {seq:?}");
        let SlotValue::List(list_id) = value else {
            return Err(format!("Collect body write at {seq:?} was not a list: {value:?}").into());
        };
        let store = observations
            .iter()
            .find_map(|(observed_seq, store)| (observed_seq == seq).then_some(store))
            .ok_or_else(|| format!("Collect write {seq:?} lacks its corresponding store"))?;
        let page = checkpoint_list(store, *list_id)?;
        let expected = if index == 0 {
            &[SlotValue::I64(1), SlotValue::I64(2)][..]
        } else {
            &[SlotValue::I64(3)][..]
        };
        assert_eq!(page, expected, "Collect page at event {seq:?}");
    }
    Ok(())
}

pub(super) fn assert_body_write_provenance(
    workflow: &vb_core::CompiledWorkflow,
    events: &[JournalEvent],
    producers: &[StepIdx],
    run: RunId,
    slot: SlotIdx,
    writes: &[(vb_storage::EventSeq, SlotValue, Taint, u16)],
) -> TestResult {
    assert_eq!(producers.len(), writes.len(), "producer count for {slot:?}");
    for ((seq, _, _, attempt), producer) in writes.iter().zip(producers) {
        let positions: Vec<usize> = events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| {
                matches!(
                    event,
                    JournalEvent::SlotWrittenEvent {
                        run: event_run,
                        seq: event_seq,
                        slot: event_slot,
                        ..
                    } if *event_run == run && event_seq == seq && *event_slot == slot
                )
                .then_some(index)
            })
            .collect();
        let [index] = positions.as_slice() else {
            return Err(
                format!("expected one body write event at {seq:?}, found {positions:?}").into(),
            );
        };
        let node = workflow
            .node(*producer)
            .ok_or("missing compiled producer")?;
        let owns_slot = node.output.slot() == Some(slot)
            || matches!(
                &node.kind,
                vb_core::CompiledNodeKind::ReduceStart { accumulator, .. }
                    | vb_core::CompiledNodeKind::ReduceFinish { accumulator }
                    if *accumulator == slot
            );
        assert!(owns_slot, "producer {producer:?} does not own {slot:?}");
        let preceding = events.get(..*index).and_then(|head| {
            head.iter().rev().find(|event| {
                matches!(event, JournalEvent::StepStarted { run: event_run, .. } | JournalEvent::StepSucceeded { run: event_run, .. } if *event_run == run)
            })
        });
        assert!(
            matches!(preceding, Some(JournalEvent::StepStarted { run: event_run, step, attempt: started_attempt, .. }) if *event_run == run && step == producer && started_attempt == attempt),
            "write {seq:?} for {slot:?} expected producer {producer:?}, preceding {preceding:?}"
        );
        let following = events.get(index.saturating_add(1)..).and_then(|tail| {
            tail.iter().find(|event| {
                matches!(event, JournalEvent::StepStarted { run: event_run, .. } | JournalEvent::StepSucceeded { run: event_run, .. } if *event_run == run)
            })
        });
        assert!(
            matches!(following, Some(JournalEvent::StepSucceeded { run: event_run, step, output, .. }) if *event_run == run && step == producer && node.output.slot() == Some(*output)),
            "write {seq:?} for {slot:?} expected completion {producer:?}, following {following:?}"
        );
    }
    Ok(())
}

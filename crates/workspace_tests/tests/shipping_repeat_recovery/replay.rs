//! Durable Repeat traces: reopen, artifact replay and evidence refusal.

use vb_core::{SlotValue, Taint};
use vb_storage::JournalEvent as E;

use crate::support::{
    SOURCE, TestResult, counter_sequence, drive_live, expect_divergence, persist_reopen,
    repeat_nodes, replay, started,
};

#[test]
fn repeat_persisted_trace_replays_exact_counter_state_and_pc() -> TestResult {
    let (workflow, events) = persist_reopen(SOURCE)?;
    let nodes = repeat_nodes(&workflow, 3)?;
    let (_, live) = drive_live(SOURCE, None, 9)?;
    assert_eq!(started(&events, nodes.body), 3);
    assert_eq!(started(&events, nodes.control), 3);
    assert_eq!(started(&events, nodes.finish), 1);
    assert_eq!(
        counter_sequence(&events, nodes.attempt)?,
        vec![(3, 0), (3, 1), (3, 2), (3, 3)]
    );
    let frame = replay(&events, &workflow)?
        .ok_or("complete scalar replay skipped")?
        .into_frame();
    assert_eq!(frame.checkpoint_v1(), live.checkpoint_v1());
    assert_eq!(frame.pc(), nodes.terminal);
    assert_eq!(frame.read_slot(nodes.result)?, &SlotValue::I64(9));
    Ok(())
}

#[test]
fn repeat_replay_rejects_incomplete_or_tampered_evidence() -> TestResult {
    let (workflow, events) = persist_reopen(SOURCE)?;
    let nodes = repeat_nodes(&workflow, 3)?;

    let mut missing = events.clone();
    let receipt = missing
        .iter()
        .rposition(|event| matches!(event, E::StepSucceeded { step, .. } if *step == nodes.control))
        .ok_or("repeat check receipt")?;
    missing.remove(receipt);
    expect_divergence(
        &missing,
        &workflow,
        nodes.finish,
        "incomplete bound event sequence",
    )?;

    let mut counter = events.clone();
    let forged = vb_core::engine::repeat::encode_repeat_state(3, 2)?;
    let capture = counter
        .iter_mut()
        .find(|event| {
            matches!(
                event,
                E::SlotWrittenEvent { slot, value: Some(bytes), .. }
                    if *slot == nodes.attempt
                        && postcard::from_bytes::<SlotValue>(bytes).ok()
                            == Some(SlotValue::I64(0x0003_0000_0001))
            )
        })
        .ok_or("first counter capture")?;
    if let E::SlotWrittenEvent { value, .. } = capture {
        *value = Some(postcard::to_allocvec(&SlotValue::I64(forged))?);
    }
    expect_divergence(
        &counter,
        &workflow,
        nodes.control,
        "capture differs from artifact replay",
    )?;

    let finish_capture = |events: &[E]| {
        events.iter().rposition(|event| {
            matches!(
                event,
                E::SlotWrittenEvent { slot, value: Some(bytes), .. }
                    if *slot == nodes.result
                        && postcard::from_bytes::<SlotValue>(bytes).ok()
                            == Some(SlotValue::I64(9))
            )
        })
    };
    let mut value = events.clone();
    let index = finish_capture(&value).ok_or("repeat finish value capture")?;
    if let Some(E::SlotWrittenEvent { value, .. }) = value.get_mut(index) {
        *value = Some(postcard::to_allocvec(&SlotValue::I64(10))?);
    }
    expect_divergence(
        &value,
        &workflow,
        nodes.finish,
        "capture differs from artifact replay",
    )?;

    let mut taint = events;
    let index = finish_capture(&taint).ok_or("repeat finish taint capture")?;
    if let Some(E::SlotWrittenEvent { extra, .. }) = taint.get_mut(index) {
        *extra = Some(
            vb_storage::slot_extra::encode_slot_written_extra(Taint::Secret, None)
                .map_err(|error| format!("taint capture: {error:?}"))?,
        );
    }
    expect_divergence(
        &taint,
        &workflow,
        nodes.finish,
        "capture differs from artifact replay",
    )
}

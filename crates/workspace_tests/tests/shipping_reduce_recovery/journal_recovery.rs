#![forbid(unsafe_code)]

//! Real Fjall journal cold recovery for a source Reduce over the runtime-owned
//! `inputs.items` list default, with a Do suspension inside the Reduce body.

#[path = "journal_recovery/support.rs"]
mod support;

use std::sync::Arc;

use support::{
    RUN, TestResult, assert_resumable, complete, list_items, open, run_to_finish, shape, start,
    tick_to_boundary,
};
use vb_core::{RunId, SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_storage::{DecodedSlotWrittenExtra, FjallJournal, JournalEvent, decode_slot_written_extra};

#[test]
fn source_reduce_owned_default_survives_fjall_reopen_and_action_resume() -> TestResult {
    run_to_finish(&shape()?)?;

    let shape = shape()?;
    assert_eq!(shape.limit, 2, "source input max becomes the reduce limit");
    let (journal, mut runtime) = start(&shape)?;
    let ticket = tick_to_boundary(&mut runtime, &shape)?.ok_or("expected first suspension")?;
    let before = runtime.inspect_run_state(RunId::new(RUN))?;
    let pc = before.frame.pc();
    let executed = before.frame.executed();
    assert_eq!(pc, shape.do_step);
    assert_eq!(
        *before.frame.read_slot(shape.accumulator)?,
        SlotValue::I64(10)
    );
    assert_eq!(before.frame.read_taint(shape.accumulator)?, Taint::Clean);
    assert_eq!(
        list_items(&runtime, &shape)?,
        [SlotValue::I64(3), SlotValue::I64(5)]
    );
    drop(runtime);
    drop(journal);

    let journal = Arc::new(FjallJournal::open(shape.directory.path(), None)?);
    let mut runtime = open(Arc::clone(&journal));
    assert_resumable(&runtime)?;
    runtime.recover_and_resume(RunId::new(RUN))?;
    runtime.tick_all()?;
    let restored = runtime.inspect_run_state(RunId::new(RUN))?;
    assert_eq!(restored.frame.pc(), pc);
    assert_eq!(restored.frame.executed(), executed);
    assert_eq!(
        *restored.frame.read_slot(shape.accumulator)?,
        SlotValue::I64(10)
    );
    assert_eq!(restored.frame.read_taint(shape.accumulator)?, Taint::Clean);
    assert_eq!(
        list_items(&runtime, &shape)?,
        [SlotValue::I64(3), SlotValue::I64(5)]
    );
    assert_eq!(
        runtime.lookup_pending_action_ticket(RunId::new(RUN), u64::from(shape.do_step.get())),
        Some(ticket)
    );

    complete(&mut runtime, &shape, ticket, 20)?;
    let second = tick_to_boundary(&mut runtime, &shape)?.ok_or("expected second suspension")?;
    let mid = runtime.inspect_run_state(RunId::new(RUN))?;
    assert_eq!(*mid.frame.read_slot(shape.accumulator)?, SlotValue::I64(20));
    assert_eq!(mid.frame.read_taint(shape.accumulator)?, Taint::Secret);
    drop(runtime);

    let mut runtime = open(Arc::clone(&journal));
    assert_resumable(&runtime)?;
    runtime.recover_and_resume(RunId::new(RUN))?;
    runtime.tick_all()?;
    complete(&mut runtime, &shape, second, 30)?;
    assert_eq!(
        list_items(&runtime, &shape)?,
        [SlotValue::I64(3), SlotValue::I64(5)]
    );
    tick_to_boundary(&mut runtime, &shape)?;
    let counters = runtime.counters_snapshot();
    assert_eq!((counters.runs_completed, counters.runs_failed), (1, 0));
    assert!(matches!(
        runtime.inspect_run_state(RunId::new(RUN)),
        Err(RuntimeError::RunNotFound)
    ));
    let events = journal.events_for_run_full(RunId::new(RUN))?;
    let workflow = vb_compile::compile_workflow(&shape.source.source)?;
    let result_slot = crate::support::finish_result(&workflow)?;
    let (encoded_value, encoded_extra) = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot,
                value: Some(value),
                extra: Some(extra),
                ..
            } if *slot == result_slot => Some((value, extra)),
            _ => None,
        })
        .ok_or("completed Reduce result capture absent")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(encoded_value)?,
        SlotValue::I64(30)
    );
    match decode_slot_written_extra(encoded_extra).map_err(|error| format!("{error:?}"))? {
        DecodedSlotWrittenExtra::Envelope(envelope) => {
            assert_eq!(envelope.taint, Taint::Secret);
            assert_eq!(envelope.frame_extra, None);
        }
        DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
            return Err("completed Reduce result uses legacy extra bytes".into());
        }
    }
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    Ok(())
}

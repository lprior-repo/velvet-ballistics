#![forbid(unsafe_code)]

use super::source::{compound, indent};
use super::support::{Case, TestResult};
use vb_core::{CompiledNodeKind as K, RunId, SlotValue, Taint};
use vb_ipc::{IpcPayload, server::IpcResponse};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::AskTicket;
use vb_storage::{DecodedSlotWrittenExtra, JournalEvent, decode_slot_written_extra};

const RUN: RunId = RunId::new(4_709);

fn nested_ask_source() -> TestResult<String> {
    let ask = "- id: question\n  ask: {prompt: '$vars.prompt'}\n";
    let repeat = compound("repeat", "middle", ask)?;
    let outer = compound("for_each", "outer", &repeat)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: answer_slot_nested_ask\nwhen: {{manual: {{}}}}\nvars: {{prompt: 'row'}}\ninputs: {{items: {{is: list, of: number, max: 2}}}}\nsteps:\n{}  - id: done\n    finish: {{result: '$step.outer'}}\n",
        indent(&outer, 2)
    ))
}

#[test]
fn nested_owned_ask_slot_answers_exact_value_taint_and_authority() -> TestResult {
    let source = nested_ask_source()?;
    let case = Case::new(
        &source,
        RUN,
        &[SlotValue::I64(10), SlotValue::I64(20)],
        &[Taint::Secret, Taint::Clean],
        &[],
    )?;
    let ask = case.step(|kind| matches!(kind, K::Ask { .. }))?;
    let (journal, mut runtime) = case.activate_live()?;
    let ticket = wait_for_ticket(&mut runtime, RUN, ask, case.workflow.node_count())?;
    let before_queue = runtime.pending_boundary_snapshot(64).command_queue_depth();
    assert_eq!(before_queue, 0);
    assert_eq!(
        answer_slot(
            &mut runtime,
            RUN,
            ticket,
            SlotValue::I64(99),
            Taint::DerivedFromSecret
        )?,
        IpcResponse::AcceptedRun { run_id: RUN.get() }
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        1
    );
    runtime.tick_all()?;
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        0
    );
    assert_ne!(runtime.lookup_pending_ask_ticket(RUN, ask), Some(ticket));
    assert_exact_answer(
        &journal.events_for_run_full(RUN)?,
        RUN,
        ticket,
        SlotValue::I64(99),
        Taint::DerivedFromSecret,
    )
}

fn wait_for_ticket(
    runtime: &mut Runtime,
    run: RunId,
    ask: vb_core::StepIdx,
    nodes: u16,
) -> TestResult<AskTicket> {
    let bound = usize::from(nodes)
        .checked_mul(64)
        .ok_or("tick bound overflow")?;
    for _ in 0..bound {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(run, ask) {
            return Ok(ticket);
        }
    }
    Err("nested owned Ask did not become pending".into())
}

fn answer_slot(
    runtime: &mut Runtime,
    run: RunId,
    ticket: AskTicket,
    value: SlotValue,
    taint: Taint,
) -> TestResult<IpcResponse> {
    let wire_ticket = 65_536_u64
        .checked_add(u64::from(ticket.answer_slot().get()))
        .ok_or("slot ticket overflow")?;
    let payload = postcard::to_allocvec(&IpcPayload::AnswerAsk {
        run_id: run,
        ticket: wire_ticket,
        answer: postcard::to_allocvec(&value)?,
        taint: Some(taint),
    })?;
    Ok(vb_ipc::server::handlers::handle_answer_ask(
        &payload, runtime,
    ))
}

fn assert_exact_answer(
    events: &[JournalEvent],
    run: RunId,
    ticket: AskTicket,
    value: SlotValue,
    taint: Taint,
) -> TestResult {
    let answers = events.iter().filter(|event| matches!(event, JournalEvent::AskAnsweredEvent { run: actual, step, .. } if *actual == run && *step == ticket.ask_step())).count();
    assert_eq!(answers, 1);
    let writes = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                run: actual,
                slot,
                value: Some(value),
                extra: Some(extra),
                ..
            } if *actual == run && *slot == ticket.answer_slot() => Some((value, extra)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!writes.is_empty());
    for (bytes, extra) in writes {
        assert_eq!(postcard::from_bytes::<SlotValue>(bytes)?, value);
        let decoded = decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))?;
        assert!(
            matches!(decoded, DecodedSlotWrittenExtra::Envelope(ref envelope) if envelope.taint == taint)
        );
    }
    Ok(())
}

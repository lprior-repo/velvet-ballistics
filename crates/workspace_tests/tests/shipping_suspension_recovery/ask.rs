//! Ask suspension recovery scenarios.

use vb_core::{SlotValue, Taint};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::shard::AskAnswer;
use vb_storage::JournalEvent;

use super::support::{
    RUN, TestResult, ask_source, ask_step, drain_until_completed, finish_result_slot, open,
    pending_ask, recorded_slot_writes, submit,
};

/// Given an open `Ask` suspension, When the store is closed and reopened,
/// Then public recovery re-issues the same ticket and the answer completes it.
#[test]
fn recovered_open_ask_keeps_its_ticket_and_the_answer_completes_the_run() -> TestResult {
    let directory = tempfile::tempdir()?;
    let workflow = vb_compile::compile_workflow(ask_source().as_bytes())?;
    let ask = ask_step(&workflow)?;
    let answer_slot = workflow
        .node(ask)
        .ok_or("Ask node")?
        .output
        .slot()
        .ok_or("Ask answer slot")?;
    let ticket = {
        let (journal, mut runtime) = open(directory.path())?;
        submit(&journal, &mut runtime, &workflow, &[])?;
        let ticket = pending_ask(&mut runtime, ask)?;
        assert_eq!(
            runtime.lookup_pending_action_ticket(RUN, u64::from(ask.get())),
            None
        );
        drop(runtime);
        drop(journal);
        ticket
    };

    let (journal, mut runtime) = open(directory.path())?;
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let recovered = pending_ask(&mut runtime, ask)?;
    assert_eq!(recovered, ticket, "recovery must preserve ask boundary seq");
    runtime.answer_ask(AskAnswer::new(
        recovered,
        recovered.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    ))?;
    drain_until_completed(&mut runtime)?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    let answered_seq = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step,
                attempt,
            } if *run == RUN && *step == ask && *attempt == 1 => Some(*seq),
            _ => None,
        })
        .ok_or("durable AskAnswered event missing")?;
    let answer_writes = recorded_slot_writes(&events, answer_slot)?;
    assert_eq!(
        answer_writes.len(),
        2,
        "answer slot must preserve both durable writes"
    );
    assert!(answer_writes.iter().all(|(_, value, taint, attempt)| {
        *value == SlotValue::I64(99) && *taint == Taint::Clean && *attempt == 1
    }));
    assert_eq!(
        answer_writes.first().map(|(seq, _, _, _)| seq.get()),
        Some(
            answered_seq
                .get()
                .checked_sub(1)
                .ok_or("answer seq underflow")?
        ),
        "the answer value write immediately precedes AskAnswered"
    );
    let result = finish_result_slot(&workflow)?;
    assert!(events.iter().any(|event| matches!(
        event,
        JournalEvent::RunFinished { run, result: actual, attempt, .. }
            if *run == RUN && *actual == result && *attempt == 1
    )));
    let result_writes = recorded_slot_writes(&events, result)?;
    assert_eq!(result_writes.len(), 1);
    assert!(result_writes.iter().all(|(_, value, taint, attempt)| {
        *value == SlotValue::I64(42) && *taint == Taint::Clean && *attempt == 1
    }));
    Ok(())
}

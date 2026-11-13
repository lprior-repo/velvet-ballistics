//! Ask suspension recovery scenarios.

use vb_core::{SlotValue, Taint};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::shard::AskAnswer;
use vb_storage::JournalEvent;

use super::support::{
    RUN, TestResult, ask_source, ask_step, drain_until_completed, open, pending_ask, submit,
};

/// Given an open `Ask` suspension, When the store is closed and reopened,
/// Then public recovery re-issues the same ticket and the answer completes it.
#[test]
fn recovered_open_ask_keeps_its_ticket_and_the_answer_completes_the_run() -> TestResult {
    let directory = tempfile::tempdir()?;
    let workflow = vb_compile::compile_workflow(ask_source().as_bytes())?;
    let ask = ask_step(&workflow)?;
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
    assert!(
        journal
            .events_for_run_full(RUN)?
            .iter()
            .any(|event| matches!(event, JournalEvent::AskAnsweredEvent { .. }))
    );
    Ok(())
}

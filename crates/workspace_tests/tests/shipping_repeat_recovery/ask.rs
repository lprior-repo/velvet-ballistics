//! Repeat across Ask suspensions: counters, reopened trace and result taint.

use vb_core::{CompiledNodeKind as K, SlotValue, StepIdx, Taint};
use vb_runtime::{runtime::Runtime, shard::AskAnswer};

use crate::support::{
    ASK_SOURCE, RUN, TestResult, counter_sequence, open, repeat_nodes, replay, started, submission,
};

#[test]
fn repeat_ask_attempts_replay_counter_and_secret_result_taint() -> TestResult {
    let workflow = vb_compile::compile_workflow(ASK_SOURCE.as_bytes())?;
    let nodes = repeat_nodes(&workflow, 2)?;
    let submission = submission(ASK_SOURCE, &workflow)?;
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    journal.commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)?;
    runtime.recover_and_resume(RUN)?;
    let ask = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| matches!(node.kind, K::Ask { .. }).then_some(node.id))
        .ok_or("repeat ask node")?;
    for value in [73, 74] {
        let ticket = loop_ask(&mut runtime, ask)?;
        runtime.answer_ask(AskAnswer::new(
            ticket,
            ticket.answer_slot(),
            SlotValue::I64(value),
            Taint::DerivedFromSecret,
        ))?;
    }
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    drop(runtime);
    drop(journal);

    let (journal, _runtime) = open(directory.path())?;
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(started(&events, nodes.body), 2);
    assert_eq!(
        counter_sequence(&events, nodes.attempt)?,
        vec![(2, 0), (2, 1), (2, 2)]
    );
    let frame = replay(&events, &workflow)?
        .ok_or("ask repeat replay skipped")?
        .into_frame();
    assert_eq!(frame.pc(), nodes.terminal);
    assert_eq!(frame.read_slot(nodes.result)?, &SlotValue::I64(74));
    assert_eq!(frame.read_taint(nodes.result)?, Taint::DerivedFromSecret);
    Ok(())
}

fn loop_ask(runtime: &mut Runtime, ask: StepIdx) -> TestResult<vb_runtime::shard::AskTicket> {
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            return Ok(ticket);
        }
    }
    Err("repeat ask boundary not reached".into())
}

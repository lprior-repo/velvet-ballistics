//! Cold arena parity: a Repeat result that is a materialized list or object
//! survives a real journal drop, recovery and Ask resume.

use vb_core::{SlotValue, StepIdx, Taint};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_storage::JournalEvent as E;

use crate::support::{
    RUN, TestResult, counter_sequence, open, repeat_nodes, replay, started, submission,
};

const COLD_SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_cold\nwhen: {manual: {}}\nvars: {prompt: 'cold arena', payload: [5, 6]}\nsteps:\n  - id: seed\n    set: {output: base, value: '7'}\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: leaf\n          set: {output: leaf_out, value: '$vars.payload'}\n  - id: gate\n    ask: {prompt: '$vars.prompt'}\n  - id: done\n    finish: {result: '$step.r'}\n";
const OBJECT_SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_cold_object\nwhen: {manual: {}}\nvars: {prompt: 'cold object', payload: {count: 5}}\nsteps:\n  - id: seed\n    set: {output: base, value: '7'}\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: leaf\n          set: {output: leaf_out, value: '$vars.payload'}\n  - id: gate\n    ask: {prompt: '$vars.prompt'}\n  - id: done\n    finish: {result: '$step.r'}\n";

fn ask_ticket(runtime: &mut vb_runtime::runtime::Runtime, ask: StepIdx) -> TestResult<AskTicket> {
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            return Ok(ticket);
        }
    }
    Err("cold repeat ask boundary not reached".into())
}

fn cold_roundtrip(source: &str) -> TestResult {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let submission = submission(source, &workflow)?;
    let parts = workflow.to_parts();
    let ask = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Ask { .. }))
        .ok_or("cold ask node")?
        .id;
    let nodes = repeat_nodes(&workflow, 2)?;
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    journal.commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)?;
    runtime.recover_and_resume(RUN)?;
    ask_ticket(&mut runtime, ask)?;
    let live = runtime.inspect_run_state(RUN)?;
    let live_result = *live.frame.read_slot(nodes.result)?;
    let live_taint = live.frame.read_taint(nodes.result)?;
    let live_store = live.store.checkpoint_v1();
    assert!(matches!(
        live_result,
        SlotValue::List(_) | SlotValue::Object(_)
    ));
    assert_eq!(live_taint, Taint::Clean);
    let pending = journal.events_for_run_full(RUN)?;
    drop(runtime);
    drop(journal);

    let (journal, mut runtime) = open(directory.path())?;
    assert_eq!(journal.events_for_run_full(RUN)?, pending);
    match runtime.recover_product(RUN)? {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(product.boundary_kind(), RecoveredRunBoundaryKind::OpenAsk);
        }
        other => return Err(format!("expected resumable OpenAsk: {other:?}").into()),
    }
    runtime.recover_and_resume(RUN)?;
    runtime.tick_all()?;
    let restored = runtime.inspect_run_state(RUN)?;
    assert_eq!(restored.frame.read_slot(nodes.result)?, &live_result);
    assert_eq!(restored.frame.read_taint(nodes.result)?, live_taint);
    assert_eq!(restored.store.checkpoint_v1(), live_store);
    let ticket = runtime
        .lookup_pending_ask_ticket(RUN, ask)
        .ok_or("restored ask ticket")?;
    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(1),
        Taint::Clean,
    ))?;
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);

    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(started(&events, nodes.body), 2);
    assert_eq!(started(&events, nodes.control), 2);
    assert_eq!(
        counter_sequence(&events, nodes.attempt)?,
        vec![(2, 0), (2, 1), (2, 2)]
    );
    let finish = events
        .iter()
        .find_map(|event| match event {
            E::SlotWrittenEvent {
                slot,
                value: Some(bytes),
                extra: Some(extra),
                ..
            } if *slot == nodes.result => {
                Some((postcard::from_bytes::<SlotValue>(bytes).ok()?, extra))
            }
            _ => None,
        })
        .ok_or("repeat finish arena capture")?;
    assert_eq!(finish.0, live_result);
    assert!(matches!(
        vb_storage::slot_extra::decode_slot_written_extra(finish.1)
            .map_err(|error| format!("finish taint: {error:?}"))?,
        vb_storage::slot_extra::DecodedSlotWrittenExtra::Envelope(envelope)
            if envelope.taint == Taint::Clean
    ));
    assert!(
        replay(&events, &workflow)?.is_none(),
        "composite arena traces stay outside scalar replay"
    );
    Ok(())
}

#[test]
fn repeat_materialized_list_result_survives_cold_reopen_and_resume() -> TestResult {
    cold_roundtrip(COLD_SOURCE)
}

#[test]
fn repeat_materialized_object_result_survives_cold_reopen_and_resume() -> TestResult {
    cold_roundtrip(OBJECT_SOURCE)
}

use super::durable::{RUN, open, pending, submission};
use super::{K, SlotValue, TestResult, repeated, source_with};
use vb_core::{RuntimePolicy, StepState, Taint};
use vb_runtime::{recovery::RuntimeRecoveryProduct, shard::AskAnswer};
use vb_storage::JournalEvent as E;

#[test]
fn handled_dynamic_ask_prompt_error_recovers_at_each_later_ask_boundary() -> TestResult {
    let body = "- id: first\n  ask: {prompt: '$vars.prompt'}\n- id: second\n  on_error: {handler: fallback}\n  ask: {prompt: '$steps.first'}\n- id: fallback\n  ask: {prompt: '$vars.prompt'}\n";
    let yaml = source_with(
        "vars: {prompt: 'question'}\n",
        &repeated("outer", body),
        "$step.outer",
    );
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    let asks = workflow
        .to_parts()
        .nodes
        .iter()
        .filter_map(|node| matches!(node.kind, K::Ask { .. }).then_some(node.id))
        .collect::<Vec<_>>();
    assert_eq!(asks.len(), 3);
    let first = *asks.first().ok_or("first Ask")?;
    let second = *asks.get(1).ok_or("second Ask")?;
    let fallback = *asks.get(2).ok_or("fallback Ask")?;
    let directory = tempfile::tempdir()?;
    let (mut journal, mut runtime) = open(directory.path())?;
    journal.commit_run_submission(&submission(&yaml, &workflow)?, RuntimePolicy::Strict)?;
    runtime.recover_and_resume(RUN)?;
    for answer in [SlotValue::I64(73), SlotValue::I64(74)] {
        let ticket = pending(&mut runtime, first)?;
        runtime.answer_ask(AskAnswer::new(
            ticket,
            ticket.answer_slot(),
            SlotValue::Bool(true),
            Taint::Clean,
        ))?;
        let ticket = pending(&mut runtime, fallback)?;
        assert_eq!(runtime.lookup_pending_ask_ticket(RUN, second), None);
        let live = runtime.inspect_run_state(RUN)?;
        assert_eq!(live.frame.step_state(second)?, StepState::Failed);
        let before = live.frame.checkpoint_v1();
        let store = live.store.checkpoint_v1();
        drop(runtime);
        drop(journal);
        (journal, runtime) = open(directory.path())?;
        assert!(matches!(
            runtime.recover_product(RUN)?,
            RuntimeRecoveryProduct::Resumable(_)
        ));
        runtime.recover_and_resume(RUN)?;
        runtime.tick_all()?;
        assert_eq!(
            runtime.inspect_run_state(RUN)?.frame.checkpoint_v1(),
            before
        );
        assert_eq!(runtime.inspect_run_state(RUN)?.store.checkpoint_v1(), store);
        assert_eq!(
            runtime.lookup_pending_ask_ticket(RUN, fallback),
            Some(ticket)
        );
        runtime.answer_ask(AskAnswer::new(
            ticket,
            ticket.answer_slot(),
            answer,
            Taint::Clean,
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
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepFailed { step, .. } if *step == second))
            .count(),
        2
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, E::AskScheduledEvent { step, .. } if *step == second))
    );
    let result = events
        .iter()
        .find_map(|event| match event {
            E::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("final result")?;
    let value = events
        .iter()
        .rev()
        .find_map(|event| match event {
            E::SlotWrittenEvent {
                slot,
                value: Some(value),
                ..
            } if *slot == result => Some(value),
            _ => None,
        })
        .ok_or("final value")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(value)?,
        SlotValue::I64(74)
    );
    Ok(())
}

#[test]
fn failed_eval_keeps_older_output_without_a_new_write_receipt() -> TestResult {
    let body = "- id: divisor\n  ask: {prompt: '$vars.prompt'}\n- id: value\n  on_error: {handler: fallback}\n  set: {output: quotient, value: '1 / $steps.divisor'}\n- id: fallback\n  ask: {prompt: '$vars.prompt'}\n";
    let yaml = source_with(
        "vars: {prompt: 'question'}\n",
        &repeated("outer", body),
        "$step.outer",
    );
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = workflow.to_parts();
    let asks = parts
        .nodes
        .iter()
        .filter_map(|node| matches!(node.kind, K::Ask { .. }).then_some(node.id))
        .collect::<Vec<_>>();
    assert_eq!(asks.len(), 2);
    let first = *asks.first().ok_or("divisor Ask")?;
    let fallback = *asks.get(1).ok_or("fallback Ask")?;
    let failed = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::EvalExpr { .. }))
        .ok_or("EvalExpr")?;
    let output = failed.output.slot().ok_or("EvalExpr output")?;
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    journal.commit_run_submission(&submission(&yaml, &workflow)?, RuntimePolicy::Strict)?;
    runtime.recover_and_resume(RUN)?;
    let ticket = pending(&mut runtime, first)?;
    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(1),
        Taint::Clean,
    ))?;
    let ticket = pending(&mut runtime, fallback)?;
    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(73),
        Taint::Clean,
    ))?;
    let ticket = pending(&mut runtime, first)?;
    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(0),
        Taint::Clean,
    ))?;
    pending(&mut runtime, fallback)?;
    let frame = &runtime.inspect_run_state(RUN)?.frame;
    assert_eq!(frame.step_state(failed.id)?, StepState::Failed);
    assert_eq!(frame.read_slot(output)?, &SlotValue::I64(1));
    let events = journal.events_for_run_full(RUN)?;
    let replay = vb_storage::recovery::complete_scalar::replay_complete_scalar(&events, &workflow)?
        .ok_or("complete scalar trace")?;
    assert_eq!(replay.into_frame().checkpoint_v1(), frame.checkpoint_v1());
    let capture = events
        .iter()
        .find(|event| matches!(event, E::SlotWrittenEvent { slot, .. } if *slot == output))
        .ok_or("first successful output receipt")?
        .clone();
    let mut repeated_receipt = events.clone();
    let position = repeated_receipt
        .iter()
        .rposition(|event| matches!(event, E::StepFailed { step, .. } if *step == failed.id))
        .ok_or("second iteration failure")?;
    repeated_receipt.insert(position, capture);
    resequence(&mut repeated_receipt)?;
    let outcome =
        vb_storage::recovery::complete_scalar::replay_complete_scalar(&repeated_receipt, &workflow);
    let observed = match &outcome {
        Ok(Some(_)) => "accepted complete trace".to_owned(),
        Ok(None) => "outside scalar scope".to_owned(),
        Err(error) => error.to_string(),
    };
    assert!(
        matches!(outcome, Err(vb_storage::recovery::RecoveryError::ReplayDivergence { step, detail }) if step == failed.id && detail == "failed step cannot emit output capture"),
        "{observed}"
    );
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(
        runtime.inspect_run_state(RUN)?.frame.checkpoint_v1(),
        frame.checkpoint_v1()
    );
    Ok(())
}

fn resequence(events: &mut [E]) -> TestResult {
    let first = events.first().ok_or("first event")?.seq().get();
    for (index, event) in events.iter_mut().enumerate() {
        let next = first
            .checked_add(u64::try_from(index)?)
            .ok_or("event sequence overflow")?;
        // Continuation identity precedes the steps and keeps its original sequence.
        if matches!(event, E::RunContinuation { .. }) {
            assert_eq!(event.seq().get(), next);
            continue;
        }
        let seq = match event {
            E::RunAccepted { seq, .. }
            | E::RunAdmission { seq, .. }
            | E::RunAdmissionBudget { seq, .. }
            | E::CompatibilityDecisionEvent { seq, .. }
            | E::TenantAdmissionEvent { seq, .. }
            | E::PolicyDecisionEvent { seq, .. }
            | E::StepStarted { seq, .. }
            | E::SlotWrittenEvent { seq, .. }
            | E::StepSucceeded { seq, .. }
            | E::StepFailed { seq, .. }
            | E::AskScheduledEvent { seq, .. }
            | E::AskAnsweredEvent { seq, .. }
            | E::RunFinished { seq, .. } => seq,
            _ => return Err("event outside scalar test scope".into()),
        };
        *seq = vb_storage::EventSeq::new(next);
    }
    Ok(())
}

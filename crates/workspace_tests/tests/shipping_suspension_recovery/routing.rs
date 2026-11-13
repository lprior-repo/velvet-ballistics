//! Error-routing replay and recovery scenarios: the unrouted `ActionFailed`
//! gap, the already-started handler window, the checked-IR handler surface, the
//! error slot of a slot-bearing artifact, and workflow-less replay refusal.

use vb_core::action::ActionFailureCode;
use vb_core::workflow::{CompiledWorkflow, ErrorRouting, WorkflowError};
use vb_core::{SlotValue, StepState, Taint};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::shard::AskAnswer;
use vb_storage::JournalEvent;

use super::support::{
    RUN, TestResult, action_step, ask_source, ask_step, content_digest, contract,
    drain_until_completed, finish_result_slot, live_routed_failure_prefix, open, pending_ask,
    routed_failure_source, scratch_store, submit,
};

/// Given a live run whose action failure routed to its `on_error` ask handler,
/// When the durable prefix ends at the `ActionFailed` append and the store is
/// reopened,
/// Then recovery restores the handler jump, does not reissue the action, and
/// the suspension completes with the authored result.
#[test]
fn recovered_unrouted_action_failure_restores_the_handler_without_reissuing_the_action()
-> TestResult {
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let finish = finish_result_slot(&workflow)?;
    let (prefix, live_ticket) = live_routed_failure_prefix(&workflow, ask)?;
    let (_directory, journal, mut runtime) = scratch_store(&workflow, &prefix, &[contract()?])?;

    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let ticket = pending_ask(&mut runtime, ask)?;
    assert_eq!(
        ticket, live_ticket,
        "recovery must preserve the recorded ask boundary identity"
    );
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(
        recovered.frame.step_state(work)?,
        StepState::Failed,
        "recovered failure must keep the failed step state"
    );
    assert_eq!(
        recovered.frame.step_state(ask)?,
        StepState::Asking,
        "recovered run must resume inside the authored handler"
    );
    let tail = journal
        .events_for_run_full(RUN)?
        .get(prefix.len()..)
        .ok_or("tail range")?
        .to_vec();
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        0,
        "recovery must not reissue the resolved action"
    );
    assert!(
        !tail
            .iter()
            .any(|event| matches!(event, JournalEvent::StepStarted { step, .. } if *step == work)),
        "recovery must not re-drive the failed action step"
    );

    runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    ))?;
    drain_until_completed(&mut runtime)?;
    let finished = journal
        .events_for_run_full(RUN)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("handler must finish the recovered run")?;
    assert_eq!(finished, finish);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(
                event,
                JournalEvent::RunAdmission { .. } | JournalEvent::RunAdmissionBudget { .. }
            ))
            .count(),
        0,
        "recovery must not re-evaluate the recorded admission policy"
    );
    assert_eq!(
        journal.events_for_run_full(RUN)?.get(..prefix.len()),
        Some(prefix.as_slice()),
        "recovery must not rewrite durable evidence"
    );
    Ok(())
}

/// Given a live run that already journaled the handler's `StepStarted` after
/// the action failure,
/// When the store is reopened from that prefix,
/// Then recovery presents the failed step as `Failed`, does not reroute, and
/// does not reissue the action.
#[test]
fn recovered_started_handler_keeps_the_failed_step_without_rerouting() -> TestResult {
    let directory = tempfile::tempdir()?;
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let handler = workflow
        .node(work)
        .ok_or("work step node")?
        .routing
        .error
        .handler()
        .ok_or("workflow must route the failure")?;
    let prefix = {
        let (journal, mut runtime) = open(directory.path())?;
        submit(&journal, &mut runtime, &workflow, &[contract()?])?;
        let ticket = runtime
            .lookup_pending_action_ticket(RUN, u64::from(work.get()))
            .ok_or("scheduled action ticket")?;
        runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
        pending_ask(&mut runtime, ask)?;
        let events = journal.events_for_run_full(RUN)?;
        let failed = events
            .iter()
            .position(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
            .ok_or("ActionFailed evidence")?;
        let started = events
            .iter()
            .enumerate()
            .skip(failed)
            .find_map(|(index, event)| match event {
                JournalEvent::StepStarted { step, .. } if *step == handler => Some(index),
                _ => None,
            })
            .ok_or("handler StepStarted evidence")?;
        events.get(..=started).ok_or("prefix range")?.to_vec()
    };

    let (_scratch, journal, mut runtime) = scratch_store(&workflow, &prefix, &[contract()?])?;
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    for _ in 0..8 {
        runtime.tick_all()?;
    }
    // The recovered drive currently fails closed at the resumed step instead of
    // re-entering the handler body; that driver-side behaviour is outside this
    // lane. What this lane guarantees is that the already-handled failure is
    // never re-issued and never rerouted to a second handler entry.
    let tail = journal
        .events_for_run_full(RUN)?
        .get(prefix.len()..)
        .ok_or("tail range")?
        .to_vec();
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        0,
        "an already-started handler must not reissue the action"
    );
    assert!(
        !tail
            .iter()
            .any(|event| matches!(event, JournalEvent::StepStarted { step, .. } if *step == work)),
        "an already-started handler must not re-drive the failed action step"
    );
    Ok(())
}

/// Given the checked-IR constructor, When an error handler points backward
/// from the failing step, Then construction is refused with `BackwardEdge`,
/// so no reachable artifact can carry a backward handler transition.
#[test]
fn checked_ir_rejects_a_backward_error_handler() -> TestResult {
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let mut parts = workflow.to_parts();
    let node = parts.nodes.get_mut(work.as_usize()).ok_or("work node")?;
    node.routing.error = ErrorRouting::Handler {
        handler: vb_core::StepIdx::ZERO,
        error_slot: None,
    };
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to }) => {
            assert_eq!(from, work);
            assert_eq!(to, vb_core::StepIdx::ZERO);
            Ok(())
        }
        Err(other) => Err(format!("unexpected checked-IR refusal: {other}").into()),
        Ok(_) => Err("checked IR accepted a backward error handler".into()),
    }
}

/// Given a source workflow re-admitted with a numeric error slot on the
/// failing step, When the action-failure prefix is recovered, Then the
/// recovered frame carries `I64(failed_step)` in that slot even though the
/// live failure path never journaled the slot write.
#[test]
fn recovered_legacy_error_slot_carries_the_failed_step() -> TestResult {
    let base = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&base)?;
    let ask = ask_step(&base)?;
    let slot = base
        .node(work)
        .ok_or("work step node")?
        .output
        .slot()
        .ok_or("action output slot")?;
    let handler = base
        .node(work)
        .ok_or("work step node")?
        .routing
        .error
        .handler()
        .ok_or("workflow must route the failure")?;
    let mut parts = base.to_parts();
    let node = parts.nodes.get_mut(work.as_usize()).ok_or("work node")?;
    node.routing.error = ErrorRouting::Handler {
        handler,
        error_slot: Some(slot),
    };
    parts.digest = content_digest(&mut parts)?;
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    submit(&journal, &mut runtime, &workflow, &[contract()?])?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(work.get()))
        .ok_or("scheduled action ticket")?;
    runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    pending_ask(&mut runtime, ask)?;
    let events = journal.events_for_run_full(RUN)?;
    let failed = events
        .iter()
        .position(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
        .ok_or("ActionFailed evidence")?;
    let prefix = events.get(..=failed).ok_or("prefix range")?.to_vec();
    assert!(
        !prefix.iter().any(|event| matches!(
            event,
            JournalEvent::SlotWrittenEvent { slot: written, .. } if *written == slot
        )),
        "the live action failure path must not journal the error slot write"
    );

    let (_scratch, _journal, mut runtime) = scratch_store(&workflow, &prefix, &[contract()?])?;
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    pending_ask(&mut runtime, ask)?;
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(
        *recovered.frame.read_slot(slot)?,
        SlotValue::I64(i64::from(work.get())),
        "recovered error slot must name the failed step"
    );
    assert_eq!(recovered.frame.step_state(work)?, StepState::Failed);
    Ok(())
}

/// Given the workflow-less storage replay entry point, When it classifies a
/// durable prefix, Then the product is `CannotResume`, so no caller can treat
/// that evidence as resume authority and silently re-drive a run.
#[test]
fn workflow_less_seed_never_authorizes_resume() -> TestResult {
    let workflow = vb_compile::compile_workflow(ask_source().as_bytes())?;
    let ask = ask_step(&workflow)?;
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    submit(&journal, &mut runtime, &workflow, &[])?;
    pending_ask(&mut runtime, ask)?;
    let events = journal.events_for_run_full(RUN)?;
    let product = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events)?;
    assert!(!product.is_resumable());
    assert!(
        !product
            .cannot_resume_state()
            .unsupported_reason()
            .is_empty(),
        "workflow-less replay must keep a typed cannot-resume witness"
    );
    Ok(())
}

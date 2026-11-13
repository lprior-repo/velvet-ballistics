//! Crash-prefix recovery for handler routing: error-slot parity and the
//! legacy wrapper surface.

use vb_core::action::ActionFailureCode;
use vb_core::workflow::{
    CompiledNodeKind, CompiledWorkflow, ErrorRouting, NodeContinuation, NodeOutput, WorkflowParts,
};
use vb_core::{SlotIdx, SlotValue, StepIdx, StepState, Taint};
use vb_storage::JournalEvent;

use super::support::{
    RUN, TestResult, action_step, ask_step, content_digest, contract, drain_until_completed,
    finish_result_slot, open, pending_ask, routed_failure_source, scratch_store, submit,
};

/// Given an artifact routing the failure to the ask with an error slot,
/// When the prefix ends at the handler's `StepStarted`, recovery restores
/// `I64(failed)` with `Clean` taint and resumes without reissuing.
#[test]
fn direct_handler_started_prefix_restores_error_slot() -> TestResult {
    let base = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&base)?;
    let ask = ask_step(&base)?;
    let slot = action_slot(&base, work)?;
    let workflow = with_route(&base, work, ask, Some(slot))?;
    let (live, _) = live_failure(&workflow, work)?;
    let prefix = truncate_at(&live, &[(&ask, 1)])?;

    let (_store, journal, mut runtime) = scratch_store(&workflow, &prefix, &[contract()?])?;
    runtime.recover_and_resume(RUN)?;
    let ticket = pending_ask(&mut runtime, ask)?;
    assert_restored(&runtime, work, ask, slot)?;
    let events = journal.events_for_run_full(RUN)?;
    let tail = events.get(prefix.len()..).ok_or("tail range")?;
    assert_no_reissue(tail, work)?;
    finish_with_answer(&mut runtime, &journal, &workflow, ticket)?;
    Ok(())
}

/// Reviewed geometry W(1) < H(2) < failed(4), reachable through the wrapper's
/// fall-through; the runtime accepts it, resolves the backward handler, and
/// both crash prefixes cold-reopen to the same repaired route.
#[test]
fn wrapper_before_handler_prefixes_restore_and_resume() -> TestResult {
    let (workflow, failed, handler, slot) = wrapper_before_geometry()?;
    assert!(
        handler.get() < failed.get(),
        "reviewed geometry is backward"
    );
    assert_eq!(
        vb_runtime::shard::helpers::find_error_handler_for_failure(&workflow, failed),
        Some((handler, Some(slot))),
        "the wrapper fallback resolves the earlier handler"
    );
    let (live, _) = live_failure(&workflow, failed)?;
    assert_eq!(
        live.iter()
            .filter(|event| matches!(
                event,
                JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.step.get() == failed.get()
            ))
            .count(),
        1,
        "the accepted run must schedule the canonical ticket for the failing step"
    );
    assert!(
        !live
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFailedEvent { .. })),
        "the wrapper route must not terminalize the live run"
    );
    let prefixes = [
        truncate_at(&live, &[])?,
        truncate_at(&live, &[(&handler, 1)])?,
    ];
    for prefix in prefixes {
        let (_store, journal, mut runtime) = scratch_store(&workflow, &prefix, &[contract()?])?;
        runtime.recover_and_resume(RUN)?;
        let ticket = pending_ask(&mut runtime, handler)?;
        assert_restored(&runtime, failed, handler, slot)?;
        let events = journal.events_for_run_full(RUN)?;
        let tail = events.get(prefix.len()..).ok_or("tail range")?;
        assert_no_reissue(tail, failed)?;
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    JournalEvent::StepStarted { step, .. } if step.get() == handler.get()
                ))
                .count(),
            1,
            "the handler entry must be journaled exactly once"
        );
        finish_with_answer(&mut runtime, &journal, &workflow, ticket)?;
    }
    Ok(())
}

fn live_failure(
    workflow: &CompiledWorkflow,
    work: StepIdx,
) -> TestResult<(Vec<JournalEvent>, std::sync::Arc<vb_storage::FjallJournal>)> {
    let store = tempfile::tempdir()?;
    let (journal, mut runtime) = open(store.path())?;
    submit(&journal, &mut runtime, workflow, &[contract()?])?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(work.get()))
        .ok_or("scheduled action ticket")?;
    runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    let ask = ask_step(workflow)?;
    pending_ask(&mut runtime, ask)?;
    let events = journal.events_for_run_full(RUN)?;
    Ok((events, journal))
}

fn wrapper_before_geometry() -> TestResult<(CompiledWorkflow, StepIdx, StepIdx, SlotIdx)> {
    let base = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&base)?;
    let handle = StepIdx::new(2);
    let failed = StepIdx::new(4);
    let finish = StepIdx::new(5);
    let slot = action_slot(&base, work)?;
    let ask_node = base.node(ask_step(&base)?).ok_or("ask node")?.clone();
    let mut parts = base.to_parts();
    let mut template = parts.nodes.get(work.as_usize()).ok_or("work node")?.clone();
    let mut finish_node = parts.nodes.get(4).ok_or("finish node")?.clone();
    finish_node.id = finish;
    let wrapper = parts.nodes.get_mut(work.as_usize()).ok_or("wrapper node")?;
    wrapper.output = NodeOutput::Discard;
    wrapper.kind = CompiledNodeKind::ErrorHandler {
        body: failed,
        handler: handle,
        error_slot: vb_core::workflow::ErrorSink::Slot(slot),
    };
    wrapper.routing.next = NodeContinuation::Step(handle);
    wrapper.routing.error = ErrorRouting::None;
    let handler = parts
        .nodes
        .get_mut(handle.as_usize())
        .ok_or("handler node")?;
    *handler = ask_node;
    handler.id = handle;
    handler.routing.next = NodeContinuation::Step(StepIdx::new(3));
    parts.nodes.get_mut(3).ok_or("resume node")?.routing.next = NodeContinuation::Step(finish);
    template.id = failed;
    template.routing.next = NodeContinuation::Step(finish);
    template.routing.error = ErrorRouting::None;
    let failing = parts
        .nodes
        .get_mut(failed.as_usize())
        .ok_or("failing slot")?;
    *failing = template;
    let mut nodes = parts.nodes.into_vec();
    nodes.push(finish_node);
    parts.nodes = nodes.into_boxed_slice();
    let workflow = rebuild(parts)?;
    Ok((workflow, failed, handle, slot))
}

fn with_route(
    base: &CompiledWorkflow,
    work: StepIdx,
    handler: StepIdx,
    slot: Option<SlotIdx>,
) -> TestResult<CompiledWorkflow> {
    let mut parts = base.to_parts();
    parts
        .nodes
        .get_mut(work.as_usize())
        .ok_or("work node")?
        .routing
        .error = ErrorRouting::Handler {
        handler,
        error_slot: slot,
    };
    rebuild(parts)
}

fn rebuild(mut parts: WorkflowParts) -> TestResult<CompiledWorkflow> {
    parts.digest = content_digest(&mut parts)?;
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn action_slot(workflow: &CompiledWorkflow, work: StepIdx) -> TestResult<SlotIdx> {
    workflow
        .node(work)
        .ok_or("work node")?
        .output
        .slot()
        .ok_or_else(|| String::from("action output slot").into())
}

/// Truncates the live journal at the requested `n`th `StepStarted(step)` for
/// each step, or at the last `ActionFailed` when no start is requested.
fn truncate_at(
    events: &[JournalEvent],
    starts: &[(&StepIdx, usize)],
) -> TestResult<Vec<JournalEvent>> {
    let mut end = None;
    for (step, count) in starts {
        let index = events
            .iter()
            .enumerate()
            .filter(|(_, event)| {
                matches!(event, JournalEvent::StepStarted { step: started, .. } if started == *step)
            })
            .nth(count.saturating_sub(1))
            .map(|(index, _)| index)
            .ok_or("StepStarted evidence")?;
        end = Some(end.map_or(index, |current: usize| current.max(index)));
    }
    let end = end.unwrap_or(
        events
            .iter()
            .rposition(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
            .ok_or("ActionFailed evidence")?,
    );
    Ok(events.get(..=end).ok_or("prefix range")?.to_vec())
}

fn assert_restored(
    runtime: &vb_runtime::runtime::Runtime,
    failed: StepIdx,
    handler: StepIdx,
    slot: SlotIdx,
) -> TestResult {
    let recovered = runtime.inspect_run_state(RUN)?;
    assert_eq!(
        *recovered.frame.read_slot(slot)?,
        SlotValue::I64(i64::from(failed.get())),
        "recovery must restore I64(failed) into the error slot"
    );
    assert_eq!(
        recovered.frame.read_taint(slot)?,
        Taint::Clean,
        "the restored error slot must carry clean taint"
    );
    assert_eq!(recovered.frame.step_state(failed)?, StepState::Failed);
    assert_eq!(recovered.frame.step_state(handler)?, StepState::Asking);
    Ok(())
}

fn assert_no_reissue(tail: &[JournalEvent], work: StepIdx) -> TestResult {
    assert!(
        !tail.iter().any(|event| matches!(
            event,
            JournalEvent::RunFailedEvent { .. } | JournalEvent::RetryCheckpoint { .. }
        )),
        "a recoverable handler prefix must not terminalize the run"
    );
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        0,
        "recovery must not reissue the action"
    );
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::StepStarted { step, .. } if step.get() == work.get()))
            .count(),
        0,
        "recovery must not re-drive the failed action step"
    );
    assert_eq!(
        tail.iter()
            .filter(|event| matches!(event, JournalEvent::AskScheduledEvent { .. }))
            .count(),
        1,
        "the handler ask must suspend exactly once"
    );
    Ok(())
}

fn finish_with_answer(
    runtime: &mut vb_runtime::runtime::Runtime,
    journal: &vb_storage::FjallJournal,
    workflow: &CompiledWorkflow,
    ticket: vb_runtime::shard::AskTicket,
) -> TestResult {
    runtime.answer_ask(vb_runtime::shard::AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    ))?;
    drain_until_completed(runtime)?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let finished = journal
        .events_for_run_full(RUN)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("recovered handler must finish the run")?;
    assert_eq!(finished, finish_result_slot(workflow)?);
    Ok(())
}

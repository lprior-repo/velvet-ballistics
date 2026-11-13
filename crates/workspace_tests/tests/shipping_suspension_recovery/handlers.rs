//! Checked-IR handler surface and workflow-less replay classification.

use vb_core::action::ActionFailureCode;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ErrorRouting, WorkflowError};
use vb_core::{SlotValue, StepState};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_storage::JournalEvent;

use super::support::{
    RUN, TestResult, action_step, ask_source, ask_step, content_digest, contract, open,
    pending_ask, routed_failure_source, scratch_store, submit,
};

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

/// Given a source workflow re-admitted with a legacy `ErrorHandler` wrapper
/// whose body is the failing step and whose handler is an earlier step, When
/// the parts are checked, Then the constructor refuses the wrapper with
/// `BackwardEdge`, so no reachable artifact can carry a backward wrapper
/// handler for recovery to repair.
#[test]
fn checked_ir_rejects_a_legacy_backward_wrapper_handler() -> TestResult {
    let base = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&base)?;
    let handler = base
        .node(work)
        .ok_or("work step node")?
        .routing
        .error
        .handler()
        .ok_or("workflow must route the failure")?;
    let mut parts = base.to_parts();
    let mut nodes = parts.nodes.into_vec();
    let wrapper = vb_core::StepIdx::new(u16::try_from(nodes.len()).map_err(|_| "node overflow")?);
    let mut node = nodes.get(handler.as_usize()).ok_or("handler node")?.clone();
    node.id = wrapper;
    node.kind = CompiledNodeKind::ErrorHandler {
        body: work,
        handler: work,
        error_slot: vb_core::workflow::ErrorSink::Discard,
    };
    node.routing.next = vb_core::workflow::NodeContinuation::End;
    nodes.push(node);
    parts.nodes = nodes.into_boxed_slice();
    let work_node = parts.nodes.get_mut(work.as_usize()).ok_or("work node")?;
    work_node.routing.error = ErrorRouting::Handler {
        handler: wrapper,
        error_slot: None,
    };
    parts.digest = content_digest(&mut parts)?;
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to }) => {
            assert_eq!(from, wrapper, "the wrapper's handler edge is the offender");
            assert_eq!(
                to, work,
                "the legacy wrapper handler points at the failed step"
            );
            Ok(())
        }
        Err(other) => Err(format!("unexpected checked-IR refusal: {other}").into()),
        Ok(_) => Err("checked IR accepted a legacy backward wrapper handler".into()),
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
    let workflow = vb_compile::compile_workflow(routed_failure_source().as_bytes())?;
    let work = action_step(&workflow)?;
    let ask = ask_step(&workflow)?;
    let store = tempfile::tempdir()?;
    let (journal, mut runtime) = open(store.path())?;
    submit(&journal, &mut runtime, &workflow, &[contract()?])?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(work.get()))
        .ok_or("scheduled action ticket")?;
    runtime.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    pending_ask(&mut runtime, ask)?;
    let failed_events = journal.events_for_run_full(RUN)?;
    let suspended = vb_compile::compile_workflow(ask_source().as_bytes())?;
    let suspended_ask = ask_step(&suspended)?;
    let store = tempfile::tempdir()?;
    let (journal, mut runtime) = open(store.path())?;
    submit(&journal, &mut runtime, &suspended, &[])?;
    pending_ask(&mut runtime, suspended_ask)?;
    let suspended_events = journal.events_for_run_full(RUN)?;
    for (label, events) in [
        ("suspended ask", suspended_events),
        ("failed action routed to a handler", failed_events),
    ] {
        let product = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events)?;
        assert!(!product.is_resumable(), "{label}: must not be resumable");
        assert!(
            !product
                .cannot_resume_state()
                .unsupported_reason()
                .is_empty(),
            "{label}: workflow-less replay must keep a typed cannot-resume witness"
        );
    }
    Ok(())
}

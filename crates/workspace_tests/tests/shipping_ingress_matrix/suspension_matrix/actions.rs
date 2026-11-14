//! `do` action completion, duplicate rejection, and typed failure scenarios.

#![forbid(unsafe_code)]

use vb_core::action::{
    ActionContract, ActionFailureCode, ActionName, ActionOutputReady, Idempotency, RetrySafety,
    SideEffect,
};
use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};

use crate::support::{
    Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};
use crate::suspension_matrix::HEADER;

fn action_contract(action: vb_core::ids::ActionId) -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("shipping-matrix-action").map_err(|error| error.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

#[test]
fn do_action_survives_cold_reopen_completes_and_rejects_duplicate() -> TestResult {
    let yaml = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '100'}}\n  - id: act\n    do: {{action: '0', input: '$step.seed'}}\n  - id: done\n    finish: {{result: '$step.act'}}\n"
    );
    let workflow = compile(&yaml)?;
    let do_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .map(|node| node.id)
        .ok_or("missing Do node")?;
    let contract = action_contract(vb_core::ids::ActionId::new(0))?;
    let contracts: Box<[ActionContract]> = Box::from([contract]);
    let dir = tempfile::tempdir()?;
    let run = RunId::new(104);
    let output_slot = {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(
            &open.journal,
            &workflow,
            RuntimePolicy::Strict,
            contracts.as_ref(),
        )?;
        open.runtime.submit_direct_with_grants_and_contracts(
            run,
            workflow.clone(),
            vb_core::CapabilitySet::empty(),
            contracts.clone(),
        )?;
        let _ticket = crate::support::tick_until_action(
            &mut open.runtime,
            run,
            u64::from(do_step.get()),
            64,
        )?;
        let durable = events(&open.journal, run)?;
        durable
            .iter()
            .find_map(|event| match event {
                vb_storage::JournalEvent::ActionScheduledTicket { output, ticket, .. }
                    if ticket.run == run =>
                {
                    Some(*output)
                }
                _ => None,
            })
            .ok_or("missing ActionScheduledTicket output")?
    };

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let product = open.runtime.recover_product(run)?;
    match &product {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::PendingAction
            );
        }
        other => return Err(format!("do boundary recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    open.runtime.tick_all()?;
    let ticket = open
        .runtime
        .lookup_pending_action_ticket(run, u64::from(do_step.get()))
        .ok_or("recovered ticket missing")?;
    let encoded_len = u32::try_from(postcard::to_allocvec(&SlotValue::I64(100))?.len())?;
    let output = ActionOutputReady {
        output_slot,
        value: SlotValue::I64(100),
        taint: Taint::Clean,
        encoded_len,
    };
    open.runtime.complete_action_with_output(ticket, output)?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    assert_eq!(
        crate::support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket.run == run
        )),
        1
    );
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(100));
    assert_eq!(result.taint, Taint::Clean);
    assert_ordering(&durable, run)?;

    let duplicate = open.runtime.complete_action_with_output(
        ticket,
        ActionOutputReady {
            output_slot,
            value: SlotValue::I64(100),
            taint: Taint::Clean,
            encoded_len,
        },
    );
    if duplicate.is_ok() {
        let applied = open.runtime.tick_all();
        assert!(
            matches!(
                applied,
                Err(vb_runtime::RuntimeError::RunNotFound
                    | vb_runtime::RuntimeError::InvalidActionCompletion)
            ),
            "duplicate completion must be rejected when applied: {applied:?}"
        );
    }
    assert_eq!(
        crate::support::count_where(&events(&open.journal, run)?, |event| matches!(
            event,
            vb_storage::JournalEvent::ActionCompletedEnvelope { .. }
        )),
        1
    );
    Ok(())
}

#[test]
fn failed_action_routes_typed_failure_and_terminal() -> TestResult {
    let yaml = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '100'}}\n  - id: act\n    do: {{action: '0', input: '$step.seed'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let workflow = compile(&yaml)?;
    let do_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .map(|node| node.id)
        .ok_or("missing Do node")?;
    let contracts: Box<[ActionContract]> =
        Box::from([action_contract(vb_core::ids::ActionId::new(0))?]);
    let dir = tempfile::tempdir()?;
    let run = RunId::new(105);
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(
        &open.journal,
        &workflow,
        RuntimePolicy::Strict,
        contracts.as_ref(),
    )?;
    open.runtime.submit_direct_with_grants_and_contracts(
        run,
        workflow.clone(),
        vb_core::CapabilitySet::empty(),
        contracts,
    )?;
    let ticket =
        crate::support::tick_until_action(&mut open.runtime, run, u64::from(do_step.get()), 64)?;
    open.runtime.fail_action(
        ticket,
        vb_core::action::ActionFailure::from(ActionFailureCode::InvalidInput),
    )?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    assert_eq!(
        crate::support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::ActionFailedEvent { run: actual, .. } if *actual == run
        )),
        1
    );
    let counters = open.runtime.counters_snapshot();
    assert_eq!(counters.runs_failed, 1);
    assert_eq!(counters.runs_completed, 0);
    Ok(())
}

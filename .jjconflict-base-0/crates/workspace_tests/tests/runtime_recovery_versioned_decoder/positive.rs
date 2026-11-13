use super::*;
use std::sync::Arc;

fn do_step(workflow: &CompiledWorkflow) -> TestResult<vb_core::StepIdx> {
    let step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Do { .. }))
        .ok_or("Do node")?
        .id;
    Ok(step)
}

fn submit_until_action(
    journal: Arc<vb_storage::FjallJournal>,
    workflow: CompiledWorkflow,
) -> TestResult<vb_core::action::ActionTicket> {
    let contracts = Box::new([contract()?]);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        contracts.as_ref(),
    )?;
    let mut runtime = open(journal.clone());
    runtime.submit_direct_with_grants_and_contracts(
        RUN,
        workflow.clone(),
        vb_core::CapabilitySet::empty(),
        contracts,
    )?;
    runtime.tick_all()?;
    let step = do_step(&workflow)?;
    let ticket = runtime
        .lookup_pending_action_ticket(RUN, u64::from(step.get()))
        .ok_or("pending ticket")?;
    Ok(ticket)
}

pub(super) fn check(workflow: CompiledWorkflow) -> TestResult<()> {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let ticket = submit_until_action(journal.clone(), workflow.clone())?;
    let mut recovered = open(journal.clone());
    let before = postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?;
    assert_eq!(recovered.recover_and_resume(RUN), Ok(()));
    assert_eq!(
        postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?,
        before
    );
    recovered.tick_all()?;
    let step = do_step(&workflow)?;
    assert_eq!(
        recovered.lookup_pending_action_ticket(RUN, u64::from(step.get())),
        Some(ticket)
    );
    assert_eq!(recovered.counters_snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn valid_v1_artifact_recovered() -> TestResult<()> {
    let workflow = fixture::action_workflow()?;
    assert_eq!(workflow.symbol_payloads(), None);
    check(workflow)
}

#[test]
fn valid_v2_artifact_recovered() -> TestResult<()> {
    let workflow = vb_compile::compile_workflow(V2_SOURCE)?;
    match workflow.symbol_payloads() {
        Some(payloads) if !payloads.is_empty() => {}
        other => return Err(format!("expected non-empty payloads, got {:?}", other).into()),
    }
    check(workflow)
}

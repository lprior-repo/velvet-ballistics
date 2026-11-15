use super::{RUN, TestResult, cut, runtime};
use std::{
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use vb_core::action::ActionName;
use vb_core::{
    ActionContract, ActionTicket, CapabilitySet, CompiledWorkflow, RuntimePolicy, SlotValue, Taint,
    ValueStore, WorkflowDigest,
};
use vb_runtime::{
    journal::{SharedRuntimeJournal, StorageRuntimeJournal},
    runtime::Runtime,
};
use vb_storage::{
    FjallJournal, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
};

pub(super) fn source(handler: bool, retry: Option<(u16, &str)>) -> String {
    let route = if handler {
        "    on_error: {handler: observe}\n"
    } else {
        ""
    };
    let retry = retry.map_or_else(String::new, |(attempts, delay)| {
        format!("    try_again: {{max_attempts: {attempts}, delay: '{delay}'}}\n")
    });
    format!(
        "version: velvet-ballistics/v1\nname: immediate_retry_containment\nwhen: {{manual: {{}}}}\ninputs: {{payload: any}}\nsteps:\n  - id: work\n{route}{retry}    do: {{action: '0', input: '$input.payload'}}\n  - id: observe\n    ask: {{prompt: '$input.payload'}}\n  - id: done\n    finish: {{result: '$input.payload'}}\n"
    )
}

pub(super) fn receipt(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    artifact: WorkflowDigest,
) -> TestResult<(bool, Vec<u8>)> {
    match journal.retained_run_inputs_for_recovery(RUN, artifact, workflow)? {
        Some(vb_storage::RetainedRunInputs::Scalar(record)) => {
            Ok((false, postcard::to_allocvec(&record)?))
        }
        Some(vb_storage::RetainedRunInputs::Owned(record)) => {
            Ok((true, postcard::to_allocvec(&record)?))
        }
        None => Err("retained input receipt absent".into()),
    }
}

pub(super) fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: vb_core::ActionId::new(0),
        name: ActionName::new("immediate-retry-containment")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

pub(super) fn pending(
    path: &Path,
    handler: bool,
    retry: Option<(u16, &str)>,
    owned: bool,
    crashing: bool,
) -> TestResult<(Arc<FjallJournal>, Runtime, ActionTicket)> {
    pending_at_failure(path, handler, retry, owned, crashing.then_some(1))
}

pub(super) fn pending_at_failure(
    path: &Path,
    handler: bool,
    retry: Option<(u16, &str)>,
    owned: bool,
    failure_to_cut: Option<u16>,
) -> TestResult<(Arc<FjallJournal>, Runtime, ActionTicket)> {
    pending_with_adapter(
        path,
        handler,
        retry,
        owned,
        |journal| match failure_to_cut {
            Some(failure) => Arc::new(cut::CutJournal::new(journal, failure)),
            None => StorageRuntimeJournal::shared_strict(journal),
        },
    )
}

pub(super) fn pending_with_adapter(
    path: &Path,
    handler: bool,
    retry: Option<(u16, &str)>,
    owned: bool,
    adapter: impl FnOnce(Arc<FjallJournal>) -> SharedRuntimeJournal,
) -> TestResult<(Arc<FjallJournal>, Runtime, ActionTicket)> {
    let source = source(handler, retry);
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let contract = contract()?;
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into());
    if owned {
        owned_submission(&journal, &workflow, &source, source_digest, &contract)?;
    } else {
        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Strict,
            &[contract.clone()],
        )?;
    }
    let mut live = runtime(adapter(journal.clone()))?;
    if owned {
        live.activate_owned_submission(RUN, source_digest)?;
    } else {
        let slot = workflow.input_slots().first().ok_or("input missing")?.slot;
        live.submit_direct_with_inputs_grants_and_contracts(
            RUN,
            workflow,
            Box::from([(slot, SlotValue::I64(42))]),
            CapabilitySet::empty(),
            Box::from([contract]),
        )?;
    }
    let issued = action(&mut live)?;
    Ok((journal, live, issued))
}

fn owned_submission(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    source: &str,
    digest: WorkflowDigest,
    contract: &ActionContract,
) -> TestResult {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list =
        store.insert_list_with_taint(Box::from([SlotValue::I64(42)]), Box::from([Taint::Clean]))?;
    let slot = workflow.input_slots().first().ok_or("input missing")?.slot;
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::from([(slot, SlotValue::List(list), Taint::Clean)]),
        store.checkpoint_v1(),
        workflow,
    )?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
    )?;
    let record = OwnedRunInputsRecordV1::new(RUN, artifact.digest, digest, &bundle, workflow)?;
    let submission = RunSubmission::owned(
        WorkflowSourceRecord {
            digest,
            source: source.as_bytes().to_vec(),
        },
        artifact,
        record,
        RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: u64::try_from(
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            )?,
        },
    )?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    Ok(())
}

pub(super) fn action(live: &mut Runtime) -> TestResult<ActionTicket> {
    for _ in 0..16 {
        live.tick_all()?;
        if let Some(ticket) = live.lookup_pending_action_ticket(RUN, 0) {
            return Ok(ticket);
        }
    }
    Err("actual pending action absent".into())
}

pub(super) fn ask(live: &mut Runtime) -> TestResult {
    for _ in 0..16 {
        live.tick_all()?;
        if live
            .lookup_pending_ask_ticket(RUN, vb_core::StepIdx::new(1))
            .is_some()
        {
            return Ok(());
        }
    }
    Err("handler Ask absent".into())
}

pub(super) fn crashed(
    path: &Path,
    handler: bool,
    resumed: bool,
    owned: bool,
) -> TestResult<(Arc<FjallJournal>, ActionTicket)> {
    super::historical::prefix(path, handler, resumed, owned, 1)
}

pub(super) fn refusal_snapshot(
    live: &Runtime,
    issued: ActionTicket,
) -> TestResult<impl FnOnce(&Runtime) -> TestResult + use<>> {
    let state = live.inspect_run_state(RUN)?.clone();
    let pending = live.pending_boundary_snapshot(8);
    let counters = live.counters_snapshot();
    let quota = live.usage_snapshot();
    let trace = live.list_events(RUN)?;
    Ok(move |live: &Runtime| {
        assert_eq!(live.inspect_run_state(RUN)?, &state);
        assert_eq!(
            live.lookup_pending_action_ticket(RUN, u64::from(issued.step.get())),
            Some(issued)
        );
        assert_eq!(live.pending_boundary_snapshot(8), pending);
        assert_eq!(live.counters_snapshot(), counters);
        assert_eq!(live.usage_snapshot(), quota);
        assert_eq!(live.list_events(RUN)?, trace);
        Ok(())
    })
}

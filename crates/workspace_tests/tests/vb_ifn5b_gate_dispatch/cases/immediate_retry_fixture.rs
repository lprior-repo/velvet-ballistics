use super::*;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{CapabilitySet, Taint, ValueStore};
use vb_storage::{
    ExecutorCatalogSubmissionContext, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: registered_immediate_retry\nwhen: {manual: {}}\ninputs: {payload: any}\nsteps:\n  - id: work\n    try_again: {max_attempts: 3, delay: '0ms'}\n    do: {action: '1', input: '$input.payload'}\n  - id: observe\n    do: {action: '0', input: '$step.work'}\n  - id: done\n    finish: {result: '$step.observe'}\n";

pub(super) fn source_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(SOURCE).into())
}

pub(super) fn submit(
    journal: &FjallJournal,
    catalog: &ExecutorCatalog,
    run: RunId,
) -> Result<vb_storage::AcceptedArtifact, Box<dyn std::error::Error>> {
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    let contract = contract()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let artifact = catalog.submit_artifact(
        journal,
        &workflow,
        RuntimePolicy::Strict,
        &[preceding_contract()?, contract],
        &[pack],
    )?;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let input = workflow
        .input_slots()
        .first()
        .ok_or("compiled input absent")?
        .slot;
    let bundle = OwnedRootBundle::new(
        Box::from([(input, SlotValue::I64(42), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let source = WorkflowSourceRecord {
        digest: source_digest(),
        source: SOURCE.to_vec(),
    };
    let inputs =
        OwnedRunInputsRecordV1::new(run, artifact.digest, source.digest, &bundle, &workflow)?;
    let header = RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?,
    };
    let grants = CapabilitySet::from_grants(Box::from([Capability::new(
        "velvet.register.write".into(),
        ActionId::new(1),
    )]));
    let context = ExecutorCatalogSubmissionContext::new(catalog, grants);
    let submission = RunSubmission::owned_with_executor_catalog(
        source,
        artifact.clone(),
        inputs,
        header,
        context,
    )?;
    journal.commit_run_submission_with_executor_catalog(
        &submission,
        RuntimePolicy::Strict,
        catalog,
    )?;
    crate::issued_run_artifact::read(journal, run, &artifact)
}

pub(super) fn runtime(
    journal: Arc<FjallJournal>,
    catalog: &ExecutorCatalog,
) -> Result<Runtime, Box<dyn std::error::Error>> {
    let mut config = crate::common::runtime_fixture::strict_config();
    config.command_queue_capacity = 1;
    config.step_budget_per_tick = 16;
    let storage =
        StorageRuntimeJournal::shared_strict_with_catalog(journal, Arc::new(catalog.clone()));
    let mut runtime = Runtime::new(std::num::NonZeroUsize::MIN, config, storage)?;
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action: ActionId::new(1),
            code: DecisionCode::Allowed,
        },
    )?;
    Ok(runtime)
}

pub(super) fn pending(
    runtime: &mut Runtime,
    run: RunId,
    attempt: u16,
) -> Result<ActionTicket, Box<dyn std::error::Error>> {
    for _ in 0..16 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, 0)
            && ticket.attempt == attempt
        {
            return Ok(ticket);
        }
    }
    Err(format!("actual registered attempt {attempt} absent").into())
}

pub(super) fn rows(
    path: &Path,
) -> Result<Vec<(String, Vec<u8>, Vec<u8>)>, Box<dyn std::error::Error>> {
    let database = fjall::Database::builder(path).open()?;
    let mut rows = Vec::new();
    for name in [
        "register_targets_v1",
        "register_receipts_v1",
        "register_metadata_v1",
    ] {
        let keyspace = database.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

pub(super) fn assert_state(
    runtime: &Runtime,
    journal: &FjallJournal,
    run: RunId,
    mut expected: vb_runtime::shard::types::RunState,
) -> TestResult {
    let events = journal.events_for_run_full(run)?;
    let actual = runtime.inspect_run_state(run)?;
    assert_eq!(
        expected.registered_execution,
        vb_runtime::shard::types::RegisteredExecutionMode::Live
    );
    assert_eq!(
        actual.registered_execution,
        vb_runtime::shard::types::RegisteredExecutionMode::ReconcileOnly
    );
    let origin = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunBudgetOriginEvent { started_millis, .. } => Some(*started_millis),
            _ => None,
        })
        .ok_or("actual durable wall origin absent")?;
    assert_eq!(actual.run_started_millis, Some(origin));
    let charged = events.iter().try_fold(
        0_u64,
        |total, event| -> Result<u64, Box<dyn std::error::Error>> {
            total
                .checked_add(u64::try_from(postcard::to_allocvec(event)?.len())?)
                .ok_or_else(|| "journal accounting overflow".into())
        },
    )?;
    assert_eq!(actual.journal_bytes_written, charged);
    expected.run_started_millis = actual.run_started_millis;
    expected.journal_bytes_written = actual.journal_bytes_written;
    expected.registered_execution = actual.registered_execution;
    assert_eq!(actual, &expected);
    Ok(())
}

pub(super) fn warm_retry_grant(
    runtime: &mut Runtime,
    catalog: &ExecutorCatalog,
    artifact: &vb_storage::AcceptedArtifact,
    ticket: ActionTicket,
    approval: &BoundApprovalReceipt,
    path: &Path,
    expected: &RegisterReceipt,
) -> TestResult {
    let register = catalog.issue_register_grant(artifact, ticket, scope())?;
    let scoped = catalog.issue_scoped_grant_for_bound_gate(
        artifact,
        &register,
        super::super::gate_cold_recovery::scoped_params(ticket.action, approval.digest)?,
        approval,
    )?;
    let dispatch = super::super::gate_cold_recovery::dispatch_context(artifact, ticket)?;
    let mut adapter = FjallRegister::open(path, 8)?;
    runtime.inspect_run(ticket.run, 94_243)?;
    assert!(matches!(
        runtime.execute_registered_action(
            &mut adapter,
            ticket.run,
            u64::from(ticket.step.get()),
            &register,
            vb_core::ScopedDispatch::new(&scoped, &dispatch, &[]).with_gate_approval(approval)
        ),
        Err(RegisteredExecutionError::Runtime(RuntimeError::QueueFull))
    ));
    assert_eq!(&adapter.reconcile(&register, ticket, 42)?, expected);
    assert_eq!(adapter.effect_count()?, 1);
    runtime.tick_all()?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(ticket.run, u64::from(ticket.step.get())),
        Some(ticket)
    );
    Ok(())
}

pub(super) fn finish_tail(runtime: &mut Runtime, run: RunId) -> TestResult {
    let ticket = runtime
        .lookup_pending_action_ticket(run, 1)
        .ok_or("issued pure tail absent")?;
    assert_eq!(ticket.action, ActionId::new(0));
    let output_slot = runtime
        .inspect_run_state(run)?
        .workflow
        .node(ticket.step)
        .and_then(|node| node.output.slot())
        .ok_or("compiled tail output absent")?;
    let value = SlotValue::I64(42);
    runtime.complete_action_with_output(
        ticket,
        vb_core::ActionOutputReady {
            output_slot,
            value,
            taint: Taint::Clean,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        },
    )?;
    runtime.tick_all()?;
    Ok(())
}

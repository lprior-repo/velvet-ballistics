use super::*;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::{
    ActionId, CapabilitySet, CompiledNodeKind, ConstIdx, ConstValue, RunId, RuntimePolicy, SlotIdx,
    SlotValue, StepIdx, WorkflowDigest, WorkflowId, WorkflowParts,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn startup_service(path: &std::path::Path) -> TestResult<IpcJournalService> {
    Ok(IpcJournalService {
        journal: Arc::new(vb_storage::FjallJournal::open(path, None)?),
        queue: Arc::new(vb_storage::JournalWriterQueue::new(
            1024,
            64,
            vb_storage::StorageLimits::DEFAULT,
        )?),
    })
}

fn startup_runtime(service: &IpcJournalService) -> TestResult<vb_runtime::runtime::Runtime> {
    let journal =
        vb_runtime::journal::RuntimeJournalConfig::new(vb_storage::DurabilityProfile::Journaled)
            .shared_journal(Arc::clone(&service.journal), Arc::clone(&service.queue))?;
    Ok(vb_runtime::runtime::Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
        journal,
    ))
}

fn startup_header(run: RunId, digest: WorkflowDigest) -> vb_storage::RunHeaderRecord {
    vb_storage::RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 0,
    }
}

fn startup_workflow(pending: bool) -> TestResult<vb_core::CompiledWorkflow> {
    let first = if pending {
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        }
    } else {
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        }
    };
    let result = if pending {
        SlotIdx::new(1)
    } else {
        SlotIdx::ZERO
    };
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("ipc-startup-retained"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            vb_core::workflow::node(StepIdx::ZERO, first, Some(result), Some(StepIdx::new(1))),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish { result },
                None,
                None,
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(7)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest =
        WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
    Ok(vb_core::CompiledWorkflow::try_from_parts(parts)?)
}

fn startup_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("startup-test")?,
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

fn seed_startup_run(
    service: &IpcJournalService,
    run: RunId,
    pending: bool,
) -> TestResult<Option<vb_core::ActionTicket>> {
    let workflow = startup_workflow(pending)?;
    let digest = workflow.digest();
    let contracts: Box<[ActionContract]> = if pending {
        Box::from([startup_contract()?])
    } else {
        Box::from([])
    };
    vb_storage::submit_artifact_with_contracts(
        &service.journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )?;
    let journal =
        vb_runtime::journal::StorageRuntimeJournal::shared_journaled(Arc::clone(&service.journal));
    let mut producer = vb_runtime::runtime::Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
        journal,
    );
    producer.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(7))]),
        CapabilitySet::empty(),
        contracts,
    )?;
    assert!(producer.tick_all()?);
    let ticket = producer.lookup_pending_action_ticket(run, 0);
    assert_eq!(ticket.is_some(), pending);
    service
        .journal
        .put_run_header(&startup_header(run, digest))?;
    service.journal.persist_strict()?;
    Ok(ticket)
}

#[test]
fn startup_header_overflow_preflights_before_recovery_or_journal_changes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = startup_service(dir.path())?;
    let digest = WorkflowDigest::from_bytes([7; 32]);
    for id in 1..=3 {
        let run = RunId::new(id);
        service
            .journal
            .put_run_header(&startup_header(run, digest))?;
        service
            .journal
            .append_strict(&vb_storage::JournalEvent::RunAccepted {
                run,
                seq: vb_storage::EventSeq::ZERO,
                workflow: digest,
            })?;
    }
    let before = service.journal.events_for_run_full(RunId::new(1))?;
    let mut runtime = startup_runtime(&service)?;
    let result = recover_startup(&service, &mut runtime, 2);
    assert!(matches!(
        result,
        Err(
            vb_storage::journal::startup_discovery::StartupDiscoveryError::RowLimitExceeded {
                limit: 2
            }
        )
    ));
    assert_eq!(service.journal.events_for_run_full(RunId::new(1))?, before);
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(STARTUP_HEADER_SCAN_LIMIT, 65_536);
    Ok(())
}

#[test]
fn startup_skips_current_terminal_history_without_using_stale_header_status() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = startup_service(dir.path())?;
    let run = RunId::new(41);
    assert!(seed_startup_run(&service, run, false)?.is_none());
    let before = service.journal.events_for_run_full(run)?;
    let mut runtime = startup_runtime(&service)?;
    recover_startup(&service, &mut runtime, STARTUP_HEADER_SCAN_LIMIT)?;
    assert_eq!(service.journal.events_for_run_full(run)?, before);
    assert!(!matches!(
        runtime.snapshot_run(run, 1)?,
        vb_runtime::shard::InspectResponse::Found(_)
    ));
    Ok(())
}

#[test]
fn startup_processes_enqueued_recover_before_returning_pending_ticket() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = startup_service(dir.path())?;
    let run = RunId::new(42);
    let ticket = seed_startup_run(&service, run, true)?.ok_or("pending ticket required")?;
    let before = service.journal.events_for_run_full(run)?;
    let mut runtime = startup_runtime(&service)?;
    assert!(runtime.lookup_pending_action_ticket(run, 0).is_none());
    recover_startup(&service, &mut runtime, STARTUP_HEADER_SCAN_LIMIT)?;
    assert_eq!(runtime.lookup_pending_action_ticket(run, 0), Some(ticket));
    assert_eq!(service.journal.events_for_run_full(run)?, before);
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 0);
    Ok(())
}

#[test]
fn startup_preserves_admission_refusal_and_leaves_history_unchanged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = startup_service(dir.path())?;
    let run = RunId::new(43);
    let digest = WorkflowDigest::from_bytes([8; 32]);
    service
        .journal
        .put_run_header(&startup_header(run, digest))?;
    service
        .journal
        .append_strict(&vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::ZERO,
            workflow: digest,
        })?;
    let before = service.journal.events_for_run_full(run)?;
    let mut runtime = startup_runtime(&service)?;
    let result = recover_startup(&service, &mut runtime, STARTUP_HEADER_SCAN_LIMIT);
    assert!(matches!(result,
        Err(vb_storage::journal::startup_discovery::StartupDiscoveryError::Visitor(
            vb_runtime::RuntimeError::RecoveryCannotResume { reason }
        )) if reason == "admission_missing"
    ));
    assert_eq!(service.journal.events_for_run_full(run)?, before);
    assert!(!matches!(
        runtime.snapshot_run(run, 1)?,
        vb_runtime::shard::InspectResponse::Found(_)
    ));
    Ok(())
}

#[test]
fn startup_discovers_acceptance_only_without_header_and_preserves_refusal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = startup_service(dir.path())?;
    let run = RunId::new(44);
    service
        .journal
        .append_strict(&vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::ZERO,
            workflow: WorkflowDigest::from_bytes([8; 32]),
        })?;
    assert!(service.journal.run_header(run)?.is_none());
    let before = service.journal.events_for_run_full(run)?;
    let mut runtime = startup_runtime(&service)?;
    let result = recover_startup(&service, &mut runtime, STARTUP_HEADER_SCAN_LIMIT);
    assert!(matches!(result,
        Err(vb_storage::journal::startup_discovery::StartupDiscoveryError::Visitor(
            vb_runtime::RuntimeError::RecoveryCannotResume { reason }
        )) if reason == "admission_missing"
    ));
    assert_eq!(service.journal.events_for_run_full(run)?, before);
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 0);
    assert!(!matches!(
        runtime.snapshot_run(run, 1)?,
        vb_runtime::shard::InspectResponse::Found(_)
    ));
    Ok(())
}

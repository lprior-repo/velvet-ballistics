use super::*;

struct TimerResolver(Box<[vb_core::CompiledWorkflow]>);

impl vb_ipc::server::WorkflowResolver for TimerResolver {
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<vb_core::CompiledWorkflow, vb_ipc::server::WorkflowResolutionError> {
        self.0
            .iter()
            .find(|workflow| workflow.digest() == digest)
            .cloned()
            .ok_or(vb_ipc::server::WorkflowResolutionError::NotFound)
    }
}

fn submit_ipc(
    runtime: &mut Runtime,
    resolver: &mut TimerResolver,
    workflow: &vb_core::CompiledWorkflow,
    run: RunId,
) -> TestResult<vb_ipc::server::IpcResponse> {
    Ok(vb_ipc::server::handlers::runs::submit_resolved_workflow(
        vb_ipc::IpcCommand::SubmitRunInline,
        vb_ipc::SubmitRunPayload {
            run_id: run,
            workflow: workflow.digest(),
            input: postcard::to_allocvec(&Vec::<(vb_core::SlotIdx, SlotValue)>::new())?,
        },
        runtime,
        Some(resolver),
    ))
}

fn has_scheduled_ask(events: &[vb_runtime::journal::RuntimeJournalEvent], run: RunId) -> bool {
    events.iter().any(|event| {
        matches!(event, vb_runtime::journal::RuntimeJournalEvent::AskScheduled { run: actual, .. } if *actual == run)
    })
}

fn has_failed_run(events: &[vb_runtime::journal::RuntimeJournalEvent], run: RunId) -> bool {
    events.iter().any(|event| {
        matches!(event, vb_runtime::journal::RuntimeJournalEvent::RunFailed { run: actual, .. } if *actual == run)
    })
}

fn process_invalid_ipc(
    runtime: &mut Runtime,
    resolver: &mut TimerResolver,
    journal: &vb_runtime::journal::VolatileRuntimeJournal,
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult {
    let run = RunId::new(51_003);
    let accepted = vb_ipc::server::IpcResponse::AcceptedRun { run_id: run.get() };
    if submit_ipc(runtime, resolver, workflow, run)? != accepted {
        return Err("IPC did not accept the invalid-timer fixture".into());
    }
    runtime.tick_all()?;
    let events = journal.snapshot()?;
    if has_scheduled_ask(&events, run) || !has_failed_run(&events, run) {
        return Err("invalid timeout was scheduled or did not fail".into());
    }
    Ok(())
}

fn process_ipc_control(
    runtime: &mut Runtime,
    resolver: &mut TimerResolver,
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult {
    let run = RunId::new(51_004);
    let accepted = vb_ipc::server::IpcResponse::AcceptedRun { run_id: run.get() };
    if submit_ipc(runtime, resolver, workflow, run)? != accepted {
        return Err("IPC did not accept the ordinary Ask control".into());
    }
    runtime.tick_all()?;
    if runtime
        .lookup_pending_ask_ticket(run, vb_core::StepIdx::new(3))
        .is_none()
    {
        return Err("IPC control Ask was not issued".into());
    }
    Ok(())
}

fn current_ordinary_workflow(journal: &FjallJournal) -> TestResult<vb_core::CompiledWorkflow> {
    let workflow = vb_compile::compile_workflow(SOURCE)?;
    journal.put_workflow_source(&WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE).into()),
        source: SOURCE.to_vec(),
    })?;
    vb_storage::submit_artifact(journal, &workflow, RuntimePolicy::Strict)?;
    Ok(workflow)
}

fn process_strict_invalid_ipc(
    runtime: &mut Runtime,
    resolver: &mut TimerResolver,
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult {
    let run = RunId::new(51_005);
    let accepted = vb_ipc::server::IpcResponse::AcceptedRun { run_id: run.get() };
    if submit_ipc(runtime, resolver, workflow, run)? != accepted {
        return Err("strict IPC did not accept the invalid timeout".into());
    }
    runtime.tick_all()?;
    let events = journal.events_for_run_full(run)?;
    strict_failure_has_no_ask(&events, run)
}

fn strict_failure_has_no_ask(events: &[JournalEvent], run: RunId) -> TestResult {
    let ask_scheduled = events.iter().any(|event| {
        matches!(event, JournalEvent::AskScheduledEvent { run: actual, .. } if *actual == run)
    });
    let failed = events.iter().any(
        |event| matches!(event, JournalEvent::RunFailedEvent { run: actual, .. } if *actual == run),
    );
    if ask_scheduled || !failed {
        return Err("strict invalid timeout scheduled an Ask or failed to terminate".into());
    }
    Ok(())
}

#[test]
fn invalid_public_ipc_timeout_leaves_service_able_to_accept_control() -> TestResult {
    let invalid = vb_compile::compile_workflow(HUGE_SOURCE)?;
    let ordinary = vb_compile::compile_workflow(SOURCE)?;
    let journal = Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            ..ShardConfig::default()
        },
        journal.clone(),
    )?;
    let mut resolver = TimerResolver(Box::from([invalid.clone(), ordinary.clone()]));
    process_invalid_ipc(&mut runtime, &mut resolver, &journal, &invalid)?;
    process_ipc_control(&mut runtime, &mut resolver, &ordinary)
}

#[test]
fn strict_public_ipc_timeout_refusal_keeps_service_healthy() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let invalid = current_huge_workflow(&journal)?;
    let ordinary = current_ordinary_workflow(&journal)?;
    let mut runtime = strict_runtime(journal.clone(), 10_000)?;
    let mut resolver = TimerResolver(Box::from([invalid.clone(), ordinary.clone()]));
    process_strict_invalid_ipc(&mut runtime, &mut resolver, &journal, &invalid)?;
    process_ipc_control(&mut runtime, &mut resolver, &ordinary)
}

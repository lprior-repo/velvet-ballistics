#[derive(Debug, Default)]
struct RejectAdmissionJournal(Mutex<Vec<RuntimeJournalEvent>>);

impl RuntimeJournal for RejectAdmissionJournal {
    fn append(&self, event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
        if matches!(event, RuntimeJournalEvent::RunSubmitted { .. }) {
            return Err(RuntimeError::JournalFull { capacity: 0 });
        }
        self.0
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(event);
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> crate::RuntimeResult<()> {
        if events
            .iter()
            .any(|event| matches!(event, RuntimeJournalEvent::RunSubmitted { .. }))
        {
            return Err(RuntimeError::JournalFull { capacity: 0 });
        }
        self.0
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .extend(events.iter().cloned());
        Ok(())
    }

    fn probe(&self) -> crate::RuntimeResult<()> {
        Ok(())
    }
}

#[test]
fn admission_persistence_failure_releases_transferred_runtime_quota() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let tenant = vb_core::ids::TenantId::new(91);
    let run = vb_core::ids::RunId::new(901);
    let journal = Arc::new(RejectAdmissionJournal::default());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    runtime
        .submit_direct_with_tenant(run, workflow, CapabilitySet::empty(), tenant)
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.global_cost_usage(), cost);
    let tick = runtime.tick_all();
    assert_eq!(tick, Err(RuntimeError::JournalFull { capacity: 0 }));
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    assert_eq!(
        journal.0.lock().map_err(|error| error.to_string())?.len(),
        0
    );
    Ok(())
}

#[test]
fn repeated_terminal_commands_release_quota_and_append_once() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let tenant = vb_core::ids::TenantId::new(92);
    let run = vb_core::ids::RunId::new(902);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone());
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    submit_suspended_tenant(&runtime, run, workflow, tenant).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    runtime.kill_run(run).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.global_cost_usage(), 0);
    let terminal = journal.snapshot().map_err(|error| error.to_string())?;
    runtime.cancel_run(run).map_err(|error| error.to_string())?;
    runtime.kill_run(run).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        terminal
    );
    Ok(())
}

#[test]
fn zero_cost_completions_preserve_shared_positive_tenant_charge() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let zero = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let positive = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let zero_cost = vb_core::budget::AggregateResourceBudget::from_workflow(&zero)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let positive_cost = vb_core::budget::AggregateResourceBudget::from_workflow(&positive)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    if zero_cost != 0 || positive_cost == 0 {
        return Err(format!(
            "fixture costs were {zero_cost} and {positive_cost}"
        ));
    }
    let tenant = vb_core::ids::TenantId::new(93);
    let other_tenant = vb_core::ids::TenantId::new(94);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal);
    runtime
        .set_resource_quotas(positive_cost, positive_cost)
        .map_err(|error| error.to_string())?;
    runtime
        .submit_direct_with_tenant(
            RunId::new(930),
            zero.clone(),
            CapabilitySet::empty(),
            tenant,
        )
        .map_err(|error| error.to_string())?;
    runtime
        .submit_direct_with_tenant(
            RunId::new(931),
            zero.clone(),
            CapabilitySet::empty(),
            tenant,
        )
        .map_err(|error| error.to_string())?;
    runtime
        .submit_direct_with_tenant(RunId::new(932), zero, CapabilitySet::empty(), other_tenant)
        .map_err(|error| error.to_string())?;
    submit_suspended_tenant(&runtime, RunId::new(933), positive, tenant)
        .map_err(|error| error.to_string())?;
    if runtime.global_cost_usage() != positive_cost
        || runtime.usage_snapshot().tenants().tenant_cost_usage(tenant) != Ok(positive_cost)
        || runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(other_tenant)
            != Ok(0)
    {
        return Err("zero-cost submissions changed positive quota accounting".into());
    }
    for _ in 0..8 {
        let _progressed = runtime
            .tick_all()
            .map_err(|error| format!("tick: {error:?}"))?;
        if runtime.counters_snapshot().runs_completed == 3 {
            break;
        }
    }
    if runtime.counters_snapshot().runs_completed != 3
        || runtime.global_cost_usage() != positive_cost
        || runtime.usage_snapshot().tenants().tenant_cost_usage(tenant) != Ok(positive_cost)
    {
        return Err("zero-cost completion released another run's quota".into());
    }
    runtime
        .kill_run(RunId::new(933))
        .map_err(|error| error.to_string())?;
    for _ in 0..8 {
        let _progressed = runtime
            .tick_all()
            .map_err(|error| format!("kill tick: {error:?}"))?;
        if runtime.global_cost_usage() == 0 {
            break;
        }
    }
    if runtime.global_cost_usage() != 0
        || runtime.usage_snapshot().tenants().tenant_cost_usage(tenant) != Ok(0)
        || runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(other_tenant)
            != Ok(0)
    {
        return Err("terminal release left a quota charge".into());
    }
    let _report = runtime
        .shutdown_graceful()
        .map_err(|error| format!("shutdown: {error:?}"))?;
    if runtime.global_cost_usage() != 0 {
        return Err("shutdown changed the released quota snapshot".into());
    }
    Ok(())
}

#[test]
fn two_zero_cost_tenant_runs_release_independently() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    if cost != 0 {
        return Err(format!("fixture cost was {cost}, expected zero"));
    }
    let tenant = vb_core::ids::TenantId::new(930);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal);
    for id in [930, 931] {
        runtime
            .submit_direct_with_tenant(
                RunId::new(id),
                workflow.clone(),
                CapabilitySet::empty(),
                tenant,
            )
            .map_err(|error| error.to_string())?;
    }
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 2);
    Ok(())
}

#[test]
fn duplicate_submit_cannot_raise_pending_run_queue_cap() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let mut parts = workflow.to_parts();
    parts.resource_contract.max_queue_depth = 1;
    let limited =
        vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let runtime = Runtime::new(
        shard_count,
        runtime_config(),
        Arc::new(VolatileRuntimeJournal::new()),
    );
    let run = RunId::new(940);
    let tenant = vb_core::ids::TenantId::new(940);
    runtime
        .submit_direct_with_tenant(run, limited, CapabilitySet::empty(), tenant)
        .map_err(|error| error.to_string())?;
    let before = runtime.pending_boundary_snapshot(8).command_queue_depth();
    assert_eq!(
        runtime.submit_direct_with_tenant(run, workflow, CapabilitySet::empty(), tenant),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1
        }),
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        before
    );
    assert_eq!(before, 1);
    Ok(())
}

#[test]
fn active_run_admission_cap_rejects_widening_duplicate_submit() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let mut low_parts = suspended_workflow()
        .ok_or_else(|| String::from("suspended workflow"))?
        .to_parts();
    low_parts.resource_contract.max_queue_depth = 1;
    let low =
        vb_core::CompiledWorkflow::try_from_parts(low_parts).map_err(|error| error.to_string())?;
    let high = suspended_workflow().ok_or_else(|| String::from("suspended workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&low)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let run = RunId::new(950);
    let tenant = vb_core::ids::TenantId::new(950);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    submit_suspended_tenant(&runtime, run, low, tenant).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let ticket = runtime
        .lookup_pending_action_ticket(run, 0)
        .ok_or_else(|| String::from("active action ticket"))?;
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert_eq!(runtime.global_cost_usage(), cost);

    submit_suspended_tenant(&runtime, run, high.clone(), tenant)
        .map_err(|error| error.to_string())?;
    let before_refusal = runtime.pending_boundary_snapshot(8).command_queue_depth();
    assert_eq!(before_refusal, 1);
    assert_eq!(
        submit_suspended_tenant(&runtime, run, high, tenant),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1,
        })
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        before_refusal
    );
    assert_eq!(runtime.lookup_pending_action_ticket(run, 0), Some(ticket));
    assert_eq!(runtime.global_cost_usage(), cost);
    let other_run = RunId::new(951);
    let other_tenant = vb_core::ids::TenantId::new(951);
    runtime
        .submit_direct_with_tenant(
            other_run,
            finished_workflow().ok_or_else(|| String::from("finished workflow"))?,
            CapabilitySet::empty(),
            other_tenant,
        )
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Err(RuntimeError::RunAlreadyExists));
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        1
    );
    assert_eq!(runtime.lookup_pending_action_ticket(run, 0), Some(ticket));
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.global_cost_usage(), cost);
    runtime.kill_run(run).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.global_cost_usage(), 0);
    Ok(())
}

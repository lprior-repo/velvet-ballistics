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

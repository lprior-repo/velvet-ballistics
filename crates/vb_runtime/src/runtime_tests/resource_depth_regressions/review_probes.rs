#[test]
fn review_two_zero_cost_tenant_runs_release_independently() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let workflow = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    assert_eq!(cost, 0);
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
fn review_duplicate_submit_cannot_raise_pending_run_queue_cap() -> Result<(), String> {
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
    assert_eq!(
        runtime.submit_direct_with_tenant(run, workflow, CapabilitySet::empty(), tenant),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1
        }),
    );
    Ok(())
}

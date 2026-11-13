fn tenant_depth_runtime(limit: u32) -> Result<(Runtime, Arc<VolatileRuntimeJournal>), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let capacity = usize::try_from(limit).map_err(|error| format!("queue capacity: {error}"))?;
    let config = ShardConfig {
        command_queue_capacity: capacity.saturating_add(1),
        ..runtime_config()
    };
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let runtime = Runtime::new(shard_count, config, journal.clone());
    Ok((runtime, journal))
}

fn submit_tenant_shape(
    runtime: &Runtime,
    workflow: &vb_core::workflow::CompiledWorkflow,
    run: vb_core::ids::RunId,
    tenant: vb_core::ids::TenantId,
    with_inputs_contracts: bool,
) -> crate::RuntimeResult<()> {
    if with_inputs_contracts {
        runtime.submit_direct_with_tenant_inputs_grants_and_contracts(
            run,
            workflow.clone(),
            Box::default(),
            CapabilitySet::empty(),
            Box::default(),
            tenant,
        )
    } else {
        runtime.submit_direct_with_tenant(run, workflow.clone(), CapabilitySet::empty(), tenant)
    }
}

fn tenant_submit_at_depth_limit(with_inputs_contracts: bool) -> Result<(), String> {
    let workflow = finished_workflow().ok_or_else(|| String::from("finished workflow"))?;
    let budget = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| format!("workflow budget: {error:?}"))?;
    let limit = budget.max_queue_depth;
    let (mut runtime, journal) = tenant_depth_runtime(limit)?;
    runtime
        .set_resource_quotas(u64::MAX, u64::MAX)
        .map_err(|error| format!("set quotas: {error:?}"))?;
    let run = vb_core::ids::RunId::new(900);
    let tenant = vb_core::ids::TenantId::new(90);
    let mut accepted = 0_u32;
    while accepted < limit {
        submit_tenant_shape(&runtime, &workflow, run, tenant, with_inputs_contracts)
            .map_err(|error| format!("tenant submit {accepted}: {error:?}"))?;
        accepted = accepted.saturating_add(1);
    }
    assert_eq!(
        submit_tenant_shape(&runtime, &workflow, run, tenant, with_inputs_contracts),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: limit,
            limit,
        })
    );
    assert_eq!(runtime.global_cost_usage(), budget.max_cost_units);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        usize::try_from(limit).map_err(|error| error.to_string())?
    );
    assert_eq!(
        journal
            .snapshot()
            .map_err(|error| format!("journal: {error:?}"))?,
        Vec::new()
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    Ok(())
}

#[test]
fn public_tenant_scoped_submit_obeys_exact_per_run_queue_depth() -> Result<(), String> {
    tenant_submit_at_depth_limit(false)
}

#[test]
fn public_tenant_inputs_contracts_submit_obeys_exact_per_run_queue_depth() -> Result<(), String> {
    tenant_submit_at_depth_limit(true)
}

fn quota_suspended_workflow() -> Result<vb_core::workflow::CompiledWorkflow, String> {
    let workflow = suspended_workflow().ok_or_else(|| String::from("workflow fixture"))?;
    let mut parts = workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let encoded =
        postcard::to_allocvec(&parts).map_err(|error| format!("workflow encode: {error:?}"))?;
    parts.digest = WorkflowDigest::from_bytes(*blake3::hash(&encoded).as_bytes());
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("workflow build: {error:?}"))
}

#[test]
fn cold_recovery_restores_tenant_global_quota_and_exact_journal_bytes() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    let workflow = quota_suspended_workflow()?;
    let action = ActionId::new(0);
    let capability = Capability::new("network".into(), action);
    let contracts = Box::from([ActionContract {
        id: action,
        name: vb_core::action::ActionName::from_static_infallible("quota-recovery"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::from([capability.clone()]),
    }]);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Journaled,
        &contracts,
    )
    .map_err(|error| error.to_string())?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    assert!(
        cost > 0,
        "recovery fixture must carry positive admitted cost"
    );
    let tenant = vb_core::ids::TenantId::new(71_011);
    let run = RunId::new(71_011);
    let shard_count = NonZeroUsize::new(2).ok_or_else(|| String::from("two shards"))?;
    let config = ShardConfig {
        policy: RuntimePolicy::Journaled,
        ..ShardConfig::default()
    };
    let mut runtime = Runtime::new(
        shard_count,
        config,
        crate::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    runtime
        .submit_direct_with_tenant_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            CapabilitySet::from_grants(Box::from([capability.clone()])),
            contracts,
            tenant,
        )
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(cost)
    );
    let before = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let origins: Vec<_> = before
        .iter()
        .filter_map(|event| match event {
            vb_storage::JournalEvent::RunBudgetOriginEvent {
                seq,
                started_millis,
                ..
            } => Some((*seq, *started_millis)),
            _ => None,
        })
        .collect();
    assert_eq!(
        origins.len(),
        1,
        "budgeted tenant submission persists one origin"
    );
    let origin = origins
        .first()
        .copied()
        .ok_or_else(|| String::from("origin missing"))?;
    let charged_bytes = crate::recovery::recover_journal_bytes_written(&before)
        .map_err(|error| error.to_string())?;
    drop(runtime);
    drop(journal);

    let reopened = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    let mut over_quota_runtime = Runtime::new(
        shard_count,
        config,
        crate::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&reopened)),
    );
    let tenant_ceiling = cost
        .checked_sub(1)
        .ok_or_else(|| String::from("cost must be positive"))?;
    over_quota_runtime
        .set_resource_quotas(tenant_ceiling, cost)
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        over_quota_runtime.recover_and_resume(run),
        Err(RuntimeError::ResourceQuotaExceeded {
            resource: "tenant_quota_cost_units",
            requested,
            available,
        }) if requested == cost && available == tenant_ceiling
    ));
    assert_eq!(over_quota_runtime.global_cost_usage(), 0);
    let rejected_events = reopened
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(rejected_events, before);
    drop(over_quota_runtime);
    let mut recovered_runtime = Runtime::new(
        shard_count,
        config,
        crate::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&reopened)),
    );
    recovered_runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    let product = recovered_runtime
        .recover_product(run)
        .map_err(|error| error.to_string())?;
    let command = match product {
        crate::recovery::RuntimeRecoveryProduct::Resumable(product) => {
            product.into_recover_command()
        }
        other => return Err(format!("expected resumable product: {other:?}")),
    };
    assert_eq!(command.recovered_tenant, Some(tenant));
    assert_eq!(command.run_started_millis, Some(origin.1));
    assert_eq!(
        command.recovered_action_attempts.as_deref(),
        Some(&[1_u16][..])
    );
    assert_eq!(command.recovered_journal_bytes_written, Some(charged_bytes));
    for missing in ["action_attempts_missing", "journal_bytes_missing"] {
        let mut incomplete = command.clone();
        match missing {
            "action_attempts_missing" => incomplete.recovered_action_attempts = None,
            _ => incomplete.recovered_journal_bytes_written = None,
        }
        let mut shard = crate::shard::Shard::new_with_journal(
            config,
            crate::journal::StorageRuntimeJournal::shared_strict(Arc::clone(&reopened)),
        );
        shard.set_resource_quotas(cost, cost);
        shard
            .enqueue(crate::shard::ShardCommand::Recover {
                command: Box::new(incomplete),
            })
            .map_err(|error| error.to_string())?;
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::RecoveryCannotResume {
                reason: missing.to_string(),
            })
        );
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.global_cost_usage(), 0);
        let unchanged = reopened
            .events_for_run_full(run)
            .map_err(|error| error.to_string())?;
        assert_eq!(unchanged, before);
    }
    recovered_runtime
        .recover_and_resume(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(recovered_runtime.global_cost_usage(), cost);
    assert_eq!(
        recovered_runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(tenant),
        Ok(cost)
    );
    assert_eq!(recovered_runtime.tick_all(), Ok(true));
    assert_eq!(
        recovered_runtime
            .inspect_run_state(run)
            .map_err(|error| error.to_string())?
            .run_started_millis,
        Some(origin.1)
    );
    let after = reopened
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before);
    recovered_runtime
        .cancel_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(recovered_runtime.tick_all(), Ok(true));
    assert_eq!(recovered_runtime.global_cost_usage(), 0);
    assert_eq!(
        recovered_runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

#[test]
fn public_tenant_quota_releases_after_action_failure() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let workflow = suspended_workflow().ok_or_else(|| String::from("workflow fixture"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let tenant = vb_core::ids::TenantId::new(71_012);
    let run = RunId::new(71_012);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    submit_suspended_tenant(&runtime, run, workflow, tenant).map_err(|error| error.to_string())?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(runtime.tick_all(), Ok(true));
    let ticket = ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run, 1, SeqNo::ZERO, ActionId::new(0)),
        capacity: 1,
        fencing_token: 0,
    };
    runtime
        .fail_action(ticket, ActionFailureCode::Rejected.into())
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

#[test]
fn public_tenant_quota_releases_after_kill() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).ok_or_else(|| String::from("one shard"))?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());
    let workflow = suspended_workflow().ok_or_else(|| String::from("workflow fixture"))?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    let tenant = vb_core::ids::TenantId::new(71_013);
    let run = RunId::new(71_013);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    submit_suspended_tenant(&runtime, run, workflow, tenant).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    runtime.kill_run(run).map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.usage_snapshot().tenants().tenant_cost_usage(tenant),
        Ok(0)
    );
    Ok(())
}

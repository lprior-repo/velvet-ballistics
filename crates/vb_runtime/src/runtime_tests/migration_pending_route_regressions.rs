// Depends on migration_quota_regressions.rs. These assert existing duplicate
// admission semantics: accept pending duplicate, reject it at dequeue.
fn submit_public_migration_duplicate(runtime: &Runtime) -> Result<(), String> {
    submit_suspended_tenant(
        runtime,
        RunId::new(28_001),
        suspended_workflow().ok_or("suspended workflow")?,
        vb_core::ids::TenantId::new(281),
    )
    .map_err(|error| error.to_string())
}

#[test]
fn migrated_pending_public_duplicate_routes_to_destination_at_exact_quota() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    // No activation has occurred. Public routing must still recognize custody
    // on the non-home shard instead of attempting a second C reservation at home.
    submit_public_migration_duplicate(&runtime)?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(vb_core::ids::TenantId::new(281)),
        Ok(cost)
    );
    assert_eq!(migration_queue_depth(&runtime, source)?, 0);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 0);
    assert_eq!(migration_queue_depth(&runtime, target)?, 2);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 2);
    assert_migration_front(&runtime, target, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, target, migration_expected_submit()?, None)?;
    Ok(())
}

#[test]
fn migrated_pending_public_duplicate_cannot_activate_two_shard_owners() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    let run = RunId::new(28_001);
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    // Roomy quota ensures wrong routing is not merely hidden by quota refusal.
    submit_public_migration_duplicate(&runtime)?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let owners = runtime
        .shards
        .iter()
        .filter(|shard| shard.run_state_contains(run))
        .count();
    assert_eq!(owners, 1, "one RunId must never activate on two shards");
    assert!(
        !runtime
            .shards
            .get(source)
            .ok_or("source shard")?
            .run_state_contains(run)
    );
    assert!(
        runtime
            .shards
            .get(target)
            .ok_or("target shard")?
            .run_state_contains(run)
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    assert_eq!(
        runtime.tick_shard(shard_error_index(target), ShardDirective::Continue),
        Err(RuntimeError::RunAlreadyExists)
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    runtime.kill_run(run).map_err(|error| error.to_string())?;
    assert_eq!(
        runtime.tick_shard(shard_error_index(target), ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(vb_core::ids::TenantId::new(281)),
        Ok(0)
    );
    assert_eq!(
        runtime
            .shards
            .iter()
            .filter(|shard| shard.run_state_contains(run))
            .count(),
        0
    );
    Ok(())
}

#[test]
fn pending_unknown_control_guides_subsequent_public_submission_placement() -> Result<(), String> {
    let count = NonZeroUsize::new(2).ok_or("two shards")?;
    let runtime = Runtime::new_for_tests_and_benchmarks_only(count, runtime_config())
        .map_err(|error| error.to_string())?;
    let run = RunId::new(28_001);
    let home = runtime.shard_index(run);
    let target = if home == 0 { 1 } else { 0 };
    enqueue_migration_inspect(&runtime, target, 28_001)?;
    assert_eq!(runtime.global_cost_usage(), 0);
    submit_public_migration_duplicate(&runtime)?;
    let workflow = suspended_workflow().ok_or("suspended workflow")?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    assert!(cost > 0);
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_pending(&runtime, home, 28_001)?, 0);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 2);
    assert_migration_front(&runtime, target, migration_expected_inspect(28_001), None)?;
    assert_migration_front(&runtime, target, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

#[test]
fn ambiguous_pending_placement_rejects_public_submission_without_mutation() -> Result<(), String> {
    let (runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    enqueue_migration_inspect(&runtime, target, 28_001)?;
    let workflow = suspended_workflow().ok_or("suspended workflow")?;
    assert_eq!(
        submit_suspended_tenant(
            &runtime,
            RunId::new(28_001),
            workflow,
            vb_core::ids::TenantId::new(281)
        ),
        Err(RuntimeError::UnsupportedOperation {
            operation: "migration_pending_owner_ambiguous"
        })
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_queue_depth(&runtime, source)?, 1);
    assert_eq!(migration_queue_depth(&runtime, target)?, 1);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 1);
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, target, migration_expected_inspect(28_001), None)?;
    Ok(())
}

// Follow-up lifecycle acceptance, dependent on migration_quota_regressions.rs.
// These tests would execute volatile submission/kill paths if run. They are not
// source-only or custody-only tests; authoring them does not clear execution holds.
fn assert_migration_lifecycle_charge(runtime: &Runtime, cost: u64) {
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(vb_core::ids::TenantId::new(281)),
        Ok(cost)
    );
}

fn finish_migration_lifecycle(runtime: &mut Runtime, owner: usize) -> Result<(), String> {
    let run = RunId::new(28_001);
    assert!(
        runtime
            .shards
            .get(owner)
            .ok_or("owner shard")?
            .run_state_contains(run)
    );
    runtime.kill_run(run).map_err(|error| error.to_string())?;
    assert_eq!(
        runtime.tick_shard(shard_error_index(owner), ShardDirective::Continue),
        Ok(true)
    );
    assert_migration_lifecycle_charge(runtime, 0);
    assert!(
        !runtime
            .shards
            .get(owner)
            .ok_or("owner shard")?
            .run_state_contains(run)
    );
    assert_eq!(migration_queue_depth(runtime, owner)?, 0);
    Ok(())
}

#[test]
fn migration_reserved_submission_activates_once_and_releases_once() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
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
    assert_migration_lifecycle_charge(&runtime, cost);
    assert_eq!(
        runtime.tick_shard(shard_error_index(target), ShardDirective::Continue),
        Ok(true)
    );
    assert_migration_lifecycle_charge(&runtime, cost);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 0);
    finish_migration_lifecycle(&mut runtime, target)
}

#[test]
fn migration_unreserved_submission_charges_on_activation_then_releases() -> Result<(), String> {
    let count = NonZeroUsize::new(2).ok_or("two shards")?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(count, runtime_config())
        .map_err(|error| error.to_string())?;
    let workflow = suspended_workflow().ok_or("suspended workflow")?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    assert!(cost > 0);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    runtime
        .shards
        .get(0)
        .ok_or("source shard")?
        .command_queue
        .enqueue(migration_expected_submit()?)
        .map_err(|error| error.to_string())?;
    assert_migration_lifecycle_charge(&runtime, 0);
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false)
    );
    assert_migration_lifecycle_charge(&runtime, 0);
    assert_eq!(runtime.tick_shard(1, ShardDirective::Continue), Ok(true));
    assert_migration_lifecycle_charge(&runtime, cost);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    finish_migration_lifecycle(&mut runtime, 1)
}

#[test]
fn migration_independent_duplicate_reservation_releases_only_rejected_charge() -> Result<(), String>
{
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    runtime
        .shards
        .get(target)
        .ok_or("target shard")?
        .enqueue(migration_expected_submit()?)
        .map_err(|error| error.to_string())?;
    let twice = cost.checked_mul(2).ok_or("quota overflow")?;
    assert_migration_lifecycle_charge(&runtime, twice);
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    assert_migration_lifecycle_charge(&runtime, twice);
    assert_eq!(
        runtime.tick_shard(shard_error_index(target), ShardDirective::Continue),
        Ok(true)
    );
    assert_migration_lifecycle_charge(&runtime, twice);
    assert_eq!(
        runtime.tick_shard(shard_error_index(target), ShardDirective::Continue),
        Err(RuntimeError::RunAlreadyExists)
    );
    assert_migration_lifecycle_charge(&runtime, cost);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    finish_migration_lifecycle(&mut runtime, target)
}

#[test]
fn migration_live_source_duplicate_refusal_preserves_owner_and_pending_action() -> Result<(), String>
{
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    let run = RunId::new(28_001);
    assert_eq!(
        runtime.tick_shard(shard_error_index(source), ShardDirective::Continue),
        Ok(true)
    );
    assert!(
        runtime
            .shards
            .get(source)
            .ok_or("source shard")?
            .run_state_contains(run)
    );
    let before = runtime
        .snapshot_run(run, 281)
        .map_err(|error| error.to_string())?;
    let events = runtime
        .list_events(run)
        .map_err(|error| error.to_string())?;
    runtime
        .shards
        .get(source)
        .ok_or("source shard")?
        .enqueue(migration_expected_submit()?)
        .map_err(|error| error.to_string())?;
    assert_migration_lifecycle_charge(&runtime, cost);
    let boundary = runtime.pending_boundary_snapshot(4);
    let owner_state = runtime
        .shards
        .get(source)
        .ok_or("source shard")?
        .run_state_get(run)
        .cloned()
        .ok_or("live run state")?;
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Err(RuntimeError::UnsupportedOperation {
            operation: "migrate_source_run_owned"
        })
    );
    assert_eq!(runtime.pending_boundary_snapshot(4), boundary);
    assert_eq!(
        runtime
            .shards
            .get(source)
            .ok_or("source shard")?
            .run_state_get(run),
        Some(&owner_state)
    );
    assert_eq!(
        runtime
            .snapshot_run(run, 281)
            .map_err(|error| error.to_string())?,
        before
    );
    assert_eq!(
        runtime
            .list_events(run)
            .map_err(|error| error.to_string())?,
        events
    );
    assert_migration_lifecycle_charge(&runtime, cost);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_eq!(migration_queue_depth(&runtime, target)?, 0);
    assert!(
        !runtime
            .shards
            .get(target)
            .ok_or("target shard")?
            .run_state_contains(run)
    );
    // Dequeue normally instead of destructively inspecting custody; the duplicate
    // must fail without releasing the live owner's existing C charge.
    assert_eq!(
        runtime.tick_shard(shard_error_index(source), ShardDirective::Continue),
        Err(RuntimeError::RunAlreadyExists)
    );
    assert_migration_lifecycle_charge(&runtime, cost);
    finish_migration_lifecycle(&mut runtime, source)
}

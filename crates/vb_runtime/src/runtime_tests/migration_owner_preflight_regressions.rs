// Synthetic partial-ownership tests, not proof these states arose in execution.
// Depends on migration_quota_regressions.rs. No Store-backed receipt fixture.
fn assert_migration_owner_refusal(
    runtime: &mut Runtime,
    source: usize,
    target: usize,
    charge: u64,
) -> Result<(), String> {
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
    assert_eq!(runtime.global_cost_usage(), charge);
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(vb_core::ids::TenantId::new(281)),
        Ok(charge)
    );
    assert_eq!(migration_pending(runtime, source, 28_001)?, 1);
    assert_eq!(migration_queue_depth(runtime, source)?, 1);
    assert_eq!(migration_queue_depth(runtime, target)?, 0);
    Ok(())
}

#[test]
fn migration_runtime_only_source_owner_refuses_without_losing_custody() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    let run = RunId::new(28_001);
    let shard = runtime.shards.get_mut(source).ok_or("source shard")?;
    shard
        .checked_out_run_insert(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        shard.runtime_state_insert(run, RuntimeState::Running),
        Ok(None)
    );
    shard.checked_out_run_remove(run);
    assert!(!shard.run_state_contains(run));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    assert_migration_owner_refusal(&mut runtime, source, target, cost)?;
    let shard = runtime.shards.get(source).ok_or("source shard")?;
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    assert!(!shard.run_state_contains(run));
    assert!(!shard.checked_out_run_contains(run));
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

#[test]
fn migration_orphan_charged_source_owner_refuses_without_releasing_charge() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    let run = RunId::new(28_001);
    let tenant = Some(vb_core::ids::TenantId::new(281));
    let shard = runtime.shards.get_mut(source).ok_or("source shard")?;
    // A separate positive reservation represents an orphan owner. The queued
    // submission still holds its own C; both charges must remain intact.
    shard
        .reserve_admission_quota(run, tenant, cost)
        .map_err(|error| error.to_string())?;
    assert!(!shard.run_state_contains(run));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), None);
    assert_eq!(shard.charged_quotas.get(&run), Some(&(tenant, cost)));
    let twice = cost.checked_mul(2).ok_or("quota overflow")?;
    assert_eq!(runtime.global_cost_usage(), twice);
    assert_migration_owner_refusal(&mut runtime, source, target, twice)?;
    let shard = runtime.shards.get(source).ok_or("source shard")?;
    assert_eq!(shard.charged_quotas.get(&run), Some(&(tenant, cost)));
    assert!(!shard.run_state_contains(run));
    assert!(!shard.checked_out_run_contains(run));
    assert_eq!(shard.runtime_state_get(run), None);
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

#[test]
fn migration_owned_activation_preflight_refusal_preserves_earlier_charged_command()
-> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    let activation = crate::shard::ShardCommand::ActivateOwnedSubmission {
        run: RunId::new(28_009),
        expected_source_digest: WorkflowDigest::from_bytes([29; 32]),
    };
    // Raw queue injection supplies no receipt. Relaxed destination must fail
    // strict-policy preflight before any Store lookup or publication.
    runtime
        .shards
        .get(source)
        .ok_or("source shard")?
        .command_queue
        .enqueue(activation.clone())
        .map_err(|error| error.to_string())?;
    enqueue_migration_inspect(&runtime, target, 28_003)?;
    for _ in 0..2 {
        assert_eq!(
            runtime.tick_shard(
                shard_error_index(source),
                ShardDirective::Migrate {
                    target: shard_error_index(target),
                }
            ),
            Err(RuntimeError::UnsupportedOperation {
                operation: "owned_activation_strict_only"
            })
        );
        assert_eq!(runtime.global_cost_usage(), cost);
        assert_eq!(migration_queue_depth(&runtime, source)?, 2);
        assert_eq!(migration_queue_depth(&runtime, target)?, 1);
        for id in [28_001, 28_009] {
            assert_eq!(migration_pending(&runtime, source, id)?, 1);
            assert_eq!(migration_pending(&runtime, target, id)?, 0);
        }
    }
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, source, activation, None)?;
    assert_migration_front(&runtime, target, migration_expected_inspect(28_003), None)?;
    Ok(())
}

#[test]
fn migration_source_terminal_marker_refuses_to_create_shadowed_target_owner() -> Result<(), String>
{
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    let run = RunId::new(28_001);
    // Synthetic terminal marker represents old home routing authority. Moving
    // only the new queued submission would leave that home marker behind.
    runtime
        .shards
        .get_mut(source)
        .ok_or("source shard")?
        .checked_out_run_insert(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        runtime
            .shards
            .get_mut(source)
            .ok_or("source shard")?
            .terminal_runs_insert(run),
        Ok(true)
    );
    assert!(
        !runtime
            .shards
            .get(source)
            .ok_or("source shard")?
            .checked_out_run_contains(run)
    );
    assert_migration_owner_refusal(&mut runtime, source, target, cost)?;
    assert!(
        runtime
            .shards
            .get(source)
            .ok_or("source shard")?
            .terminal_runs_contains(run)
    );
    assert!(
        !runtime
            .shards
            .get(target)
            .ok_or("target shard")?
            .terminal_runs_contains(run)
    );
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

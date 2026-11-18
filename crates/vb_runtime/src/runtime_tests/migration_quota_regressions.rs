// In-memory migration invariants. No command execution or durable Store is needed.
fn migration_quota_fixture() -> Result<(Runtime, u64, usize, usize), String> {
    migration_quota_fixture_with_capacity(2)
}

fn migration_quota_fixture_with_capacity(
    capacity: usize,
) -> Result<(Runtime, u64, usize, usize), String> {
    let count = NonZeroUsize::new(2).ok_or("two shards")?;
    let config = ShardConfig {
        command_queue_capacity: capacity,
        ..runtime_config()
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(count, config)
        .map_err(|error| error.to_string())?;
    let workflow = suspended_workflow().ok_or("suspended workflow")?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    assert!(cost > 0, "migration fixture must reserve positive quota");
    let limit = cost.checked_mul(4).ok_or("quota overflow")?;
    runtime
        .set_resource_quotas(limit, limit)
        .map_err(|error| error.to_string())?;
    let run = RunId::new(28_001);
    submit_suspended_tenant(&runtime, run, workflow, vb_core::ids::TenantId::new(281))
        .map_err(|error| error.to_string())?;
    let source = runtime.shard_index(run);
    let target = if source == 0 { 1 } else { 0 };
    assert_eq!(runtime.global_cost_usage(), cost);
    Ok((runtime, cost, source, target))
}

fn migration_queue_depth(runtime: &Runtime, shard: usize) -> Result<usize, String> {
    runtime
        .shards
        .get(shard)
        .map(|shard| shard.command_queue.len())
        .ok_or_else(|| String::from("missing fixture shard"))
}

fn enqueue_migration_inspect(runtime: &Runtime, shard: usize, id: u64) -> Result<(), String> {
    runtime
        .shards
        .get(shard)
        .ok_or("missing fixture shard")?
        .enqueue(crate::shard::ShardCommand::Inspect {
            run: RunId::new(id),
            correlation: id,
        })
        .map_err(|error| error.to_string())
}

#[test]
fn migration_full_target_preserves_source_command_and_remainder() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    // Source holds a positive-cost submission followed by a distinct command.
    enqueue_migration_inspect(&runtime, source, 28_002)?;
    enqueue_migration_inspect(&runtime, target, 28_003)?;
    enqueue_migration_inspect(&runtime, target, 28_004)?;
    assert_eq!(migration_queue_depth(&runtime, source)?, 2);
    assert_eq!(migration_queue_depth(&runtime, target)?, 2);
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Err(RuntimeError::QueueFull)
    );
    assert_eq!(migration_queue_depth(&runtime, source)?, 2);
    assert_eq!(migration_queue_depth(&runtime, target)?, 2);
    assert_eq!(runtime.global_cost_usage(), cost);
    Ok(())
}

#[test]
fn migration_transfers_existing_quota_without_reserving_again() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    assert_eq!(migration_queue_depth(&runtime, source)?, 0);
    assert_eq!(migration_queue_depth(&runtime, target)?, 1);
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(vb_core::ids::TenantId::new(281)),
        Ok(cost)
    );
    Ok(())
}

#[test]
fn migration_at_exact_quota_limit_does_not_readmit_submission() -> Result<(), String> {
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
    assert_eq!(migration_queue_depth(&runtime, source)?, 0);
    assert_eq!(migration_queue_depth(&runtime, target)?, 1);
    assert_eq!(runtime.global_cost_usage(), cost);
    Ok(())
}

fn migration_expected_submit() -> Result<crate::shard::ShardCommand, String> {
    Ok(
        crate::shard::ShardCommand::SubmitTenantWithInputsAndContracts {
            run: RunId::new(28_001),
            workflow: suspended_workflow().ok_or("suspended workflow")?,
            inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0,
            )),
            tenant: vb_core::ids::TenantId::new(281),
        },
    )
}

fn migration_expected_inspect(id: u64) -> crate::shard::ShardCommand {
    crate::shard::ShardCommand::Inspect {
        run: RunId::new(id),
        correlation: id,
    }
}

// Destructive oracle only after all ledger assertions: keep the wrapper intact
// until checking its reservation. This is not a lifecycle/release simulation.
fn assert_migration_front(
    runtime: &Runtime,
    shard: usize,
    expected: crate::shard::ShardCommand,
    reserved_cost: Option<u64>,
) -> Result<(), String> {
    let queue = &runtime
        .shards
        .get(shard)
        .ok_or("missing shard")?
        .command_queue;
    let queued = queue.pop_queued().ok_or("missing command")?;
    assert_eq!(queued.command, expected);
    assert_eq!(
        queued.quota.map(|quota| (quota.tenant, quota.cost_units)),
        reserved_cost.map(|cost| (Some(vb_core::ids::TenantId::new(281)), cost))
    );
    Ok(())
}

fn migration_pending(runtime: &Runtime, shard: usize, id: u64) -> Result<u32, String> {
    Ok(runtime
        .shards
        .get(shard)
        .ok_or("missing shard")?
        .command_queue
        .pending_for(RunId::new(id)))
}

#[test]
fn migration_appends_exact_fifo_and_moves_pending_indexes() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    enqueue_migration_inspect(&runtime, source, 28_002)?;
    enqueue_migration_inspect(&runtime, target, 28_003)?;
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_queue_depth(&runtime, source)?, 0);
    assert_eq!(migration_queue_depth(&runtime, target)?, 3);
    for id in [28_001, 28_002] {
        assert_eq!(migration_pending(&runtime, source, id)?, 0);
        assert_eq!(migration_pending(&runtime, target, id)?, 1);
    }
    assert_eq!(migration_pending(&runtime, target, 28_003)?, 1);
    assert_migration_front(&runtime, target, migration_expected_inspect(28_003), None)?;
    assert_migration_front(&runtime, target, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, target, migration_expected_inspect(28_002), None)?;
    for id in [28_001, 28_002, 28_003] {
        assert_eq!(migration_pending(&runtime, target, id)?, 0);
    }
    assert_eq!(migration_queue_depth(&runtime, target)?, 0);
    Ok(())
}

#[test]
fn migration_insufficient_space_preserves_exact_fifos_and_indexes() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    enqueue_migration_inspect(&runtime, source, 28_002)?;
    enqueue_migration_inspect(&runtime, target, 28_003)?;
    // One free target slot must not cause a partially committed prefix.
    for _ in 0..2 {
        assert_eq!(
            runtime.tick_shard(
                shard_error_index(source),
                ShardDirective::Migrate {
                    target: shard_error_index(target),
                }
            ),
            Err(RuntimeError::QueueFull)
        );
        assert_eq!(runtime.global_cost_usage(), cost);
        assert_eq!(migration_queue_depth(&runtime, source)?, 2);
        assert_eq!(migration_queue_depth(&runtime, target)?, 1);
        for id in [28_001, 28_002] {
            assert_eq!(migration_pending(&runtime, source, id)?, 1);
            assert_eq!(migration_pending(&runtime, target, id)?, 0);
        }
        assert_eq!(migration_pending(&runtime, target, 28_003)?, 1);
    }
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, source, migration_expected_inspect(28_002), None)?;
    assert_migration_front(&runtime, target, migration_expected_inspect(28_003), None)?;
    Ok(())
}

#[test]
fn migration_duplicate_submissions_keep_single_reservation_and_index_custody() -> Result<(), String>
{
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    let submit = migration_expected_submit()?;
    runtime
        .shards
        .get(source)
        .ok_or("source shard")?
        .enqueue(submit.clone())
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    // Destination pending-submission index suppresses another reservation.
    runtime
        .shards
        .get(target)
        .ok_or("target shard")?
        .enqueue(submit.clone())
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 3);
    // Source pending-submission index was cleared, so fresh admission charges C.
    runtime
        .shards
        .get(source)
        .ok_or("source shard")?
        .enqueue(submit.clone())
        .map_err(|error| error.to_string())?;
    let twice = cost.checked_mul(2).ok_or("quota overflow")?;
    assert_eq!(runtime.global_cost_usage(), twice);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_migration_front(&runtime, target, submit.clone(), Some(cost))?;
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 2);
    assert_migration_front(&runtime, target, submit.clone(), None)?;
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 1);
    assert_migration_front(&runtime, target, submit.clone(), None)?;
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 0);
    assert_migration_front(&runtime, source, submit, Some(cost))?;
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 0);
    Ok(())
}

#[test]
fn migration_retains_source_per_run_bound_after_transfer() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    let inspect = migration_expected_inspect(28_001);
    runtime
        .shards
        .get(source)
        .ok_or("source shard")?
        .command_queue
        .enqueue_with_limit(inspect.clone(), Some(2))
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
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime
            .shards
            .get(target)
            .ok_or("target shard")?
            .enqueue(inspect.clone()),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run: RunId::new(28_001),
            pending: 2,
            limit: 2
        })
    );
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 2);
    assert_migration_front(&runtime, target, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, target, inspect, None)?;
    Ok(())
}

#[test]
fn migration_keeps_independent_duplicate_reservations_on_both_queues() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    let submit = migration_expected_submit()?;
    runtime
        .shards
        .get(target)
        .ok_or("target shard")?
        .enqueue(submit.clone())
        .map_err(|error| error.to_string())?;
    let twice = cost.checked_mul(2).ok_or("quota overflow")?;
    assert_eq!(runtime.global_cost_usage(), twice);
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    assert_eq!(runtime.global_cost_usage(), twice);
    assert_eq!(
        runtime
            .usage_snapshot()
            .tenants()
            .tenant_cost_usage(vb_core::ids::TenantId::new(281)),
        Ok(twice)
    );
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 0);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 2);
    assert_migration_front(&runtime, target, submit.clone(), Some(cost))?;
    assert_migration_front(&runtime, target, submit, Some(cost))?;
    assert_eq!(migration_queue_depth(&runtime, target)?, 0);
    Ok(())
}

#[test]
fn migration_preserves_standalone_unreserved_submission_custody() -> Result<(), String> {
    let count = NonZeroUsize::new(2).ok_or("two shards")?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(count, runtime_config())
        .map_err(|error| error.to_string())?;
    let submit = migration_expected_submit()?;
    let workflow = suspended_workflow().ok_or("suspended workflow")?;
    let cost = vb_core::budget::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?
        .max_cost_units;
    assert!(cost > 0);
    runtime
        .set_resource_quotas(cost, cost)
        .map_err(|error| error.to_string())?;
    // Raw queue admission intentionally does not reserve quota. Transfer must
    // not silently convert this valid representation into a fresh reservation.
    runtime
        .shards
        .get(0)
        .ok_or("source shard")?
        .command_queue
        .enqueue(submit.clone())
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false)
    );
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_eq!(migration_pending(&runtime, 0, 28_001)?, 0);
    assert_eq!(migration_pending(&runtime, 1, 28_001)?, 1);
    assert_migration_front(&runtime, 1, submit, None)?;
    Ok(())
}

#[test]
fn migration_source_checkout_refusal_preserves_charged_queue() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    // Synthetic ownership marker, not a claim to reproduce live execution.
    runtime
        .shards
        .get_mut(source)
        .ok_or("source shard")?
        .checked_out_run_insert(RunId::new(28_001))
        .map_err(|error| error.to_string())?;
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
    assert!(
        runtime
            .shards
            .get(source)
            .ok_or("source shard")?
            .checked_out_run_contains(RunId::new(28_001))
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_eq!(migration_queue_depth(&runtime, target)?, 0);
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

#[test]
fn migration_shutdown_target_preserves_charged_source() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture()?;
    // Target is empty: shutdown does not execute a queued command.
    assert_eq!(
        runtime.tick_shard(shard_error_index(target), ShardDirective::Shutdown),
        Ok(false)
    );
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Err(RuntimeError::ShutdownInProgress)
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_eq!(migration_queue_depth(&runtime, target)?, 0);
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

#[test]
fn migration_retains_historical_limit_after_its_command_was_popped() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    let inspect = migration_expected_inspect(28_001);
    let queue = &runtime
        .shards
        .get(target)
        .ok_or("target shard")?
        .command_queue;
    queue
        .enqueue_with_limit(inspect.clone(), Some(2))
        .map_err(|error| error.to_string())?;
    queue
        .enqueue(inspect.clone())
        .map_err(|error| error.to_string())?;
    assert_migration_front(&runtime, target, inspect.clone(), None)?;
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 1);
    assert_eq!(
        runtime.tick_shard(
            shard_error_index(source),
            ShardDirective::Migrate {
                target: shard_error_index(target),
            }
        ),
        Ok(false)
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(
        runtime
            .shards
            .get(target)
            .ok_or("target shard")?
            .enqueue(inspect.clone()),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run: RunId::new(28_001),
            pending: 2,
            limit: 2
        })
    );
    assert_migration_front(&runtime, target, inspect, None)?;
    assert_migration_front(&runtime, target, migration_expected_submit()?, Some(cost))?;
    Ok(())
}

#[test]
fn migration_tighter_target_limit_refuses_without_committing_prefix() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(4)?;
    // The charged submission would fit. A later command targets a run already
    // at its retained cap; no earlier source command may be published.
    let inspect = migration_expected_inspect(28_002);
    enqueue_migration_inspect(&runtime, source, 28_002)?;
    runtime
        .shards
        .get(target)
        .ok_or("target shard")?
        .command_queue
        .enqueue_with_limit(inspect.clone(), Some(1))
        .map_err(|error| error.to_string())?;
    let result = runtime.tick_shard(
        shard_error_index(source),
        ShardDirective::Migrate {
            target: shard_error_index(target),
        },
    );
    assert!(
        matches!(result, Err(RuntimeError::PerRunQueueDepthExceeded {
        run, limit: 1, ..
    }) if run == RunId::new(28_002))
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_queue_depth(&runtime, source)?, 2);
    assert_eq!(migration_queue_depth(&runtime, target)?, 1);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 0);
    assert_eq!(migration_pending(&runtime, source, 28_002)?, 1);
    assert_eq!(migration_pending(&runtime, target, 28_002)?, 1);
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, source, inspect.clone(), None)?;
    assert_migration_front(&runtime, target, inspect, None)?;
    Ok(())
}

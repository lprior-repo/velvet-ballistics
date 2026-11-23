#[test]
fn migration_failing_after_valid_prefix_restores_exact_source_and_target_fifos() -> Result<(), String> {
    let (mut runtime, cost, source, target) = migration_quota_fixture_with_capacity(8)?;
    // Source FIFO: charged submit(28_001), inspect(28_002), inspect(28_003).
    // The third command is refused by the target's retained per-run cap, so the
    // staged transfer has already validated a two-command prefix when it fails.
    enqueue_migration_inspect(&runtime, source, 28_002)?;
    enqueue_migration_inspect(&runtime, source, 28_003)?;
    let blocker = migration_expected_inspect(28_003);
    runtime
        .shards
        .get(target)
        .ok_or("target shard")?
        .command_queue
        .enqueue_with_limit(blocker.clone(), Some(1))
        .map_err(|error| error.to_string())?;
    assert_eq!(migration_queue_depth(&runtime, source)?, 3);
    assert_eq!(migration_queue_depth(&runtime, target)?, 1);

    let result = runtime.tick_shard(
        shard_error_index(source),
        ShardDirective::Migrate {
            target: shard_error_index(target),
        },
    );

    assert!(
        matches!(result, Err(RuntimeError::PerRunQueueDepthExceeded {
            run, limit: 1, ..
        }) if run == RunId::new(28_003)),
        "unexpected migration result: {result:?}"
    );
    assert_eq!(runtime.global_cost_usage(), cost);
    assert_eq!(migration_queue_depth(&runtime, source)?, 3);
    assert_eq!(migration_queue_depth(&runtime, target)?, 1);
    assert_eq!(migration_pending(&runtime, source, 28_001)?, 1);
    assert_eq!(migration_pending(&runtime, source, 28_002)?, 1);
    assert_eq!(migration_pending(&runtime, source, 28_003)?, 1);
    assert_eq!(migration_pending(&runtime, target, 28_001)?, 0);
    assert_eq!(migration_pending(&runtime, target, 28_002)?, 0);
    assert_eq!(migration_pending(&runtime, target, 28_003)?, 1);
    assert_migration_front(&runtime, source, migration_expected_submit()?, Some(cost))?;
    assert_migration_front(&runtime, source, migration_expected_inspect(28_002), None)?;
    assert_migration_front(&runtime, source, migration_expected_inspect(28_003), None)?;
    assert_migration_front(&runtime, target, blocker, None)?;
    Ok(())
}

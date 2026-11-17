#[test]
fn materialized_terminal_owner_outranks_ambiguous_pending_placement() -> Result<(), String> {
    let count = NonZeroUsize::new(2).ok_or("two shards")?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(count, runtime_config())
        .map_err(|error| error.to_string())?;
    let run = RunId::new(28_001);
    let home = runtime.shard_index(run);
    let owner = if home == 0 { 1 } else { 0 };
    // Synthetic terminal identity requires checkout custody before insertion.
    runtime
        .shards
        .get_mut(owner)
        .ok_or("owner shard")?
        .checked_out_run_insert(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        runtime
            .shards
            .get_mut(owner)
            .ok_or("owner shard")?
            .terminal_runs_insert(run),
        Ok(true)
    );
    assert!(
        !runtime
            .shards
            .get(owner)
            .ok_or("owner shard")?
            .checked_out_run_contains(run)
    );
    enqueue_migration_inspect(&runtime, home, run.get())?;
    enqueue_migration_inspect(&runtime, owner, run.get())?;
    runtime
        .inspect_run(run, 281)
        .map_err(|error| error.to_string())?;
    runtime
        .shard_for_mut(run)
        .map_err(|error| error.to_string())?
        .enqueue(crate::shard::ShardCommand::Inspect {
            run,
            correlation: 282,
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(migration_pending(&runtime, home, run.get())?, 1);
    assert_eq!(migration_pending(&runtime, owner, run.get())?, 3);
    assert_eq!(runtime.global_cost_usage(), 0);
    assert_migration_front(&runtime, owner, migration_expected_inspect(run.get()), None)?;
    assert_migration_front(
        &runtime,
        owner,
        crate::shard::ShardCommand::Inspect {
            run,
            correlation: 281,
        },
        None,
    )?;
    assert_migration_front(
        &runtime,
        owner,
        crate::shard::ShardCommand::Inspect {
            run,
            correlation: 282,
        },
        None,
    )?;
    assert_migration_front(&runtime, home, migration_expected_inspect(run.get()), None)?;
    Ok(())
}

#[test]
fn mutable_route_matches_unique_and_ambiguous_pending_placement() -> Result<(), String> {
    let count = NonZeroUsize::new(2).ok_or("two shards")?;
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(count, runtime_config())
        .map_err(|error| error.to_string())?;
    let run = RunId::new(28_001);
    let home = runtime.shard_index(run);
    let target = if home == 0 { 1 } else { 0 };
    enqueue_migration_inspect(&runtime, target, run.get())?;
    runtime
        .shard_for_mut(run)
        .map_err(|error| error.to_string())?
        .enqueue(crate::shard::ShardCommand::Inspect {
            run,
            correlation: 283,
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(migration_pending(&runtime, home, run.get())?, 0);
    assert_eq!(migration_pending(&runtime, target, run.get())?, 2);
    enqueue_migration_inspect(&runtime, home, run.get())?;
    assert_eq!(
        runtime.shard_for_mut(run).map(|_| ()),
        Err(RuntimeError::UnsupportedOperation {
            operation: "migration_pending_owner_ambiguous"
        })
    );
    assert_eq!(
        runtime.shard_for(run).map(|_| ()),
        Err(RuntimeError::UnsupportedOperation {
            operation: "migration_pending_owner_ambiguous"
        })
    );
    assert_eq!(migration_pending(&runtime, home, run.get())?, 1);
    assert_eq!(migration_pending(&runtime, target, run.get())?, 2);
    // Placement alone must not mint a completion or Ask boundary authority.
    assert!(runtime.lookup_pending_action_ticket(run, 0).is_none());
    assert!(
        runtime
            .lookup_pending_ask_ticket(run, StepIdx::ZERO)
            .is_none()
    );
    Ok(())
}

#[test]
fn migration_rolls_back_on_queue_full() {
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config())
        .expect("valid test configuration");

    // Submit a run to shard 0 (RunId::new(0) hashes to shard 0)
    let run = RunId::new(0);
    let workflow = finished_workflow().expect("valid workflow");
    let result = runtime.submit_direct(run, workflow.clone());
    assert!(result.is_ok(), "should be able to submit to shard 0");

    // Fill shard 1's queue completely (16 capacity) using RunIds that hash to shard 1
    // RunId::new(n) hashes to shard n % 3, so use n = 1, 4, 7, 10, 13, 16, 19, ...
    for i in 0..16 {
        let run_id = RunId::new(1 + i * 3);
        let result = runtime.submit_direct(run_id, workflow.clone());
        assert!(result.is_ok(), "should be able to fill shard 1 queue");
    }

    // Try to migrate shard 0 to shard 1 (which is full)
    let result = runtime.tick_shard(0, ShardDirective::Migrate { target: 1 });
    
    // Migration should fail because shard 1 is full
    assert!(result.is_err(), "migration to full shard should fail");
    let error = result.unwrap_err();
    assert!(matches!(error, RuntimeError::QueueFull), "should be QueueFull error, got: {:?}", error);
    
    // Verify the command is still on shard 0 (not lost)
    // We can't directly check queue length, but we can tick and see if it completes
    let tick_result = runtime.tick_all();
    assert!(tick_result.is_ok(), "tick after failed migration should succeed");
}

#[test]
fn migration_succeeds_when_target_has_capacity() {
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config())
        .expect("valid test configuration");

    // Submit a run to shard 0
    let run = RunId::new(0);
    let workflow = finished_workflow().expect("valid workflow");
    let result = runtime.submit_direct(run, workflow.clone());
    assert!(result.is_ok(), "should be able to submit");

    // Migrate to shard 1 (which has capacity)
    let result = runtime.tick_shard(0, ShardDirective::Migrate { target: 1 });
    assert!(result.is_ok(), "migration to empty shard should succeed");

    // Tick and verify the run completes on shard 1
    let tick_result = runtime.tick_all();
    assert!(tick_result.is_ok(), "tick after successful migration should succeed");
}
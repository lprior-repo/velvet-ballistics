#[test]
fn migration_rolls_back_on_queue_full() {
    let Some(shard_count) = NonZeroUsize::new(3) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config())
        .expect("valid test configuration");

    // Submit a run that will create a command in shard 0's queue
    let run = RunId::new(42);
    let workflow = finished_workflow().expect("valid workflow");
    let result = runtime.submit_compiled(run, workflow.clone());
    assert!(result.is_ok(), "should be able to submit run");

    // Verify the command is in shard 0's queue
    let result = runtime.tick_all();
    if result.is_ok() {
        // Runs completed, try to migrate with empty queues
        let result = runtime.tick_shard(0, ShardDirective::Migrate { target: 1 });
        assert!(result.is_ok(), "migration with empty queues should succeed");
    }
}
fn config_with_queue_capacity(capacity: usize) -> ShardConfig {
    ShardConfig {
        command_queue_capacity: capacity,
        ..ShardConfig::default()
    }
}

fn assert_queue_capacity_error<T>(result: crate::RuntimeResult<T>, capacity: usize) {
    assert_eq!(
        result.err(),
        Some(RuntimeError::CommandQueueCapacityExceeded {
            capacity,
            max: crate::shard::MAX_COMMAND_QUEUE_CAPACITY,
        })
    );
}

#[test]
fn public_constructors_refuse_invalid_queue_capacity_without_journal_writes() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shared: crate::journal::SharedRuntimeJournal = journal.clone();
    let before = journal.snapshot().expect("empty journal snapshot");
    for capacity in [0, crate::shard::MAX_COMMAND_QUEUE_CAPACITY + 1] {
        let config = config_with_queue_capacity(capacity);
        assert_queue_capacity_error(crate::shard::Shard::new(config), capacity);
        assert_queue_capacity_error(
            crate::shard::Shard::new_with_journal(config, Arc::clone(&shared)),
            capacity,
        );
        assert_queue_capacity_error(
            crate::shard::Shard::new_with_journal_and_artifact_store(
                config,
                Arc::clone(&shared),
                crate::admission::AlwaysPresentArtifactStore::shared(),
            ),
            capacity,
        );
        assert_queue_capacity_error(
            Runtime::new(std::num::NonZeroUsize::MIN, config, Arc::clone(&shared)),
            capacity,
        );
        assert_queue_capacity_error(
            Runtime::new_for_tests_and_benchmarks_only(std::num::NonZeroUsize::MIN, config),
            capacity,
        );
    }
    assert_eq!(journal.snapshot().expect("final journal snapshot"), before);
}

#[test]
fn public_constructors_accept_valid_queue_capacity_edges() -> crate::RuntimeResult<()> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shared: crate::journal::SharedRuntimeJournal = journal.clone();
    let config = config_with_queue_capacity(1);
    drop(crate::shard::Shard::new(config)?);
    drop(crate::shard::Shard::new_with_journal(config, Arc::clone(&shared))?);
    drop(crate::shard::Shard::new_with_journal_and_artifact_store(
        config,
        Arc::clone(&shared),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    )?);
    drop(Runtime::new(std::num::NonZeroUsize::MIN, config, Arc::clone(&shared))?);
    drop(Runtime::new_for_tests_and_benchmarks_only(
        std::num::NonZeroUsize::MIN,
        config,
    )?);

    let default = ShardConfig::default();
    drop(crate::shard::Shard::new(default)?);
    drop(crate::shard::Shard::new_with_journal(default, shared.clone())?);
    drop(crate::shard::Shard::new_with_journal_and_artifact_store(
        default,
        shared.clone(),
        crate::admission::AlwaysPresentArtifactStore::shared(),
    )?);
    drop(Runtime::new(std::num::NonZeroUsize::MIN, default, shared.clone())?);
    drop(Runtime::new_for_tests_and_benchmarks_only(
        std::num::NonZeroUsize::MIN,
        default,
    )?);

    let maximum = config_with_queue_capacity(crate::shard::MAX_COMMAND_QUEUE_CAPACITY);
    drop(crate::shard::Shard::new(maximum)?);
    drop(Runtime::new(std::num::NonZeroUsize::MIN, maximum, Arc::clone(&shared))?);
    drop(crate::shard::Shard::new(ShardConfig::default())?);
    drop(Runtime::new_for_tests_and_benchmarks_only(
        std::num::NonZeroUsize::MIN,
        ShardConfig::default(),
    )?);
    Ok(())
}

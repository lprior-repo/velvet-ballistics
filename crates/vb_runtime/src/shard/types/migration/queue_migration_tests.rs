use super::*;
use crate::RuntimeError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn inspect(run: u64, correlation: u64) -> ShardCommand {
    ShardCommand::Inspect {
        run: RunId::new(run),
        correlation,
    }
}

fn pop_correlation(queue: &ShardCommandQueue) -> Option<u64> {
    match queue.pop_queued()?.command {
        ShardCommand::Inspect { correlation, .. } => Some(correlation),
        _ => None,
    }
}

#[test]
fn staged_queue_fences_access_and_restores_fifo_and_indices() -> TestResult {
    let mut queue = ShardCommandQueue::new(3)?;
    queue.enqueue_with_limit(inspect(91, 11), Some(2))?;
    queue.enqueue_with_limit(inspect(91, 22), Some(2))?;
    queue.stage_migration_source()?;
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.pending_for(RunId::new(91)), 2);
    assert!(queue.pop_queued().is_none());
    assert_eq!(
        queue.enqueue(inspect(92, 33)),
        Err(RuntimeError::UnsupportedOperation {
            operation: "migration_queue_fenced",
        })
    );
    queue.restore_migration_source()?;
    assert_eq!(pop_correlation(&queue), Some(11));
    assert_eq!(pop_correlation(&queue), Some(22));
    assert_eq!(queue.pending_for(RunId::new(91)), 0);
    Ok(())
}

#[test]
fn index_commit_keeps_destination_prefix_and_source_depth_history() -> TestResult {
    let mut source = ShardCommandQueue::new(3)?;
    let mut target = ShardCommandQueue::new(3)?;
    target.enqueue(inspect(92, 10))?;
    source.enqueue_with_limit(inspect(91, 11), Some(2))?;
    source.enqueue_with_limit(inspect(91, 22), Some(2))?;
    source.stage_migration_source()?;
    target.prepare_migration_target(&source)?;
    target.transfer_staged_migration(&mut source)?;
    assert!(source.is_empty());
    assert_eq!(source.pending_for(RunId::new(91)), 0);
    assert_eq!(target.pending_for(RunId::new(91)), 2);
    assert_eq!(
        target.enqueue(inspect(91, 33)),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run: RunId::new(91),
            pending: 2,
            limit: 2,
        })
    );
    assert_eq!(pop_correlation(&target), Some(10));
    assert_eq!(pop_correlation(&target), Some(11));
    assert_eq!(pop_correlation(&target), Some(22));
    Ok(())
}

#[test]
fn rollback_of_partial_transfer_restores_both_fifos_and_indices() -> TestResult {
    let mut source = ShardCommandQueue::new(3)?;
    let mut target = ShardCommandQueue::new(3)?;
    target.enqueue(inspect(92, 10))?;
    source.enqueue(inspect(91, 11))?;
    source.enqueue(inspect(91, 22))?;
    source.stage_migration_source()?;
    target.prepare_migration_target(&source)?;
    assert_eq!(
        source
            .migration
            .stage
            .transfer_front(QueuePush(&target.inner)),
        TransferProgress::Accepted
    );
    target.rollback_migration_pair(&mut source)?;
    assert_eq!(source.pending_for(RunId::new(91)), 2);
    assert_eq!(target.pending_for(RunId::new(92)), 1);
    assert_eq!(target.pending_for(RunId::new(91)), 0);
    assert_eq!(pop_correlation(&target), Some(10));
    assert_eq!(pop_correlation(&source), Some(11));
    assert_eq!(pop_correlation(&source), Some(22));
    Ok(())
}

#[test]
fn partially_staged_source_stays_fenced_instead_of_reordering() -> TestResult {
    let mut queue = ShardCommandQueue::new(3)?;
    queue.enqueue(inspect(91, 11))?;
    queue.enqueue(inspect(91, 22))?;
    queue.fence_migration()?;
    assert_eq!(
        queue.migration.stage.fill_one(|| queue.inner.pop()),
        TransferProgress::Accepted
    );
    assert_eq!(
        queue.restore_migration_source(),
        Err(RuntimeError::UnsupportedOperation {
            operation: "migration_restore_without_complete_staging",
        })
    );
    assert!(queue.is_migration_fenced());
    assert_eq!(queue.migration.stage.entries().count(), 1);
    assert_eq!(queue.inner.len(), 1);
    assert_eq!(queue.pending_for(RunId::new(91)), 2);
    Ok(())
}

fn fenced_test_shard() -> Result<crate::shard::Shard, Box<dyn std::error::Error>> {
    crate::shard::Shard::new(crate::shard::ShardConfig {
        command_queue_capacity: 3,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        ..crate::shard::ShardConfig::default()
    })
    .map_err(Into::into)
}

fn assert_fenced_shard_refuses_progress(shard: &mut crate::shard::Shard) {
    let error = RuntimeError::UnsupportedOperation {
        operation: "migration_queue_fenced",
    };
    assert_eq!(shard.tick(), Err(error.clone()));
    assert_eq!(shard.quiesce_for_shutdown(), Err(error.clone()));
    assert_eq!(shard.drain_pending_and_shutdown(), Err(error));
    assert!(!shard.is_shutting_down());
    assert!(shard.command_queue.is_migration_fenced());
}

#[test]
fn empty_fenced_destination_refuses_tick_and_shutdown_before_empty_success() -> TestResult {
    let mut shard = fenced_test_shard()?;
    let run = RunId::new(93);
    let tenant = Some(vb_core::ids::TenantId::new(9));
    shard.reserve_admission_quota(run, tenant, 7)?;
    shard.command_queue.fence_migration()?;
    for _ in 0..2 {
        assert_fenced_shard_refuses_progress(&mut shard);
        assert_eq!(shard.command_queue.len(), 0);
        assert!(shard.command_queue.inner.is_empty());
        assert_eq!(shard.command_queue.migration.stage.len(), 0);
        assert_eq!(shard.global_cost_usage(), 7);
        assert_eq!(shard.charged_quotas.get(&run), Some(&(tenant, 7)));
        assert_eq!(shard.command_queue.pending_for(run), 0);
    }
    Ok(())
}

#[test]
fn partially_staged_source_tick_and_shutdown_preserve_envelope_custody() -> TestResult {
    let mut shard = fenced_test_shard()?;
    let quota = PendingQuotaReservation {
        tenant: Some(vb_core::ids::TenantId::new(9)),
        cost_units: 7,
    };
    // Synthetic charged envelope: this deliberately isolates custody, not a
    // valid submission or a reachable allocation failure. It must never execute.
    shard.command_queue.enqueue_with_reservation(
        inspect(91, 11),
        Some(2),
        Some(quota),
        Some(&shard.quota),
    )?;
    shard.command_queue.enqueue(inspect(91, 22))?;
    shard.command_queue.fence_migration()?;
    let queue = &mut shard.command_queue;
    assert_eq!(
        queue.migration.stage.fill_one(|| queue.inner.pop()),
        TransferProgress::Accepted
    );
    for _ in 0..2 {
        assert_fenced_shard_refuses_progress(&mut shard);
        assert_eq!(shard.global_cost_usage(), 7);
        assert_eq!(shard.command_queue.len(), 2);
        assert_eq!(shard.command_queue.pending_for(RunId::new(91)), 2);
        assert_eq!(shard.command_queue.inner.len(), 1);
        assert_eq!(shard.command_queue.migration.stage.len(), 1);
        let staged = shard
            .command_queue
            .migration_entries()
            .next()
            .ok_or("missing staged envelope")?;
        assert_eq!(staged.command, inspect(91, 11));
        assert_eq!(staged.quota, Some(quota));
        assert!(shard.command_queue.pop_queued().is_none());
        assert_eq!(
            shard.command_queue.restore_migration_source(),
            Err(RuntimeError::UnsupportedOperation {
                operation: "migration_restore_without_complete_staging",
            })
        );
    }
    // Inspect the remaining raw envelope only after all accounting assertions.
    // This is not a recovery/unfence operation and does not prove quota release.
    let raw = shard
        .command_queue
        .inner
        .pop()
        .ok_or("missing raw envelope")?;
    assert_eq!(raw.command, inspect(91, 22));
    assert_eq!(raw.quota, None);
    assert!(shard.command_queue.is_migration_fenced());
    Ok(())
}

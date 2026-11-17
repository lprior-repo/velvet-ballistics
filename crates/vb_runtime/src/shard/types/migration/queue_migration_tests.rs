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
            .transfer_front(|entry| target.inner.push(entry)),
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

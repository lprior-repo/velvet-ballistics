mod pop_critical_section_tests {
    use super::*;
    use std::sync::TryLockError;

    fn inspect(run: u64) -> ShardCommand {
        ShardCommand::Inspect {
            run: RunId::new(run),
            correlation: run,
        }
    }

    #[test]
    fn raw_slot_is_not_freed_outside_the_complete_index_critical_section() -> RuntimeResult<()> {
        let queue = ShardCommandQueue::new(1)?;
        queue.enqueue_with_limit(inspect(1), Some(1))?;
        let pending_capacity = ShardCommandQueue::lock_or_recover(&queue.pending).capacity();
        let admitted_capacity =
            ShardCommandQueue::lock_or_recover(&queue.admitted_depth).capacity();
        let mut observed = false;
        let dequeued = queue.pop_unfenced_queued_observed(|| {
            observed = true;
            // This is precisely the old Q+1 window: a raw slot is available,
            // but A has not yet been removed from its index maps. A producer
            // must not be able to acquire any part of the canonical bundle.
            assert_eq!(queue.inner.len(), 0);
            assert!(matches!(
                queue.pending.try_lock(),
                Err(TryLockError::WouldBlock)
            ));
            assert!(matches!(
                queue.admitted_depth.try_lock(),
                Err(TryLockError::WouldBlock)
            ));
            assert!(matches!(
                queue.pending_submissions.try_lock(),
                Err(TryLockError::WouldBlock)
            ));
        });
        assert!(observed);
        assert!(
            matches!(dequeued, Some(QueuedShardCommand { command, quota: None }) if command == inspect(1))
        );
        assert_eq!(queue.pending_for(RunId::new(1)), 0);
        assert_eq!(ShardCommandQueue::lock_or_recover(&queue.pending).len(), 0);
        assert_eq!(
            ShardCommandQueue::lock_or_recover(&queue.admitted_depth).len(),
            0
        );
        assert_eq!(
            ShardCommandQueue::lock_or_recover(&queue.pending_submissions).len(),
            0
        );
        // The real public path reuses the same helper and must release all
        // locks, permitting a distinct run to use the one raw slot afterward.
        queue.enqueue_with_limit(inspect(2), Some(1))?;
        assert_eq!(queue.pending_for(RunId::new(2)), 1);
        assert_eq!(queue.pop(), Some(inspect(2)));
        assert_eq!(queue.pending_for(RunId::new(2)), 0);
        assert_eq!(
            ShardCommandQueue::lock_or_recover(&queue.pending).capacity(),
            pending_capacity
        );
        assert_eq!(
            ShardCommandQueue::lock_or_recover(&queue.admitted_depth).capacity(),
            admitted_capacity
        );
        assert!(queue.is_empty());
        Ok(())
    }

    #[test]
    fn empty_pop_skips_observer_and_releases_index_guards() -> RuntimeResult<()> {
        let queue = ShardCommandQueue::new(1)?;
        let mut observed = false;
        assert!(
            queue
                .pop_unfenced_queued_observed(|| observed = true)
                .is_none()
        );
        assert!(!observed);
        assert!(queue.pending.try_lock().is_ok());
        assert!(queue.admitted_depth.try_lock().is_ok());
        assert!(queue.pending_submissions.try_lock().is_ok());
        queue.enqueue(inspect(3))?;
        assert_eq!(queue.pop(), Some(inspect(3)));
        Ok(())
    }
}

// Pure checked index planning for pending-command migration.
use crate::{RuntimeError, RuntimeResult};
use vb_core::RunId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MigrationRunCounts {
    pub(crate) pending: u32,
    pub(crate) submissions: u32,
    pub(crate) retained_depth: Option<u32>,
}

pub(crate) fn plan_migration_run_counts(
    run: RunId,
    target: MigrationRunCounts,
    source: MigrationRunCounts,
    destination_depth: Option<u32>,
) -> RuntimeResult<MigrationRunCounts> {
    validate_migration_run_counts(target, source)?;
    let depth = [
        target.retained_depth,
        source.retained_depth,
        destination_depth,
    ]
    .into_iter()
    .flatten()
    .min();
    let pending = checked_migration_count(run, target.pending, source.pending)?;
    check_migration_depth(run, target.pending, pending, depth)?;
    let submissions = checked_migration_count(run, target.submissions, source.submissions)?;
    Ok(MigrationRunCounts {
        pending,
        submissions,
        retained_depth: depth,
    })
}

fn check_migration_depth(
    run: RunId,
    target_pending: u32,
    pending: u32,
    depth: Option<u32>,
) -> RuntimeResult<()> {
    if let Some(limit) = depth {
        if pending > limit {
            return Err(RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: target_pending,
                limit,
            });
        }
    }
    Ok(())
}

fn validate_migration_run_counts(
    target: MigrationRunCounts,
    source: MigrationRunCounts,
) -> RuntimeResult<()> {
    if source.pending == 0
        || source.submissions > source.pending
        || target.submissions > target.pending
        || (target.pending == 0 && target.retained_depth.is_some())
    {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "migration_queue_index_invariant",
        });
    }
    Ok(())
}

fn checked_migration_count(run: RunId, target: u32, source: u32) -> RuntimeResult<u32> {
    target
        .checked_add(source)
        .ok_or(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: target,
            limit: u32::MAX,
        })
}

pub(crate) fn check_migration_queue_capacity(
    target_len: usize,
    source_len: usize,
    target_capacity: usize,
) -> RuntimeResult<()> {
    match target_len.checked_add(source_len) {
        Some(total) if total <= target_capacity => Ok(()),
        _ => Err(RuntimeError::QueueFull),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tighter_source_history_applies_to_existing_target_backlog() {
        let run = RunId::new(81_101);
        let target = MigrationRunCounts {
            pending: 2,
            submissions: 1,
            retained_depth: Some(8),
        };
        let source = MigrationRunCounts {
            pending: 1,
            submissions: 1,
            retained_depth: Some(2),
        };
        assert_eq!(
            plan_migration_run_counts(run, target, source, None),
            Err(RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: 2,
                limit: 2
            })
        );
        assert_eq!(target.pending, 2);
        assert_eq!(source.pending, 1);
    }

    #[test]
    fn exact_depth_and_duplicate_submission_counts_are_preserved() {
        let run = RunId::new(81_102);
        let target = MigrationRunCounts {
            pending: 2,
            submissions: 1,
            retained_depth: Some(8),
        };
        let source = MigrationRunCounts {
            pending: 1,
            submissions: 1,
            retained_depth: Some(3),
        };
        assert_eq!(
            plan_migration_run_counts(run, target, source, Some(4)),
            Ok(MigrationRunCounts {
                pending: 3,
                submissions: 2,
                retained_depth: Some(3)
            })
        );
    }

    #[test]
    fn queue_capacity_and_count_overflow_fail_closed() {
        assert_eq!(check_migration_queue_capacity(2, 1, 3), Ok(()));
        assert_eq!(
            check_migration_queue_capacity(2, 2, 3),
            Err(RuntimeError::QueueFull)
        );
        assert_eq!(
            check_migration_queue_capacity(usize::MAX, 1, usize::MAX),
            Err(RuntimeError::QueueFull)
        );
        let run = RunId::new(81_103);
        assert_eq!(
            checked_migration_count(run, u32::MAX, 1),
            Err(RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: u32::MAX,
                limit: u32::MAX
            })
        );
    }

    #[test]
    fn invalid_source_and_destination_depth_are_explicit() {
        let run = RunId::new(81_104);
        let empty = MigrationRunCounts {
            pending: 0,
            submissions: 0,
            retained_depth: None,
        };
        assert_eq!(
            plan_migration_run_counts(run, empty, empty, None),
            Err(RuntimeError::UnsupportedOperation {
                operation: "migration_queue_index_invariant"
            })
        );
        let source = MigrationRunCounts {
            pending: 1,
            submissions: 2,
            retained_depth: None,
        };
        assert_eq!(
            plan_migration_run_counts(run, empty, source, None),
            Err(RuntimeError::UnsupportedOperation {
                operation: "migration_queue_index_invariant"
            })
        );
        let source = MigrationRunCounts {
            pending: 1,
            submissions: 1,
            retained_depth: None,
        };
        assert_eq!(
            plan_migration_run_counts(run, empty, source, Some(0)),
            Err(RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: 0,
                limit: 0
            })
        );
        assert_eq!(
            plan_migration_run_counts(run, empty, source, None),
            Ok(source)
        );
        assert_eq!(
            plan_migration_run_counts(run, empty, source, Some(1)),
            Ok(MigrationRunCounts {
                retained_depth: Some(1),
                ..source
            })
        );
    }
}

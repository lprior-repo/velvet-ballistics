use super::{PendingQuotaReservation, ShardCommand, ShardCommandQueue, pending_quota};
use crate::quota::SharedQuotaLedger;
use crate::RuntimeError;
use vb_core::ids::{RunId, TenantId};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn reservation() -> PendingQuotaReservation {
    PendingQuotaReservation { tenant: Some(TenantId::new(97)), cost_units: 7 }
}

// Synthetic reservation injection isolates queue custody, not submit admission.
// No Shard, migration, Store, replay or runtime tick is exercised.
#[test]
fn dequeue_returns_custody_without_releasing_the_ledger() -> TestResult {
    let queue = ShardCommandQueue::new(2)?;
    let ledger = SharedQuotaLedger::new();
    ledger.set_limits(20, 20)?;
    let empty = ledger.usage();
    let run = RunId::new(82_101);
    let reserved = reservation();
    queue.enqueue_with_reservation(
        ShardCommand::Inspect { run, correlation: 41 },
        Some(2), Some(reserved), Some(&ledger),
    )?;
    let charged = ledger.usage();
    assert_eq!(ledger.global_usage(), 7);
    assert_eq!(queue.pending_for(run), 1);
    let queued = queue.pop_queued().ok_or("missing queued command")?;
    assert!(matches!(queued.command,
        ShardCommand::Inspect { run: actual, correlation: 41 } if actual == run));
    assert_eq!(queued.quota, Some(reserved));
    assert_eq!(queue.pending_for(run), 0);
    assert!(queue.is_empty());
    assert_eq!(ledger.usage(), charged);
    assert_eq!(ledger.global_usage(), 7);
    let custody = queued.quota.ok_or("missing reservation custody")?;
    ledger.release(custody.tenant, custody.cost_units)?;
    assert_eq!(ledger.usage(), empty);
    assert_eq!(ledger.global_usage(), 0);
    assert!(queue.pop_queued().is_none());
    Ok(())
}

#[test]
fn full_queue_rolls_back_only_the_rejected_reservation() -> TestResult {
    let queue = ShardCommandQueue::new(1)?;
    let ledger = SharedQuotaLedger::new();
    ledger.set_limits(20, 20)?;
    let first = RunId::new(82_102);
    let rejected = RunId::new(82_103);
    let reserved = reservation();
    queue.enqueue_with_reservation(
        ShardCommand::Inspect { run: first, correlation: 43 },
        None, Some(reserved), Some(&ledger),
    )?;
    let charged = ledger.usage();
    assert_eq!(queue.enqueue_with_reservation(
        ShardCommand::Inspect { run: rejected, correlation: 47 },
        None, Some(reserved), Some(&ledger),
    ), Err(RuntimeError::QueueFull));
    assert_eq!(ledger.usage(), charged);
    assert_eq!(ledger.global_usage(), 7);
    assert_eq!(queue.pending_for(first), 1);
    assert_eq!(queue.pending_for(rejected), 0);
    let queued = queue.pop_queued().ok_or("missing original command")?;
    assert!(matches!(queued.command,
        ShardCommand::Inspect { run, correlation: 43 } if run == first));
    assert_eq!(queued.quota, Some(reserved));
    ledger.release(reserved.tenant, reserved.cost_units)?;
    assert_eq!(ledger.global_usage(), 0);
    Ok(())
}

#[test]
fn duplicate_submit_reservation_selection_preserves_first_owner() {
    let run = RunId::new(82_104);
    let mut pending = indexmap::IndexMap::new();
    let candidate = Some(reservation());
    assert_eq!(pending_quota(Some(run), &pending, candidate), candidate);
    pending.insert(run, 1);
    assert_eq!(pending_quota(Some(run), &pending, candidate), None);
    pending.insert(run, 2);
    assert_eq!(pending_quota(Some(run), &pending, candidate), None);
    pending.swap_remove(&run);
    assert_eq!(pending_quota(Some(run), &pending, candidate), candidate);
}

#[test]
fn zero_cost_reservation_retains_some_identity() -> TestResult {
    let queue = ShardCommandQueue::new(2)?;
    let ledger = SharedQuotaLedger::new();
    ledger.set_limits(0, 0)?;
    let run = RunId::new(82_105);
    let zero = PendingQuotaReservation { tenant: Some(TenantId::new(97)), cost_units: 0 };
    queue.enqueue_with_reservation(
        ShardCommand::Inspect { run, correlation: 51 },
        None, Some(zero), Some(&ledger),
    )?;
    queue.enqueue(ShardCommand::Inspect { run, correlation: 53 })?;
    let first = queue.pop_queued().ok_or("missing zero reservation")?;
    assert_eq!(first.quota, Some(zero));
    assert!(matches!(first.command, ShardCommand::Inspect { correlation: 51, .. }));
    let second = queue.pop_queued().ok_or("missing unreserved command")?;
    assert_eq!(second.quota, None);
    assert!(matches!(second.command, ShardCommand::Inspect { correlation: 53, .. }));
    assert_eq!(ledger.global_usage(), 0);
    assert_eq!(queue.pending_for(run), 0);
    Ok(())
}

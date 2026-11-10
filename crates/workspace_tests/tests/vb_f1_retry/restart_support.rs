use super::*;
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use vb_runtime::runtime::Runtime;
use vb_storage::{FjallJournal, retry_timer::RetryTimerV1};

pub(super) fn open(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let runtime = runtime_with_policy(
        StorageRuntimeJournal::shared_strict(journal.clone()),
        RuntimePolicy::Journaled,
    )?;
    Ok((journal, runtime))
}
pub(super) fn last_timer(events: &[JournalEvent]) -> TestResult<RetryTimerV1> {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RetryTimerScheduled { timer, .. } => Some(timer.clone()),
            _ => None,
        })
        .ok_or_else(|| "missing durable retry timer".into())
}
pub(super) fn wait_for_action(runtime: &mut Runtime, timer: &RetryTimerV1) -> TestResult {
    let start = Instant::now();
    while runtime.lookup_pending_action_ticket(RUN, 1).is_none()
        && start.elapsed() < Duration::from_secs(10)
    {
        runtime.tick_all()?;
    }
    let current = ticket(runtime)?;
    assert_eq!(current.attempt, timer.next_attempt);
    assert_eq!(current.capacity, timer.failed_ticket.capacity);
    assert_eq!(current.idempotency_key, timer.failed_ticket.idempotency_key);
    assert!(
        u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?
            >= timer.deadline_unix_ms
    );
    Ok(())
}
pub(super) fn recover(runtime: &mut Runtime, kind: RecoveredRunBoundaryKind) -> TestResult {
    let product = runtime.recover_product(RUN)?;
    let vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(product) = product else {
        return Err(format!("not resumable: {product:?}").into());
    };
    assert_eq!(product.boundary_kind(), kind);
    runtime.recover_and_resume(RUN)?;
    runtime.tick_all()?;
    Ok(())
}

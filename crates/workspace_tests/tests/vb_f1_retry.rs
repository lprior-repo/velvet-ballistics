#![forbid(unsafe_code)]
#[path = "vb_f1_retry/support.rs"]
mod support;
use support::*;
use vb_core::{
    CompiledNodeKind, StepIdx, action::ActionFailureCode, workflow::CompiledRetryPolicy,
};
use vb_runtime::RuntimeError;
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_storage::JournalEvent;
#[path = "vb_f1_retry/rejection_cases.rs"]
mod rejection_cases;

#[test]
fn yaml_retry_success_continues_once_without_retry_check() -> TestResult {
    let workflow = compile(3, "0ms", false)?;
    assert_eq!(
        workflow.node(StepIdx::new(1)).ok_or("missing Do")?.retry,
        Some(CompiledRetryPolicy {
            max_attempts: 3,
            delay_ms: 0
        })
    );
    assert!(
        !workflow
            .to_parts()
            .nodes
            .iter()
            .any(|node| matches!(node.kind, CompiledNodeKind::RetryCheck { .. }))
    );
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = runtime(journal.clone())?;
    submit(&mut runtime, workflow)?;
    assert_eq!(ticket(&runtime)?.capacity, 3);
    succeed(&mut runtime)?;
    let events = journal.snapshot()?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeJournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeJournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
    Ok(())
}

#[test]
fn yaml_retry_preserves_key_and_stops_at_total_attempt_capacity() -> TestResult {
    for (handler, attempts) in [(false, 1), (true, 1), (false, 3), (true, 3)] {
        let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = runtime(journal.clone())?;
        submit(&mut runtime, compile(attempts, "0ms", handler)?)?;
        let first = ticket(&runtime)?;
        for attempt in 1..=attempts {
            let current = ticket(&runtime)?;
            assert_eq!(current.attempt, attempt);
            assert_eq!(current.capacity, attempts);
            assert_eq!(current.idempotency_key, first.idempotency_key);
            fail(&mut runtime, ActionFailureCode::Timeout)?;
        }
        assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
        let events = journal.snapshot()?;
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RuntimeJournalEvent::ActionScheduledTicket { .. }))
                .count(),
            usize::from(attempts)
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RuntimeJournalEvent::RunFinished { .. }))
                .count(),
            usize::from(handler)
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RuntimeJournalEvent::RunFailed { .. }))
                .count(),
            usize::from(!handler)
        );
    }
    Ok(())
}

#[test]
fn yaml_retry_transient_then_success_and_permanent_handler() -> TestResult {
    for (code, policy, permanent) in [
        (
            ActionFailureCode::Timeout,
            vb_core::action::RetryPolicy::Retryable,
            false,
        ),
        (
            ActionFailureCode::Rejected,
            vb_core::action::RetryPolicy::Retryable,
            true,
        ),
        (
            ActionFailureCode::Timeout,
            vb_core::action::RetryPolicy::NonRetryable,
            true,
        ),
    ] {
        let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = runtime(journal.clone())?;
        submit(&mut runtime, compile(3, "0ms", true)?)?;
        let mut failure: vb_core::action::ActionFailure = code.into();
        failure.retry_policy = policy;
        runtime.fail_action(ticket(&runtime)?, failure)?;
        runtime.tick_all()?;
        if !permanent {
            assert_eq!(ticket(&runtime)?.attempt, 2);
            succeed(&mut runtime)?;
        }
        let events = journal.snapshot()?;
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RuntimeJournalEvent::ActionScheduledTicket { .. }))
                .count(),
            if permanent { 1 } else { 2 }
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RuntimeJournalEvent::RunFinished { .. }))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn yaml_retry_delay_is_a_journaled_timer_before_next_action() -> TestResult {
    let (_directory, journal, mut runtime) = durable()?;
    let workflow = compile(2, "200ms", false)?;
    let parts = workflow.to_parts();
    assert!(
        vb_core::budget::WholeWorkflowBudget::compute(
            &parts.nodes,
            parts.entry,
            &parts.resource_contract
        )?
        .max_timer_entries
            > 0
    );
    submit(&mut runtime, workflow)?;
    fail(&mut runtime, ActionFailureCode::Timeout)?;
    assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
    let timer = runtime.capture_timer_entry(RUN)?;
    let events = journal.events_for_run(RUN)?;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, JournalEvent::WaitResolvedEvent { .. }))
    );
    // Advance through the public timer authority, without blocking the runtime thread.
    let start = std::time::Instant::now();
    while runtime.lookup_pending_action_ticket(RUN, 1).is_none()
        && start.elapsed() < std::time::Duration::from_secs(2)
    {
        runtime.tick_all()?;
    }
    let second = ticket(&runtime)?;
    assert!(std::time::Instant::now() >= timer.deadline);
    assert_eq!(second.attempt, 2);
    assert!(matches!(
        runtime.capture_timer_entry(RUN),
        Err(RuntimeError::InvalidTimerFire)
    ));
    succeed(&mut runtime)?;
    let events = journal.events_for_run(RUN)?;
    let resolved = events
        .iter()
        .position(|event| matches!(event, JournalEvent::WaitResolvedEvent { .. }))
        .ok_or("missing resolved timer")?;
    let schedule = events
        .iter()
        .rposition(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
        .ok_or("missing reschedule")?;
    assert!(resolved < schedule);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn retry_timer_journal_capacity_rejection_preserves_original_ticket() -> TestResult {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut probe = runtime(journal.clone())?;
    submit(&mut probe, compile(2, "1s", false)?)?;
    let initial_count = journal.snapshot()?.len();
    let capacity =
        std::num::NonZeroUsize::new(initial_count.checked_add(1).ok_or("capacity overflow")?)
            .ok_or("zero capacity")?;
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::with_capacity(capacity));
    let mut runtime = runtime(journal.clone())?;
    submit(&mut runtime, compile(2, "1s", false)?)?;
    let original = ticket(&runtime)?;
    let before = journal.snapshot()?;
    runtime.fail_action(original, ActionFailureCode::Timeout.into())?;
    assert!(
        matches!(runtime.tick_all(), Err(RuntimeError::JournalFull { capacity: actual }) if actual == capacity.get())
    );
    assert_eq!(journal.snapshot()?, before);
    assert_eq!(ticket(&runtime)?, original);
    assert!(matches!(
        runtime.capture_timer_entry(RUN),
        Err(RuntimeError::InvalidTimerFire)
    ));
    Ok(())
}

#[test]
fn retry_timer_rejects_early_and_forged_authority_without_rescheduling() -> TestResult {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = runtime(journal.clone())?;
    submit(&mut runtime, compile(2, "1h", false)?)?;
    fail(&mut runtime, ActionFailureCode::Timeout)?;
    let entry = runtime.capture_timer_entry(RUN)?;
    let before = journal.snapshot()?;
    runtime.timer_entry_fired(entry)?;
    assert!(matches!(
        runtime.tick_all(),
        Err(RuntimeError::InvalidTimerFire)
    ));
    assert_eq!(runtime.capture_timer_entry(RUN)?, entry);
    let mut forged = entry;
    forged.generation = forged
        .generation
        .checked_add(1)
        .ok_or("generation overflow")?;
    runtime.timer_entry_fired(forged)?;
    assert!(matches!(
        runtime.tick_all(),
        Err(RuntimeError::InvalidTimerFire)
    ));
    assert_eq!(runtime.capture_timer_entry(RUN)?, entry);
    assert_eq!(journal.snapshot()?, before);
    assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
    Ok(())
}

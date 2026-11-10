#![forbid(unsafe_code)]
#[path = "vb_f1_retry/support.rs"]
mod support;
use support::*;

use vb_core::{RuntimePolicy, action::ActionFailureCode};
use vb_runtime::RuntimeError;
use vb_runtime::{journal::StorageRuntimeJournal, recovery::RecoveredRunBoundaryKind};
use vb_storage::JournalEvent;

#[path = "vb_f1_retry/restart_support.rs"]
mod restart_support;
use restart_support::*;

#[test]
fn fjall_retry_delay_survives_waiting_and_action_restarts_then_finishes_once() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (prefix, timer) = {
        let (journal, mut runtime) = open(directory.path())?;
        let workflow = compile(3, "2s", false)?;
        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Journaled,
            &[contract()?],
        )?;
        submit(&mut runtime, workflow)?;
        fail(&mut runtime, ActionFailureCode::Timeout)?;
        let prefix = journal.events_for_run_full(RUN)?;
        let timer = last_timer(&prefix)?;
        timer.validate()?;
        assert_eq!(timer.version, 1);
        assert_eq!(timer.next_attempt, 2);
        assert_eq!(
            timer.deadline_unix_ms.checked_sub(timer.scheduled_unix_ms),
            Some(2_000)
        );
        assert_eq!(
            runtime.capture_timer_entry(RUN)?.generation,
            timer.generation
        );
        (prefix, timer)
    };
    let second = {
        let (journal, mut runtime) = open(directory.path())?;
        recover(&mut runtime, RecoveredRunBoundaryKind::RetryTimer)?;
        assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
        let entry = runtime.capture_timer_entry(RUN)?;
        assert_eq!(entry.generation, timer.generation);
        assert_eq!(journal.events_for_run_full(RUN)?, prefix);
        runtime.timer_entry_fired(entry)?;
        assert!(matches!(
            runtime.tick_all(),
            Err(RuntimeError::InvalidTimerFire)
        ));
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
        assert_eq!(journal.events_for_run_full(RUN)?, prefix);
        wait_for_action(&mut runtime, &timer)?;
        restart_integrity::reject_duplicate_action_schedule(&journal.events_for_run_full(RUN)?)?;
        ticket(&runtime)?
    };
    {
        let (journal, mut runtime) = open(directory.path())?;
        let before = journal.events_for_run_full(RUN)?;
        recover(&mut runtime, RecoveredRunBoundaryKind::PendingAction)?;
        assert_eq!(ticket(&runtime)?, second);
        assert_eq!(journal.events_for_run_full(RUN)?, before);
        succeed(&mut runtime)?;
        let events = journal.events_for_run_full(RUN)?;
        assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
                .count(),
            1
        );
        assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    }
    Ok(())
}

#[test]
fn fjall_retry_attempt_and_generation_survive_repeated_restart_until_exhaustion() -> TestResult {
    for handler in [false, true] {
        let directory = tempfile::tempdir()?;
        let first_generation = {
            let (journal, mut runtime) = open(directory.path())?;
            let workflow = compile(3, "200ms", handler)?;
            vb_storage::submit_artifact_with_contracts(
                &journal,
                &workflow,
                RuntimePolicy::Journaled,
                &[contract()?],
            )?;
            submit(&mut runtime, workflow)?;
            fail(&mut runtime, ActionFailureCode::Timeout)?;
            last_timer(&journal.events_for_run_full(RUN)?)?.generation
        };
        for expected in [2, 3] {
            let (journal, mut runtime) = open(directory.path())?;
            recover(&mut runtime, RecoveredRunBoundaryKind::RetryTimer)?;
            let timer = last_timer(&journal.events_for_run_full(RUN)?)?;
            assert_eq!(timer.next_attempt, expected);
            if expected == 3 {
                assert!(timer.generation > first_generation);
            }
            wait_for_action(&mut runtime, &timer)?;
            fail(&mut runtime, ActionFailureCode::Timeout)?;
            if expected == 3 {
                assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
                assert!(matches!(
                    runtime.capture_timer_entry(RUN),
                    Err(RuntimeError::InvalidTimerFire)
                ));
                let events = journal.events_for_run_full(RUN)?;
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
                        .count(),
                    3
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
                        .count(),
                    2
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
                        .count(),
                    usize::from(handler)
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, JournalEvent::RunFailedEvent { .. }))
                        .count(),
                    usize::from(!handler)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn legacy_wait_record_cannot_authorize_a_retry_deadline() -> TestResult {
    let (_directory, journal, _runtime) = durable()?;
    let mut runtime = runtime_with_policy(
        StorageRuntimeJournal::shared_strict(journal.clone()),
        RuntimePolicy::Journaled,
    )?;
    let workflow = compile(2, "1h", false)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    submit(&mut runtime, workflow.clone())?;
    fail(&mut runtime, ActionFailureCode::Timeout)?;
    let events = journal.events_for_run_full(RUN)?;
    let other = tempfile::tempdir()?;
    let (target, mut recovered) = open(other.path())?;
    vb_storage::submit_artifact_with_contracts(
        &target,
        &workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    for event in events {
        let old = match event {
            JournalEvent::RetryTimerScheduled {
                run,
                seq,
                step,
                attempt,
                ..
            } => JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step,
                attempt,
            },
            event => event,
        };
        target.append_strict(&old)?;
    }
    let product = recovered.recover_product(RUN)?;
    assert!(!product.is_resumable());
    assert_eq!(product.cannot_resume_reason(), Some("pending_timers"));
    assert!(
        matches!(recovered.recover_and_resume(RUN), Err(RuntimeError::RecoveryCannotResume { reason }) if reason == "pending_timers")
    );
    recovered.tick_all()?;
    assert!(matches!(
        recovered.lookup_pending_action_ticket(RUN, 1),
        None
    ));
    Ok(())
}

#[path = "vb_f1_retry/restart_integrity.rs"]
mod restart_integrity;

#[path = "vb_f1_retry/cooperative_completion.rs"]
mod cooperative_completion;

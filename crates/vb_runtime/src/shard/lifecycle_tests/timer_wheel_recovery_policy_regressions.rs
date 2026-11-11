#[test]
fn cold_fjall_recovery_rearms_wait_then_resolves_once() -> ColdTimerResult {
    let expected = ColdTimerExpectation {
        run: vb_core::RunId::new(86_014),
        step: vb_core::StepIdx::new(1),
        kind: crate::shard::types::PendingTimerKind::Wait,
        timeout: std::time::Duration::ZERO,
    };
    let events = cold_recovery_case(
        timer_wait_workflow().ok_or("wait workflow missing")?,
        expected,
    )?;
    assert_eq!(events.iter().filter(|event| matches!(event, vb_storage::JournalEvent::WaitResolvedEvent { step, .. } if *step == expected.step)).count(), 1);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
    );
    Ok(())
}

#[test]
fn cold_fjall_recovery_rearms_timed_ask_then_records_timeout() -> ColdTimerResult {
    let expected = ColdTimerExpectation {
        run: vb_core::RunId::new(86_015),
        step: vb_core::StepIdx::new(2),
        kind: crate::shard::types::PendingTimerKind::Ask,
        timeout: std::time::Duration::ZERO,
    };
    let events = cold_recovery_case(
        timer_ask_workflow().ok_or("ask workflow missing")?,
        expected,
    )?;
    assert_eq!(events.iter().filter(|event| matches!(event, vb_storage::JournalEvent::AskTimedOutEvent { step, .. } if *step == expected.step)).count(), 1);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
    );
    Ok(())
}

#[test]
fn cold_fjall_wait_recovery_preserves_positive_timeout_policy() -> ColdTimerResult {
    let expected = ColdTimerExpectation {
        run: vb_core::RunId::new(86_016),
        step: vb_core::StepIdx::new(1),
        kind: crate::shard::types::PendingTimerKind::Wait,
        timeout: std::time::Duration::from_millis(25),
    };
    cold_recovery_policy_case(
        timer_wait_workflow().ok_or("wait workflow missing")?,
        expected,
    )
}

#[test]
fn cold_fjall_timed_ask_recovery_preserves_positive_timeout_policy() -> ColdTimerResult {
    let expected = ColdTimerExpectation {
        run: vb_core::RunId::new(86_017),
        step: vb_core::StepIdx::new(2),
        kind: crate::shard::types::PendingTimerKind::Ask,
        timeout: std::time::Duration::from_millis(25),
    };
    cold_recovery_policy_case(
        timer_ask_workflow().ok_or("ask workflow missing")?,
        expected,
    )
}

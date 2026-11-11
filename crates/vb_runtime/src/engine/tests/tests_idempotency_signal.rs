// =====================================================================
// Idempotency key tests
// =====================================================================

#[test]
fn compute_idempotency_key_is_deterministic() {
    let key1 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(2), ActionId::new(3));
    let key2 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(2), ActionId::new(3));
    assert_eq!(key1, key2);
}

#[test]
fn compute_idempotency_key_differs_for_different_runs() {
    let key1 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(0), ActionId::new(0));
    let key2 = compute_idempotency_key(RunId::new(2), 1, SeqNo::new(0), ActionId::new(0));
    assert_ne!(key1, key2);
}

#[test]
fn compute_idempotency_key_is_unique_for_different_seq() {
    let key1 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(0), ActionId::new(0));
    let key2 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(1), ActionId::new(0));
    assert_ne!(key1, key2);
}

#[test]
fn compute_idempotency_key_is_unique_for_different_action() {
    let key1 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(0), ActionId::new(0));
    let key2 = compute_idempotency_key(RunId::new(1), 1, SeqNo::new(0), ActionId::new(1));
    assert_ne!(key1, key2);
}

#[test]
fn compute_idempotency_key_handles_large_values_without_overflow() {
    let key = compute_idempotency_key(
        RunId::new(u64::MAX),
        1,
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    let key2 = compute_idempotency_key(
        RunId::new(u64::MAX),
        1,
        SeqNo::new(u64::MAX),
        ActionId::new(65535),
    );
    assert_eq!(key, key2);
}

// =====================================================================
// RuntimeSignal tests
// =====================================================================

#[test]
fn runtime_signal_equality_continue() {
    assert_eq!(RuntimeSignal::Continue, RuntimeSignal::Continue);
}

#[test]
fn runtime_signal_equality_exhausted() {
    assert_eq!(
        RuntimeSignal::StepBudgetExhausted,
        RuntimeSignal::StepBudgetExhausted
    );
}

#[test]
fn runtime_signal_equality_awaiting_wait() {
    assert_eq!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingWait);
}

#[test]
fn runtime_signal_equality_awaiting_ask() {
    assert_eq!(RuntimeSignal::AwaitingAsk, RuntimeSignal::AwaitingAsk);
}

#[test]
fn runtime_signal_differs_awaiting_wait_from_awaiting_ask() {
    assert_ne!(RuntimeSignal::AwaitingWait, RuntimeSignal::AwaitingAsk);
}

#[test]
fn runtime_signal_equality_differs_for_different_finished_values() {
    let a = RuntimeSignal::Finished(SlotValue::I64(1));
    let b = RuntimeSignal::Finished(SlotValue::I64(2));
    assert_ne!(a, b);
}

#[test]
fn runtime_signal_awaiting_action_equality_matches_on_ticket() {
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(0),
        action: ActionId::new(5),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 42,
        capacity: 1,
        fencing_token: 0,
    };
    let a = RuntimeSignal::AwaitingAction(ticket);
    let b = RuntimeSignal::AwaitingAction(ticket);
    assert_eq!(a, b);
}

#[test]
fn runtime_signal_awaiting_action_differs_for_different_ticket() {
    let a = RuntimeSignal::AwaitingAction(ActionTicket {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    });
    let b = RuntimeSignal::AwaitingAction(ActionTicket {
        run: RunId::new(2),
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        fencing_token: 0,
    });
    assert_ne!(a, b);
}


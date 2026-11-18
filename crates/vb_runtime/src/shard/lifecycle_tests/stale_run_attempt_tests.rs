// Source-level regression fixtures. The controlled state edit below is not a
// claim that a retry/checkpoint/recovery transition has executed.
type StaleRunFixture = (
    Shard,
    std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    ActionTicket,
    ActionTicket,
);

fn stale_run_fixture() -> Result<StaleRunFixture, String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|error| format!("stale-run shard setup: {error:?}"))?;
    // Bind the real top-level completion guard to this existing allowed route.
    // This fixture deliberately does not simulate a packed registered executor.
    assert!(shard.artifact_store.executor_catalog().is_none());
    let run = RunId::new(61_201);
    let workflow = require_workflow("stale-run suspended", suspended_workflow())?;
    submit_run(&mut shard, run, workflow, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let old = shard.pending_action_get(run).ok_or("missing initial ticket")?;
    assert_eq!(old.run_attempt, 1);
    assert_eq!(old.attempt, 1);
    // Literal negative controls below must change the issued identity.
    assert_ne!(old.seq, SeqNo::new(1));
    assert_ne!(old.capacity, 2);
    assert_ne!(old.fencing_token, 0);
    assert!(vb_core::action::action_ticket_has_valid_key(old));
    let current = rekey_run_ticket(old, 2);
    let state = shard.run_state_get_mut(run).ok_or("missing live run")?;
    state.run_attempt = core::num::NonZeroU16::new(2).ok_or("invalid run attempt")?;
    assert_eq!(state.action_attempts.first().copied(), Some(1));
    assert_eq!(shard.pending_action_insert(run, current), Ok(Some(old)));
    assert_eq!(shard.require_pending_action_ownership(current), Ok(()));
    Ok((shard, journal, old, current))
}

fn rekey_run_ticket(mut ticket: ActionTicket, run_attempt: u16) -> ActionTicket {
    ticket.run_attempt = run_attempt;
    ticket.idempotency_key = vb_core::action::compute_action_idempotency_key(
        ticket.run, ticket.run_attempt, ticket.seq, ticket.action,
    );
    ticket
}

fn assert_completion_rejected_without_effects(
    shard: &mut Shard,
    journal: &crate::journal::VolatileRuntimeJournal,
    live: RunId,
    incoming: ActionTicket,
    expected: RuntimeError,
) -> Result<(), String> {
    assert_stale_route_rejected_without_effects(shard, journal, live, expected, |shard| {
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
            encoded_len: 2,
        };
        shard.handle_action_completion(incoming, output)
    })
}

fn assert_stale_route_rejected_without_effects(
    shard: &mut Shard,
    journal: &crate::journal::VolatileRuntimeJournal,
    live: RunId,
    expected: RuntimeError,
    operation: impl FnOnce(&mut Shard) -> crate::RuntimeResult<()>,
) -> Result<(), String> {
    let state = shard.run_state_get(live).ok_or("missing before state")?;
    let frame = state.frame.clone();
    let store = state.store.clone();
    let attempts = state.action_attempts.clone();
    let run_attempt = state.run_attempt;
    let pending = shard.pending_action_clone();
    let counters = shard.counters().snapshot();
    let events = require_snapshot(journal)?;
    let sequences = shard.journal_sequences.clone();
    let trace = shard.trace_ring().snapshot_for_run(live, shard.trace_ring().capacity());
    let active = shard.active_run_count();
    assert_eq!(operation(shard), Err(expected));
    let after = shard.run_state_get(live).ok_or("missing after state")?;
    assert_eq!(after.frame, frame);
    assert_eq!(after.store, store);
    assert_eq!(after.action_attempts, attempts);
    assert_eq!(after.run_attempt, run_attempt);
    assert_eq!(shard.pending_action_clone(), pending);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(require_snapshot(journal)?, events);
    assert_eq!(shard.journal_sequences, sequences);
    assert_eq!(shard.trace_ring().snapshot_for_run(live, shard.trace_ring().capacity()), trace);
    assert_eq!(shard.active_run_count(), active);
    Ok(())
}

#[test]
fn previous_run_attempt_same_action_attempt_has_exact_stale_diagnostic() -> Result<(), String> {
    let (mut shard, journal, old, current) = stale_run_fixture()?;
    assert_eq!(current.run_attempt, 2);
    assert_eq!(current.attempt, old.attempt);
    assert_ne!(old.idempotency_key, current.idempotency_key);
    assert_completion_rejected_without_effects(
        &mut shard, &journal, current.run, old,
        RuntimeError::StaleAttempt { incoming: 1, current: 2 },
    )
}

#[test]
fn prior_run_diagnostic_does_not_replace_identity_or_future_rejections() -> Result<(), String> {
    for case in 0..11 {
        let (mut shard, journal, old, current) = stale_run_fixture()?;
        let mut incoming = old;
        let expected = match case {
            0 => { incoming.idempotency_key ^= 1; RuntimeError::InvalidActionCompletion }
            1 => { incoming.step = StepIdx::new(1); RuntimeError::InvalidActionCompletion }
            2 => {
                incoming.action = ActionId::new(1);
                incoming = rekey_run_ticket(incoming, 1);
                RuntimeError::InvalidActionCompletion
            }
            3 => {
                incoming.seq = SeqNo::new(1);
                incoming = rekey_run_ticket(incoming, 1);
                RuntimeError::InvalidActionCompletion
            }
            4 => { incoming.capacity = 2; RuntimeError::InvalidActionCompletion }
            5 => { incoming = rekey_run_ticket(incoming, 3); RuntimeError::InvalidActionCompletion }
            6 => { incoming = rekey_run_ticket(incoming, 0); RuntimeError::InvalidActionCompletion }
            7 => { incoming.attempt = 0; RuntimeError::InvalidActionCompletion }
            8 => { incoming.attempt = 2; RuntimeError::InvalidActionCompletion }
            9 => {
                incoming.run = RunId::new(61_202);
                incoming = rekey_run_ticket(incoming, 1);
                RuntimeError::RunNotFound
            }
            _ => {
                // A pending ticket inconsistent with live authority is not the
                // narrowly classified previous-run case.
                let state = shard.run_state_get_mut(current.run).ok_or("missing live state")?;
                state.run_attempt = core::num::NonZeroU16::new(3).ok_or("invalid attempt")?;
                RuntimeError::InvalidActionCompletion
            }
        };
        assert_completion_rejected_without_effects(
            &mut shard, &journal, current.run, incoming, expected,
        )?;
    }
    Ok(())
}

#[test]
fn current_invocation_fencing_and_old_action_attempt_controls_remain_distinct() -> Result<(), String> {
    let (mut shard, journal, _, current) = stale_run_fixture()?;
    let token = shard.fencing_tokens.mint(current.run);
    assert_ne!(token, current.fencing_token);
    assert_completion_rejected_without_effects(
        &mut shard, &journal, current.run, current,
        RuntimeError::StaleFencingToken { incoming: current.fencing_token },
    )?;
    let unknown_token = ActionTicket { fencing_token: 0, ..current };
    assert_eq!(shard.require_pending_action_ownership(unknown_token), Ok(()));
    let newest = ActionTicket { fencing_token: token, ..current };
    assert_eq!(shard.require_pending_action_ownership(newest), Ok(()));

    let (mut shard, journal, _, current) = stale_run_fixture()?;
    let pending = ActionTicket { attempt: 2, capacity: 2, ..current };
    let state = shard.run_state_get_mut(current.run).ok_or("missing live state")?;
    *state.action_attempts.first_mut().ok_or("missing action attempt")? = 2;
    assert_eq!(shard.pending_action_insert(current.run, pending), Ok(Some(current)));
    let previous_action = ActionTicket { attempt: 1, ..pending };
    assert_completion_rejected_without_effects(
        &mut shard, &journal, current.run, previous_action,
        RuntimeError::StaleAttempt { incoming: 1, current: 2 },
    )?;
    assert_eq!(shard.pending_action_remove(current.run), Some(pending));
    assert_completion_rejected_without_effects(
        &mut shard, &journal, current.run, pending, RuntimeError::InvalidActionCompletion,
    )
}

#[test]
fn previous_run_attempt_failure_rejects_before_effects() -> Result<(), String> {
    let (mut shard, journal, old, current) = stale_run_fixture()?;
    assert_stale_route_rejected_without_effects(
        &mut shard, &journal, current.run,
        RuntimeError::StaleAttempt { incoming: 1, current: 2 },
        |shard| shard.handle_action_failure(old, ActionFailure {
            code: ActionFailureCode::Rejected,
            retry_policy: VbRetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        }),
    )
}

#[test]
fn previous_run_attempt_registered_context_rejects_before_artifact_lookup() -> Result<(), String> {
    let (mut shard, journal, old, current) = stale_run_fixture()?;
    // This is the actual first gate of handle_registered_action_completion.
    // No fabricated receipt or claim of packed-executor success coverage.
    assert_stale_route_rejected_without_effects(
        &mut shard, &journal, current.run,
        RuntimeError::StaleAttempt { incoming: 1, current: 2 },
        |shard| shard.registered_action_context(old).map(|_| ()),
    )
}

#[test]
fn canonical_zero_and_future_run_attempts_remain_invalid() -> Result<(), String> {
    let (shard, _journal, _old, current) = stale_run_fixture()?;
    assert_eq!(current.run_attempt, 2);
    assert_eq!(shard.require_pending_action_ownership(current), Ok(()));

    for run_attempt in [0, 3, u16::MAX] {
        let incoming = rekey_run_ticket(current, run_attempt);
        // A canonical key is not proof of valid run-attempt authority.
        assert!(vb_core::action::action_ticket_has_valid_key(incoming));
        assert_eq!(incoming.run_attempt, run_attempt);
        assert_eq!(incoming.run, current.run);
        assert_eq!(incoming.step, current.step);
        assert_eq!(incoming.seq, current.seq);
        assert_eq!(incoming.action, current.action);
        assert_eq!(incoming.attempt, current.attempt);
        assert_eq!(incoming.capacity, current.capacity);
        assert_eq!(incoming.fencing_token, current.fencing_token);
        assert_eq!(
            shard.require_pending_action_ownership(incoming),
            Err(RuntimeError::InvalidActionCompletion),
        );
    }
    assert_eq!(shard.require_pending_action_ownership(current), Ok(()));
    Ok(())
}

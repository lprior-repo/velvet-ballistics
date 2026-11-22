// Bound-gate open submissions at the decision-deadline boundary.

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn assert_open_rejected_as_deadline_overflow(
    run: RunId,
    opening_ms: impl Fn(u64) -> Option<u64>,
) -> TestResult {
    let shard_count = NonZeroUsize::new(1).ok_or("one shard")?;
    let journal = GateDecisionTestJournal::shared();
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone())?;
    let workflow = gate_chain_workflow().ok_or("chain workflow must compile")?;
    assert_eq!(runtime.submit_compiled(run, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let policy_digest = WorkflowDigest::from_bytes([5; 32]);
    let shard = runtime.shards.first_mut().ok_or("single shard")?;
    let state = shard
        .run_state_get_mut(run)
        .ok_or("submitted run state")?;
    let admission = state.admission.take().ok_or("run admission")?;
    state.admission =
        Some(admission.with_policy_pin(vb_storage::RunPolicyPin::Pinned(policy_digest)));
    let gate = bound_gate_fixture(&runtime, run, 7);
    let opening = opening_ms(gate.gate.timeout_ms).ok_or("opening fits u64")?;
    let queued_before = runtime
        .shards
        .first()
        .ok_or("single shard")?
        .command_queue_len();
    assert_eq!(
        runtime.open_bound_gate(run, gate, opening, gate_person("bob")),
        Err(RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::DeadlineOverflow,
        })
    );
    assert!(
        journal
            .bound_opened
            .lock()
            .map_err(|_| "journal lock poisoned")?
            .is_empty()
    );
    let shard = runtime.shards.first().ok_or("single shard")?;
    assert_eq!(shard.command_queue_len(), queued_before);
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    assert!(shard.pending_action_get(run).is_none());
    assert_eq!(shard.open_gate_for(run), None);
    Ok(())
}

/// Running runtime with a pinned policy whose run lifetime ends at `u64::MAX`.
fn exact_fit_runtime(run: RunId) -> Result<(Runtime, Arc<GateDecisionTestJournal>), Box<dyn std::error::Error>> {
    let shard_count = NonZeroUsize::new(1).ok_or("one shard")?;
    let journal = GateDecisionTestJournal::shared();
    let mut runtime = Runtime::new(shard_count, runtime_config(), journal.clone())?;
    let workflow = gate_chain_workflow().ok_or("chain workflow must compile")?;
    assert_eq!(runtime.submit_compiled(run, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let policy_digest = WorkflowDigest::from_bytes([5; 32]);
    let shard = runtime.shards.first_mut().ok_or("single shard")?;
    let state = shard
        .run_state_get_mut(run)
        .ok_or("submitted run state")?;
    let admission = state.admission.take().ok_or("run admission")?;
    state.admission =
        Some(admission.with_policy_pin(vb_storage::RunPolicyPin::Pinned(policy_digest)));
    let limit_ms = state
        .admission
        .as_ref()
        .and_then(|admission| admission.budget())
        .map(|budget| budget.max_run_time_seconds)
        .and_then(|seconds| seconds.checked_mul(1_000))
        .ok_or("run lifetime budget")?;
    state.run_started_millis = Some(
        u64::MAX
            .checked_sub(limit_ms)
            .ok_or("lifetime fits u64")?,
    );
    Ok((runtime, journal))
}

#[test]
fn runtime_open_bound_gate_rejects_deadline_overflow_before_suspending() -> TestResult {
    assert_open_rejected_as_deadline_overflow(RunId::new(93106), |_| Some(u64::MAX))
}

#[test]
fn runtime_open_bound_gate_rejects_one_past_deadline_overflow_before_suspending() -> TestResult {
    assert_open_rejected_as_deadline_overflow(RunId::new(93108), |timeout_ms| {
        u64::MAX
            .checked_sub(timeout_ms)
            .and_then(|latest| latest.checked_add(1))
    })
}

#[test]
fn runtime_open_bound_gate_accepts_exact_deadline_fit() -> TestResult {
    let run = RunId::new(93107);
    let (mut runtime, _journal) = exact_fit_runtime(run)?;
    let gate = bound_gate_fixture(&runtime, run, 7);
    let timeout_ms = gate.gate.timeout_ms;
    // Pin the clock at the latest opening instant with a representable deadline.
    let exact_clock_ms = u64::MAX
        .checked_sub(timeout_ms)
        .ok_or("timeout fits u64")?;
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(exact_clock_ms);
    let queued_before = runtime
        .shards
        .first()
        .ok_or("single shard")?
        .command_queue_len();
    assert_eq!(
        runtime.open_bound_gate(run, gate, exact_clock_ms, gate_person("bob")),
        Ok(())
    );
    let shard = runtime.shards.first().ok_or("single shard")?;
    assert_eq!(
        shard.command_queue_len(),
        queued_before
            .checked_add(1)
            .ok_or("queue length fits usize")?
    );
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    assert_eq!(runtime.tick_all(), Ok(true));
    let shard = runtime.shards.first().ok_or("single shard")?;
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    assert!(shard.open_gate_for(run).is_some());
    Ok(())
}

#[test]
fn runtime_submit_bound_gate_decision_approves_at_exact_deadline() -> TestResult {
    let run = RunId::new(93109);
    let (mut runtime, journal) = exact_fit_runtime(run)?;
    let gate = bound_gate_fixture(&runtime, run, 7);
    let exact_clock_ms = u64::MAX
        .checked_sub(gate.gate.timeout_ms)
        .ok_or("timeout fits u64")?;
    {
        let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(exact_clock_ms);
        assert_eq!(
            runtime.open_bound_gate(run, gate, exact_clock_ms, gate_person("bob")),
            Ok(())
        );
        assert_eq!(runtime.tick_all(), Ok(true));
    }
    // The deadline itself is inclusive: a decision processed at u64::MAX is still live.
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(u64::MAX);
    let input = GateDecisionInput {
        decided_at_ms: u64::MAX,
        ..gate_input_fixture(7, GateDecision::Approve)
    };
    assert_eq!(runtime.submit_bound_gate_decision(run, input), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let shard = runtime.shards.first().ok_or("single shard")?;
    assert_eq!(shard.open_gate_for(run), None);
    let decisions = journal.bound_decisions()?;
    assert_eq!(decisions.len(), 1);
    let decision = decisions.first().ok_or("bound decision recorded")?;
    assert_eq!(decision.record.outcome, GateOutcome::Approved);
    Ok(())
}

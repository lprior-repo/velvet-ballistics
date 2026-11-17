#[test]
fn run_lifetime_with_missing_clock_fails_closed() -> Result<(), String> {
    let workflow = two_slot_workflow().ok_or_else(|| String::from("two-slot workflow"))?;
    let run = RunId::new(79_001);
    let digest = workflow.digest();
    let mut state = crate::shard::helpers::make_run_state(workflow, run)
        .ok_or_else(|| String::from("run state"))?;
    state.admission = Some(RunAdmission::with_budget(
        digest,
        run,
        CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
        run_time_budget(10),
    ));
    state.run_started_millis = Some(20_000);
    crate::shard::helpers::timer::install_wall_clock_unavailable_for_test();
    let mut evidence = EvidenceCollector::new();
    let result = Shard::drive_state_with_manual_gates(&mut state, 100, 0, &mut evidence);
    crate::shard::helpers::timer::restore_wall_clock_for_test();
    assert!(matches!(result,
        Err(RuntimeEngineError::Core(EngineError::InternalInvariantViolation { ref reason }))
            if reason.as_str() == "wall clock unavailable"
    ));
    Ok(())
}

// Synthetic zero-capacity refusal checks preservation, not allocator failure.
#[cfg(not(kani))]
#[test]
fn lifetime_terminal_preflight_preserves_missing_run_precedence() -> Result<(), String> {
    let mut shard = Shard::new(small_config()).map_err(|error| error.to_string())?;
    shard.max_active_runs = 0;
    assert_eq!(
        shard.apply_lifetime_expiry(RunId::new(79_031), 10),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(shard.last_lifetime_expiry(), None);
    Ok(())
}

#[cfg(not(kani))]
#[test]
fn lifetime_terminal_capacity_refusal_preserves_live_state() -> Result<(), String> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)
        .map_err(|error| error.to_string())?;
    let run = RunId::new(79_032);
    let workflow = require_workflow("lifetime-terminal-preflight", suspended_workflow())?;
    submit_run(&mut shard, run, workflow, &[(SlotIdx::ZERO, SlotValue::I64(0))]);
    let before = shard.run_state_get(run).ok_or("missing live state")?.clone();
    let pending = shard.pending_action_clone();
    let runtime = shard.runtime_state_get(run);
    let seq = shard.journal_seq_get(run);
    let quota = shard.charged_quotas.clone();
    let usage = shard.usage_snapshot();
    let global = shard.global_cost_usage();
    let counters = shard.counters().snapshot();
    let trace = shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity());
    let accepted = journal.snapshot().map_err(|error| error.to_string())?;
    shard.max_active_runs = 0;
    assert_eq!(
        shard.apply_lifetime_expiry(run, 10),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 0 })
    );
    assert_eq!(shard.run_state_get(run), Some(&before));
    assert_eq!(shard.pending_action_clone(), pending);
    assert_eq!(shard.runtime_state_get(run), runtime);
    assert_eq!(shard.journal_seq_get(run), seq);
    assert_eq!(shard.charged_quotas, quota);
    assert_eq!(shard.usage_snapshot(), usage);
    assert_eq!(shard.global_cost_usage(), global);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        shard.trace_ring().snapshot_for_run(run, shard.trace_ring().capacity()),
        trace
    );
    assert_eq!(journal.snapshot().map_err(|error| error.to_string())?, accepted);
    assert!(!shard.checked_out_run_contains(run));
    assert!(!shard.terminal_runs_contains(run));
    assert_eq!(shard.last_lifetime_expiry(), None);
    Ok(())
}

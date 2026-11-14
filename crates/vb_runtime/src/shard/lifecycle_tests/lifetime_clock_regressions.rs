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

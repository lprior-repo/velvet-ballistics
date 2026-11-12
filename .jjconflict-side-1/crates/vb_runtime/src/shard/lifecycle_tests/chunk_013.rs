#[test]
fn single_shard_owner() -> Result<(), String> {
    let mut owner = Shard::new(small_config());
    let mut other = Shard::new(small_config());
    let run = RunId::new(9_030);
    let workflow = require_workflow("suspended", suspended_workflow())?;
    submit_run(&mut owner, run, workflow);
    let state = owner
        .run_state_get(run)
        .cloned()
        .ok_or_else(|| "submitted run must remain owned".to_string())?;

    assert!(owner.run_state_contains(run));
    assert_eq!(owner.active_run_count(), 1);
    assert!(owner.pending_action_get(run).is_some());
    assert!(!other.run_state_contains(run));
    assert_eq!(other.active_run_count(), 0);
    assert_eq!(other.runtime_state_get(run), None);
    assert_eq!(other.pending_action_get(run), None);
    assert_eq!(other.pending_timer_get(run), None);
    let owners = [&owner, &other]
        .iter()
        .filter(|shard| shard.run_state_contains(run))
        .count();
    assert_eq!(owners, 1);

    assert_eq!(
        other.run_state_insert(run, state),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(
        other.pending_action_insert(run, make_ticket(run, StepIdx::ZERO, 1)),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(
        other.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
        Err(RuntimeError::RunNotFound)
    );

    assert!(owner.run_state_contains(run));
    assert_eq!(owner.active_run_count(), 1);
    assert!(!other.run_state_contains(run));
    assert_eq!(other.active_run_count(), 0);
    Ok(())
}

#[test]
fn single_shard_owner_checked_out_run_returns_only_to_owner() -> Result<(), String> {
    let mut owner = Shard::new(small_config());
    let mut other = Shard::new(small_config());
    let run = RunId::new(9_031);
    let workflow = require_workflow("suspended", suspended_workflow())?;
    submit_run(&mut owner, run, workflow);
    let state = owner
        .take_run_state(run)
        .map_err(|error| format!("take_run_state failed: {error:?}"))?;

    assert!(owner.checked_out_run_contains(run));
    assert!(!owner.run_state_contains(run));
    assert_eq!(owner.active_run_count(), 1);
    assert!(!other.checked_out_run_contains(run));

    assert_eq!(
        other.run_state_insert(run, state.clone()),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(owner.run_state_insert(run, state), Ok(None));

    assert!(owner.run_state_contains(run));
    assert!(!owner.checked_out_run_contains(run));
    assert_eq!(owner.active_run_count(), 1);
    assert!(!other.run_state_contains(run));
    assert_eq!(other.active_run_count(), 0);
    Ok(())
}

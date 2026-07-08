
    use super::super::types::{PendingTimer, RuntimeEvent, RuntimeState};

    fn guard_test_timer(kind: PendingTimerKind) -> PendingTimer {
        PendingTimer {
            step: StepIdx::ZERO,
            kind,
            generation: 1,
            deadline: std::time::Instant::now(),
        }
    }

    #[test]
    fn pending_boundary_maps_reject_never_admitted_run() {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_001);
        let timer = guard_test_timer(PendingTimerKind::Wait);
        let ticket = make_ticket(run, StepIdx::ZERO, 1);

        assert_eq!(shard.pending_timer_insert(run, timer), Err(RuntimeError::RunNotFound));
        assert_eq!(shard.pending_action_insert(run, ticket), Err(RuntimeError::RunNotFound));
        assert_eq!(shard.pending_timer_get(run), None);
        assert_eq!(shard.pending_action_get(run), None);
    }

    #[test]
    fn runtime_state_insert_rejects_untracked_run() {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_020);

        assert_eq!(
            shard.runtime_state_insert(run, RuntimeState::Running),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(shard.runtime_state_get(run), None);
        assert_eq!(shard.active_run_count(), 0);
    }

    #[test]
    fn terminal_membership_rejects_never_admitted_run() {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_026);

        assert_eq!(shard.terminal_runs_insert(run), Err(RuntimeError::RunNotFound));
        assert!(!shard.terminal_runs_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        assert_eq!(shard.active_run_count(), 0);
    }

    #[test]
    fn run_state_insert_rejects_untracked_active_owner() -> Result<(), String> {
        let mut source = Shard::new(small_config());
        let mut target = Shard::new(small_config());
        let run = RunId::new(9_021);
        let workflow = require_workflow("suspended", suspended_workflow())?;
        submit_run(&mut source, run, workflow);
        let state = source
            .run_state_get(run)
            .cloned()
            .ok_or_else(|| "submitted run must remain active".to_string())?;

        assert_eq!(
            target.run_state_insert(run, state),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(target.runtime_state_get(run), None);
        assert_eq!(target.active_run_count(), 0);
        assert!(source.run_state_contains(run));
        Ok(())
    }

    #[test]
    fn pending_boundary_maps_allow_checked_out_active_run() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_002);
        submit_run(&mut shard, run, workflow);
        let _removed_action = shard.pending_action_remove(run);
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;
        let timer = guard_test_timer(PendingTimerKind::Wait);
        let ticket = make_ticket(run, StepIdx::ZERO, 1);

        assert_eq!(shard.active_run_count(), 1);
        assert_eq!(shard.pending_timer_insert(run, timer), Ok(None));
        assert_eq!(shard.pending_action_insert(run, ticket), Ok(None));
        assert_eq!(shard.run_state_insert(run, state), Ok(None));
        Ok(())
    }

    #[test]
    fn terminal_transition_clears_active_and_pending_boundaries() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_003);
        submit_run(&mut shard, run, workflow);
        assert!(shard.pending_action_get(run).is_some());
        assert_eq!(
            shard.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
            Ok(None)
        );
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;

        assert_eq!(shard.finish_run_after_journaled(run, state), Ok(()));
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.pending_timer_get(run), None);
        assert_eq!(shard.pending_action_get(run), None);
        assert!(shard.terminal_runs_contains(run));
        Ok(())
    }

    #[test]
    fn failed_terminal_parity_clears_boundaries_and_runtime_state() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_016);
        submit_run(&mut shard, run, workflow);
        assert!(shard.pending_action_get(run).is_some());
        assert_eq!(
            shard.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
            Ok(None)
        );
        assert!(shard.runtime_state_get(run).is_some());
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;
        assert!(shard.checked_out_run_contains(run));

        shard
            .fail_run_state(run, state)
            .map_err(|error| format!("fail_run_state failed: {error:?}"))?;
        shard
            .apply(run, RuntimeEvent::Fail)
            .map_err(|error| format!("apply fail failed: {error:?}"))?;

        assert_eq!(shard.active_run_count(), 0);
        assert!(!shard.checked_out_run_contains(run));
        assert_eq!(shard.pending_timer_count(), 0);
        assert_eq!(shard.pending_timer_get(run), None);
        assert_eq!(shard.pending_action_len(), 0);
        assert_eq!(shard.pending_action_get(run), None);
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        Ok(())
    }

    #[test]
    fn kill_terminal_path_clears_boundaries_and_runtime_state() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_027);
        submit_run(&mut shard, run, workflow);
        assert!(shard.pending_action_get(run).is_some());
        assert_eq!(
            shard.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
            Ok(None)
        );

        assert_eq!(shard.enqueue(ShardCommand::Kill { run, reason: None }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));

        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.pending_timer_get(run), None);
        assert_eq!(shard.pending_action_get(run), None);
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        Ok(())
    }

    #[test]
    fn terminal_membership_clears_runtime_state_immediately() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_017);
        submit_run(&mut shard, run, workflow);
        assert!(shard.runtime_state_get(run).is_some());
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;

        assert_eq!(shard.terminal_runs_insert(run), Ok(true));
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        assert_eq!(
            shard.apply(run, RuntimeEvent::ResumeRollback),
            Err(RuntimeError::RunAlreadyExists)
        );
        assert_eq!(shard.runtime_state_get(run), None);
        shard.release_frame(state.frame);
        Ok(())
    }

    fn assert_checked_out_terminal_apply_clears_runtime_state(
        event: RuntimeEvent,
        run: RunId,
    ) -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        submit_run(&mut shard, run, workflow);
        assert_eq!(
            shard.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
            Ok(None)
        );
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;

        assert!(shard.checked_out_run_contains(run));
        assert!(shard.pending_action_get(run).is_some());
        assert!(shard.pending_timer_get(run).is_some());
        assert!(shard.runtime_state_get(run).is_some());
        shard
            .apply(run, event)
            .map_err(|error| format!("terminal apply failed: {error:?}"))?;

        assert_eq!(shard.runtime_state_get(run), None);
        assert!(shard.checked_out_run_contains(run));
        assert!(shard.pending_action_get(run).is_some());
        assert!(shard.pending_timer_get(run).is_some());
        shard.release_frame(state.frame);
        Ok(())
    }

    #[test]
    fn checked_out_fail_apply_clears_only_runtime_state() -> Result<(), String> {
        assert_checked_out_terminal_apply_clears_runtime_state(
            RuntimeEvent::Fail,
            RunId::new(9_022),
        )
    }

    #[test]
    fn checked_out_terminal_remove_apply_clears_only_runtime_state() -> Result<(), String> {
        assert_checked_out_terminal_apply_clears_runtime_state(
            RuntimeEvent::TerminalRemove,
            RunId::new(9_023),
        )
    }

    #[test]
    fn checked_out_drive_finished_apply_clears_only_runtime_state() -> Result<(), String> {
        assert_checked_out_terminal_apply_clears_runtime_state(
            RuntimeEvent::DriveFinished,
            RunId::new(9_024),
        )
    }

    #[test]
    fn checked_out_nonterminal_apply_preserves_runtime_state() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_025);
        submit_run(&mut shard, run, workflow);
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;

        shard
            .apply(run, RuntimeEvent::AwaitAction)
            .map_err(|error| format!("nonterminal apply failed: {error:?}"))?;

        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
        assert!(shard.checked_out_run_contains(run));
        shard.release_frame(state.frame);
        Ok(())
    }

    #[test]
    fn failed_runtime_state_insert_is_rejected_without_split_membership() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(9_018);
        submit_run(&mut shard, run, workflow);

        assert_eq!(
            shard.runtime_state_insert(run, RuntimeState::Failed),
            Err(RuntimeError::UnsupportedOperation {
                operation: "runtime_state_failed_terminal_split",
            })
        );
        assert_ne!(shard.runtime_state_get(run), Some(RuntimeState::Failed));
        assert!(!shard.terminal_runs_contains(run));
        Ok(())
    }

    #[test]
    fn terminal_failed_runtime_state_insert_is_rejected_and_cannot_coexist() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_019);
        submit_run(
            &mut shard,
            run,
            require_workflow("finished", finished_workflow())?,
        );

        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        assert_eq!(
            shard.runtime_state_insert(run, RuntimeState::Failed),
            Err(RuntimeError::UnsupportedOperation {
                operation: "runtime_state_failed_terminal_split",
            })
        );
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.runtime_state_get(run), None);
        Ok(())
    }

    #[test]
    fn terminal_retained_run_rejects_new_pending_boundaries() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let workflow = require_workflow("finished", finished_workflow())?;
        let run = RunId::new(9_006);
        submit_run(&mut shard, run, workflow);
        assert!(shard.terminal_runs_contains(run));
        assert_eq!(shard.active_run_count(), 0);

        assert_eq!(
            shard.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(
            shard.pending_action_insert(run, make_ticket(run, StepIdx::ZERO, 1)),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(shard.pending_timer_count(), 0);
        assert_eq!(shard.pending_action_clone().len(), 0);
        assert!(shard.terminal_runs_contains(run));
        Ok(())
    }

    #[test]
    fn pending_timer_aggregate_only_tracks_active_owner() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let terminal_run = RunId::new(9_011);
        let active_run = RunId::new(9_012);

        submit_run(
            &mut shard,
            terminal_run,
            require_workflow("finished", finished_workflow())?,
        );
        submit_run(
            &mut shard,
            active_run,
            require_workflow("suspended", suspended_workflow())?,
        );
        let active_timer = guard_test_timer(PendingTimerKind::Wait);
        let terminal_timer = guard_test_timer(PendingTimerKind::Ask);

        assert_eq!(shard.pending_timer_insert(active_run, active_timer), Ok(None));
        assert_eq!(
            shard.pending_timer_insert(terminal_run, terminal_timer),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.pending_timer_get(active_run), Some(active_timer));
        assert_eq!(shard.pending_timer_get(terminal_run), None);
        Ok(())
    }

    #[test]
    fn pending_action_aggregate_only_tracks_active_owner() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let terminal_run = RunId::new(9_009);
        let active_run = RunId::new(9_010);

        submit_run(
            &mut shard,
            terminal_run,
            require_workflow("finished", finished_workflow())?,
        );
        submit_run(
            &mut shard,
            active_run,
            require_workflow("suspended", suspended_workflow())?,
        );
        assert!(shard.terminal_runs_contains(terminal_run));
        assert_eq!(shard.pending_action_len(), 1);
        assert!(shard.pending_action_get(active_run).is_some());

        assert_eq!(
            shard.pending_action_insert(
                terminal_run,
                make_ticket(terminal_run, StepIdx::ZERO, 1),
            ),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(shard.pending_action_len(), 1);
        assert!(shard.pending_action_get(active_run).is_some());
        assert_eq!(shard.pending_action_get(terminal_run), None);
        Ok(())
    }

    #[test]
    fn terminal_membership_rejects_live_run_state_owner() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_014);
        submit_run(
            &mut shard,
            run,
            require_workflow("suspended", suspended_workflow())?,
        );

        assert_eq!(shard.active_run_count(), 1);
        assert!(shard.pending_action_get(run).is_some());
        assert_eq!(
            shard.terminal_runs_insert(run),
            Err(RuntimeError::RunAlreadyExists)
        );
        assert!(!shard.terminal_runs_contains(run));
        assert_eq!(shard.active_run_count(), 1);
        assert!(shard.pending_action_get(run).is_some());
        Ok(())
    }

    #[test]
    fn terminal_membership_clears_checked_out_and_pending_boundaries() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_015);
        submit_run(
            &mut shard,
            run,
            require_workflow("suspended", suspended_workflow())?,
        );
        assert_eq!(
            shard.pending_timer_insert(run, guard_test_timer(PendingTimerKind::Wait)),
            Ok(None)
        );
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;

        assert!(shard.checked_out_run_contains(run));
        assert_eq!(shard.active_run_count(), 1);
        assert_eq!(shard.terminal_runs_insert(run), Ok(true));
        assert!(shard.terminal_runs_contains(run));
        assert!(!shard.checked_out_run_contains(run));
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.pending_timer_get(run), None);
        assert_eq!(shard.pending_action_get(run), None);
        shard.release_frame(state.frame);
        Ok(())
    }

    #[test]
    fn pending_boundary_snapshot_counts_checked_out_active_owner() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let run = RunId::new(9_013);
        submit_run(
            &mut shard,
            run,
            require_workflow("suspended", suspended_workflow())?,
        );
        let state = shard
            .take_run_state(run)
            .map_err(|error| format!("take_run_state failed: {error:?}"))?;

        let snapshot = shard.pending_boundary_snapshot(7, 8);
        assert_eq!(snapshot.active_run_count(), shard.active_run_count());
        assert_eq!(snapshot.active_run_count(), 1);
        assert_eq!(snapshot.active_runs(), &[run]);
        assert_eq!(shard.run_state_insert(run, state), Ok(None));
        Ok(())
    }

    #[test]
    fn terminal_retention_does_not_consume_active_capacity() -> Result<(), String> {
        let mut config = small_config();
        config.max_active_runs = 1;
        let mut shard = Shard::new(config);
        let terminal_run = RunId::new(9_004);
        let active_run = RunId::new(9_005);
        let terminal_workflow = require_workflow("finished", finished_workflow())?;
        let active_workflow = require_workflow("suspended", suspended_workflow())?;

        submit_run(&mut shard, terminal_run, terminal_workflow);
        assert!(shard.terminal_runs_contains(terminal_run));
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: active_run,
                workflow: active_workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        assert!(shard.terminal_runs_contains(terminal_run));
        assert!(!shard.terminal_runs_contains(active_run));
        Ok(())
    }

    #[test]
    fn terminal_retention_evicts_oldest_terminal_identity_at_capacity() -> Result<(), String> {
        let mut config = small_config();
        config.max_active_runs = 1;
        let mut shard = Shard::new(config);
        let oldest_terminal = RunId::new(9_007);
        let retained_terminal = RunId::new(9_008);

        submit_run(
            &mut shard,
            oldest_terminal,
            require_workflow("oldest", finished_workflow())?,
        );
        submit_run(
            &mut shard,
            retained_terminal,
            require_workflow("retained", finished_workflow())?,
        );

        assert!(!shard.terminal_runs_contains(oldest_terminal));
        assert!(shard.terminal_runs_contains(retained_terminal));
        assert_eq!(shard.active_run_count(), 0);
        Ok(())
    }

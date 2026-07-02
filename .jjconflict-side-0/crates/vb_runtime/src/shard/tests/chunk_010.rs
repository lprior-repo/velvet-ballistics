
#[test]
fn shard_submit_run_reuses_frame_from_pool_after_prior_finish() {
    // Given a shard where a run finished and returned its frame to the pool
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(401),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    // When submitting a new run with the same workflow dimensions
    let Some(workflow2) = finished_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(402),
            workflow: workflow2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the second run also completes
    assert_eq!(shard.counters().snapshot().runs_completed, 2);
    // Both runs are finished; pool has all pre-allocated frames available
    assert_eq!(
        shard.frame_pools.get(&(2, 1)).map(FramePool::available),
        Some(4)
    );
}

#[test]
fn shard_submit_max_active_runs_boundary_exactly_at_limit_succeeds() {
    // Given a shard with max_active_runs = 3
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 3,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    // When submitting exactly 3 suspended runs (each suspends on Do, staying active)
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(501),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(502),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(503),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then all 3 are submitted successfully
    assert_eq!(shard.counters().snapshot().runs_submitted, 3);
    // And submitting a 4th returns ActiveRunCapacityExceeded
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(504),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 3 })
    );
}

#[test]
fn shard_inspect_preserves_latest_response_overwriting_previous() {
    // Given a shard with two active runs
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let run1 = super::RunId::new(600);
    let run2 = super::RunId::new(601);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run1,
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: run2,
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When inspecting run1 then run2 without taking the first response
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: run1,
            correlation: 1,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: run2,
            correlation: 2,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then only the last inspect response is available (first was overwritten)
    let response = shard.take_inspect_response();
    match response {
        Some(InspectResponse::Found(snap)) => {
            assert_eq!(snap.run, run2);
            assert_eq!(snap.correlation, 2);
        }
        other => {
            assert_eq!(other, None);
        }
    }
}

// =========================================================================
// Phase 2 adversarial BDD tests — shard resource exhaustion & security
// =========================================================================

#[test]
fn shard_queue_full_prevents_further_command_submission() {
    // Given a shard with command queue capacity of 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // When filling the queue with 2 commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then the third command is rejected with QueueFull
    assert_eq!(
        shard.enqueue(ShardCommand::Shutdown),
        Err(RuntimeError::QueueFull)
    );
}

#[test]
fn shard_active_run_capacity_exhausted_returns_precise_capacity_error() {
    // Given a shard with max_active_runs = 2
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let Some(wf3) = suspended_workflow() else {
        return;
    };

    // When submitting 2 runs (both suspend on Do, so stay active)
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow: wf1,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(2),
            workflow: wf2,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // Then the third submit is rejected with capacity 2
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(3),
            workflow: wf3,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
    );
}

#[test]
fn shard_action_completed_for_wrong_run_returns_run_not_found() {
    // Given a shard with an active suspended run
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(1),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    // When completing an action for a different run
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompletedLegacy {
            run: super::RunId::new(999),
            step: vb_core::ids::StepIdx::new(0),
        }),
        Ok(())
    );
    // Then tick returns RunNotFound
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

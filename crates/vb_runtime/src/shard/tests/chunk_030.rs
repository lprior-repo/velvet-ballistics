
// =========================================================================
// Section 1: Shard Creation and Ownership
// =========================================================================

#[test]
fn shard_created_with_default_config_has_expected_properties() {
    let config = ShardConfig::default();
    let shard = Shard::new(config);
    kani::assert(shard.command_queue_capacity(, "assertion failed") == 1024, "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.pending_timer_count(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.is_shutting_down(, "assertion failed") == false, "assertion failed");
    let status = shard.status();
     == false, "assertion failed");
    let status = shard.status();
    kani::assert(status.health == super::ShardHealth::Running, "assertion failed");
    kani::assert(status.running == true, "assertion failed");
    kani::assert(status.shutting_down == false, "assertion failed");
    kani::assert(status.command_queue_capacity == 1024, "assertion failed");
    kani::assert(status.max_active_runs == 1024, "assertion failed");
    kani::assert(status.step_budget_per_tick == 1000, "assertion failed");
}

#[test]
fn shard_created_with_custom_config_has_custom_properties() {
    let config = ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 128,
        step_budget_per_tick: 500,
        max_active_runs: 16,
        policy: vb_core::policy::RuntimePolicy::Strict,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    kani::assert(shard.command_queue_capacity() == 64, "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 0, "assertion failed");
    let status = shard.status();
     == 0, "assertion failed");
    let status = shard.status();
    kani::assert(status.command_queue_capacity == 64, "assertion failed");
    kani::assert(status.max_active_runs == 16, "assertion failed");
    kani::assert(status.step_budget_per_tick == 500, "assertion failed");
    kani::assert(status.trace_capacity == 128, "assertion failed");
    kani::assert(status.runtime_policy == vb_core::policy::RuntimePolicy::Strict, "assertion failed");
}

#[test]
fn shard_config_new_rejects_zero_queue_capacity_via_validated_constructor() {
    let result = ShardConfig::new(
        0,
        4096,
        1000,
        1024,
        vb_core::policy::RuntimePolicy::Strict,
    );
    kani::assert(result == Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: 0,
            max: super::MAX_COMMAND_QUEUE_CAPACITY,
        }), "assertion failed");
}

#[test]
fn shard_config_new_rejects_exceeding_max_command_queue_capacity() {
    let too_large = super::MAX_COMMAND_QUEUE_CAPACITY.saturating_add(1);
    let result = ShardConfig::new(
        too_large,
        4096,
        1000,
        1024,
        vb_core::policy::RuntimePolicy::Strict,
    );
    kani::assert(result == Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: too_large,
            max: super::MAX_COMMAND_QUEUE_CAPACITY,
        }, "assertion failed"), "assertion failed");
}

#[test]
fn shard_config_new_rejects_zero_active_runs_via_validated_constructor() {
    let result = ShardConfig::new(
        1024,
        4096,
        1000,
        0,
        vb_core::policy::RuntimePolicy::Strict,
    );
    kani::assert(result == Err(RuntimeError::ActiveRunCapacityZero, "assertion failed"), "assertion failed");
}

#[test]
fn shard_config_new_accepts_minimal_valid_config() {
    let result = ShardConfig::new(
        1,
        1,
        1,
        1,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    kani::assert(result.is_ok(, "assertion failed") == true, "assertion failed");
    let config = match result {
        Ok(c) => c,
        Err(_) => return,
    };
     == true, "assertion failed");
    let config = match result {
        Ok(c) => c,
        Err(_) => return,
    };
    kani::assert(config.command_queue_capacity == 1, "assertion failed");
    kani::assert(config.max_active_runs == 1, "assertion failed");
}

#[test]
fn shard_config_new_accepts_maximum_capacity_limit() {
    let result = ShardConfig::new(
        super::MAX_COMMAND_QUEUE_CAPACITY,
        4096,
        1000,
        1,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    kani::assert(result.is_ok() == true, "assertion failed");
}

// =========================================================================
// Section 2: Shard State Machine
// =========================================================================

#[test]
fn shard_status_reports_running_health_for_fresh_shard() {
    let config = small_config();
    let shard = Shard::new(config);
    let status = shard.status();
     == true, "assertion failed");
}

// =========================================================================
// Section 2: Shard State Machine
// =========================================================================

#[test]
fn shard_status_reports_running_health_for_fresh_shard() {
    let config = small_config();
    let shard = Shard::new(config);
    let status = shard.status();
    kani::assert(status.health == super::ShardHealth::Running, "assertion failed");
    kani::assert(status.running == true, "assertion failed");
    kani::assert(status.shutting_down == false, "assertion failed");
    kani::assert(status.active_runs == 0, "assertion failed");
    kani::assert(status.command_queue_depth == 0, "assertion failed");
}

#[test]
fn shard_status_reports_shutting_down_after_shutdown_command() {
    let config = small_config();
    let mut shard = Shard::new(config);
    kani::assert(shard.enqueue(ShardCommand::Shutdown) == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(false), "assertion failed");
    let status = shard.status();
     == Ok(false), "assertion failed");
    let status = shard.status();
    kani::assert(status.health == super::ShardHealth::ShuttingDown, "assertion failed");
    kani::assert(status.running == false, "assertion failed");
    kani::assert(status.shutting_down == true, "assertion failed");
}

#[test]
fn shard_status_active_runs_increments_after_submitting_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(9001);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }) == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
    let status = shard.status();
     == 1, "assertion failed");
    let status = shard.status();
    kani::assert(status.active_runs == 1, "assertion failed");
}

#[test]
fn shard_is_shutting_down_returns_false_before_shutdown_command() {
    let config = small_config();
    let mut shard = Shard::new(config);
    kani::assert(shard.is_shutting_down() == false, "assertion failed");
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(9002),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.is_shutting_down(, "assertion failed") == false, "assertion failed");
}

#[test]
fn shard_tick_returns_false_when_shutting_down_and_queue_empty() {
    let config = small_config();
    let mut shard = Shard::new(config);
    kani::assert(shard.enqueue(ShardCommand::Shutdown, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(false), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(false), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(false), "assertion failed");
    kani::assert(shard.is_shutting_down(, "assertion failed") == true, "assertion failed");
}

// =========================================================================
// Section 3: Shard Migration Between Owners (Drain / Queue Transfer)
// =========================================================================

#[test]
fn shard_drain_for_shutdown_clears_queue_and_shuts_down() {
    let config = small_config();
    let mut shard = Shard::new(config);
    kani::assert(shard.enqueue(ShardCommand::Shutdown, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.drain_for_shutdown(, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.is_shutting_down(, "assertion failed") == true, "assertion failed");
    kani::assert(shard.command_queue_len(, "assertion failed") == 0, "assertion failed");
}

#[test]
fn shard_drain_pending_and_shutdown_clears_timers_and_shuts_down() {
    let config = small_config();
    let mut shard = Shard::new(config);
    kani::assert(shard.drain_pending_and_shutdown(, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.is_shutting_down(, "assertion failed") == true, "assertion failed");
    kani::assert(shard.pending_timer_count(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.command_queue_len(, "assertion failed") == 0, "assertion failed");
}

#[test]
fn shard_remaining_capacity_decreases_after_each_enqueue() {
    let config = ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    let initial = shard.remaining_capacity();
     == 0, "assertion failed");
}

#[test]
fn shard_remaining_capacity_decreases_after_each_enqueue() {
    let config = ShardConfig {
        command_queue_capacity: 8,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    let initial = shard.remaining_capacity();
    kani::assert(initial == 8, "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(9999),
            reason: None,
        }) == Ok(()), "assertion failed");
    kani::assert(shard.remaining_capacity(, "assertion failed") == 7, "assertion failed");
    kani::assert(shard.command_queue_len(, "assertion failed") == 1, "assertion failed");
}

#[test]
fn shard_can_process_commands_in_fifo_order() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run_a = super::RunId::new(5001);
    let run_b = super::RunId::new(5002);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_a,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_b,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 2, "assertion failed");
}

// =========================================================================
// Section 4: Shard Resource Allocation and Deallocation
// =========================================================================

#[test]
fn shard_command_queue_initial_capacity_matches_config() {
    let config = ShardConfig {
        command_queue_capacity: 256,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    kani::assert(shard.command_queue_capacity(, "assertion failed") == 256, "assertion failed");
    kani::assert(shard.remaining_capacity(, "assertion failed") == 256, "assertion failed");
    kani::assert(shard.command_queue_len(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.is_queue_full(, "assertion failed") == false, "assertion failed");
}

#[test]
fn shard_frame_pool_metrics_new_shard_has_no_pools() {
    let config = small_config();
    let shard = Shard::new(config);
    let (free, total) = shard.frame_pool_metrics();
     == false, "assertion failed");
}

#[test]
fn shard_frame_pool_metrics_new_shard_has_no_pools() {
    let config = small_config();
    let shard = Shard::new(config);
    let (free, total) = shard.frame_pool_metrics();
    kani::assert(free == 0, "assertion failed");
    kani::assert(total == 0, "assertion failed");
}

#[test]
fn shard_trace_ring_capacity_matches_config() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 32,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    let status = shard.status();
    kani::assert(status.trace_capacity == 32, "assertion failed");
}

#[test]
fn shard_pending_timer_count_zero_for_new_shard() {
    let config = small_config();
    let shard = Shard::new(config);
    kani::assert(shard.pending_timer_count() == 0, "assertion failed");
}

#[test]
fn shard_snapshot_run_returns_not_found_for_unknown_run() {
    let config = small_config();
    let shard = Shard::new(config);
    let unknown_run = super::RunId::new(8888);
    let response = shard.snapshot_run(unknown_run, 42);
    match response {
        InspectResponse::NotFound { run, correlation } => {
             == 0, "assertion failed");
}

#[test]
fn shard_snapshot_run_returns_not_found_for_unknown_run() {
    let config = small_config();
    let shard = Shard::new(config);
    let unknown_run = super::RunId::new(8888);
    let response = shard.snapshot_run(unknown_run, 42);
    match response {
        InspectResponse::NotFound { run, correlation } => {
            kani::assert(run == unknown_run, "assertion failed");
            kani::assert(correlation == 42, "assertion failed");
        }
        InspectResponse::Found(_) => panic!("expected NotFound for unknown run"),
    }
}

#[test]
fn shard_snapshot_run_returns_found_for_active_run() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(6001);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }) == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    let response = shard.snapshot_run(run, 99);
    match response {
        InspectResponse::Found(snapshot) => {
             == Ok(true), "assertion failed");
    let response = shard.snapshot_run(run, 99);
    match response {
        InspectResponse::Found(snapshot) => {
            kani::assert(snapshot.run == run, "assertion failed");
            kani::assert(snapshot.correlation == 99, "assertion failed");
        }
        InspectResponse::NotFound { .. } => panic!("expected Found for active run"),
    }
}

// =========================================================================
// Section 5: Shard Capacity Enforcement
// =========================================================================

#[test]
fn shard_enqueue_rejects_when_queue_is_full() {
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    let cancel_cmd = ShardCommand::Cancel {
        run: super::RunId::new(7001),
        reason: None,
    };
    kani::assert(shard.enqueue(cancel_cmd.clone()) == Ok(()), "assertion failed");
    kani::assert(shard.enqueue(cancel_cmd.clone(), "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.enqueue(cancel_cmd, "assertion failed") == Err(RuntimeError::QueueFull), "assertion failed");
    kani::assert(shard.is_queue_full(, "assertion failed") == true, "assertion failed");
}

#[test]
fn shard_is_queue_full_returns_false_when_below_capacity() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    kani::assert(shard.is_queue_full(, "assertion failed") == false, "assertion failed");
    let cancel_cmd = ShardCommand::Cancel {
        run: super::RunId::new(7002),
        reason: None,
    };
    kani::assert(shard.enqueue(cancel_cmd, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.is_queue_full(, "assertion failed") == false, "assertion failed");
}

#[test]
fn shard_max_active_runs_rejects_excess_submissions() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 16,
        max_active_runs: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run_a = super::RunId::new(7003);
    let run_b = super::RunId::new(7004);
    let run_c = super::RunId::new(7005);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_a,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_b,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 2, "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_c,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 }), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 2, "assertion failed");
}

#[test]
fn shard_command_queue_capacity_match_is_exact_after_construction() {
    let config = ShardConfig {
        command_queue_capacity: 512,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    let status = shard.status();
     == 2, "assertion failed");
}

#[test]
fn shard_command_queue_capacity_match_is_exact_after_construction() {
    let config = ShardConfig {
        command_queue_capacity: 512,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    let status = shard.status();
    kani::assert(status.command_queue_capacity == 512, "assertion failed");
    kani::assert(status.command_queue_depth == 0, "assertion failed");
    kani::assert(shard.remaining_capacity() == 512, "assertion failed");
}

#[test]
fn shard_queue_is_empty_and_full_are_mutually_consistent() {
    let config = ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    };
    let shard = Shard::new(config);
    kani::assert(shard.command_queue_len(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.remaining_capacity(, "assertion failed") == 1, "assertion failed");
    kani::assert(shard.is_queue_full(, "assertion failed") == false, "assertion failed");
    let cancel_cmd = ShardCommand::Cancel {
        run: super::RunId::new(7006),
        reason: None,
    };
    kani::assert(shard.enqueue(cancel_cmd, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.command_queue_len(, "assertion failed") == 1, "assertion failed");
    kani::assert(shard.remaining_capacity(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.is_queue_full(, "assertion failed") == true, "assertion failed");
}

// =========================================================================
// Section 6: Concurrent Shard Access Patterns
// =========================================================================

#[test]
fn shard_multiple_runs_different_states_preserve_isolation() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run_a = super::RunId::new(8001);
    let run_b = super::RunId::new(8002);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_a,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_b,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 2, "assertion failed");
    let Some(_) = shard.timer_entry(run_a) else {
        panic!("run_a should have a timer entry");
    };
    let Some(_) = shard.timer_entry(run_b) else {
        panic!("run_b should have a timer entry");
    };
    kani::assert(shard.enqueue(ShardCommand::Cancel {
            run: run_a,
            reason: None,
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
    kani::assert(shard.timer_entry(run_b, "assertion failed").is_some() == true, "assertion failed");
}

#[test]
fn shard_sequential_ticks_process_all_queued_commands() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow_a) = suspended_workflow() else {
        return;
    };
    let Some(workflow_b) = suspended_workflow() else {
        return;
    };
    let run_a = super::RunId::new(8003);
    let run_b = super::RunId::new(8004);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_a,
            workflow: workflow_a,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: run_b,
            workflow: workflow_b,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 2, "assertion failed");
    kani::assert(shard.command_queue_len(, "assertion failed") == 0, "assertion failed");
}

#[test]
fn shard_cancel_command_removes_run_and_frees_resources() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(8005);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Cancel {
            run,
            reason: Some(String::from("test cancellation")),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.pending_timer_count(, "assertion failed") == 0, "assertion failed");
}

// =========================================================================
// Section 7: Shard Recovery After Failure
// =========================================================================

#[test]
fn shard_remains_operational_after_tick_returns_error_for_bad_command() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let invalid_ticket = vb_core::action::ActionTicket {
        run: super::RunId::new(9999),
        step: vb_core::ids::StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: vb_core::ids::ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
            ..Default::default()
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: vb_core::ids::SlotIdx::ZERO,
        value: vb_core::value::SlotValue::I64(42),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 0,
    };
    kani::assert(shard.enqueue(ShardCommand::ActionCompleted { ticket: invalid_ticket, output }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Err(RuntimeError::RunNotFound), "assertion failed");
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let recovery_run = super::RunId::new(9006);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: recovery_run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
}

#[test]
fn shard_after_cancel_run_can_accept_new_run_with_same_id() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(9007);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Cancel {
            run,
            reason: None,
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 0, "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
}

#[test]
fn shard_enqueue_rejects_submit_after_shutdown() {
    let config = small_config();
    let mut shard = Shard::new(config);
    kani::assert(shard.enqueue(ShardCommand::Shutdown, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(false), "assertion failed");
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run: super::RunId::new(9008),
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(false), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 0, "assertion failed");
}

#[test]
fn shard_submit_rejects_duplicate_run_id_with_exact_error() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = super::RunId::new(9009);
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Ok(true), "assertion failed");
    kani::assert(shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }, "assertion failed") == Ok(()), "assertion failed");
    kani::assert(shard.tick(, "assertion failed") == Err(RuntimeError::RunAlreadyExists), "assertion failed");
    kani::assert(shard.active_run_count(, "assertion failed") == 1, "assertion failed");
}

#[test]
fn shard_timer_entry_returns_none_for_unknown_run() {
    let config = small_config();
    let shard = Shard::new(config);
    let entry = shard.timer_entry(super::RunId::new(9010));
     == 1, "assertion failed");
}

#[test]
fn shard_timer_entry_returns_none_for_unknown_run() {
    let config = small_config();
    let shard = Shard::new(config);
    let entry = shard.timer_entry(super::RunId::new(9010));
    kani::assert(entry == None, "assertion failed");
}

// =========================================================================
// Section 8: Kani — Shard capacity never exceeded, state transitions valid
// =========================================================================

#[cfg(kani)]
mod kani_proofs {
    use super::super::*;

    #[kani::proof]
    fn verify_command_queue_capacity_never_exceeded() {
        let capacity: usize = kani::any();
        kani::assume(capacity > 0);
        kani::assume(capacity <= super::MAX_COMMAND_QUEUE_CAPACITY);
        let config = ShardConfig {
            command_queue_capacity: capacity,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        };
        let shard = Shard::new(config);
        kani::assert(shard.command_queue_capacity() == capacity, "assertion failed");
        kani::assert(shard.command_queue_len(, "assertion failed") == 0, "assertion failed");
        kani::assert(shard.remaining_capacity(, "assertion failed") <= capacity, "kani harness assertion");
        kani::assert(shard.remaining_capacity(, "assertion failed") == capacity, "assertion failed");
    }

    #[kani::proof]
    fn verify_queue_remaining_capacity_never_overflows() {
        let capacity: usize = kani::any();
        kani::assume(capacity > 0);
        kani::assume(capacity <= super::MAX_COMMAND_QUEUE_CAPACITY);
        let shard = Shard::new(ShardConfig {
            command_queue_capacity: capacity,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        });
        let rem = shard.remaining_capacity();
         == capacity, "assertion failed");
    }

    #[kani::proof]
    fn verify_queue_remaining_capacity_never_overflows() {
        let capacity: usize = kani::any();
        kani::assume(capacity > 0);
        kani::assume(capacity <= super::MAX_COMMAND_QUEUE_CAPACITY);
        let shard = Shard::new(ShardConfig {
            command_queue_capacity: capacity,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        });
        let rem = shard.remaining_capacity();
        kani::assert(rem <= capacity, "kani harness assertion");
    }

    #[kani::proof]
    fn verify_is_queue_full_consistent_with_length() {
        let capacity: usize = kani::any();
        kani::assume(capacity > 0);
        kani::assume(capacity <= super::MAX_COMMAND_QUEUE_CAPACITY);
        let shard = Shard::new(ShardConfig {
            command_queue_capacity: capacity,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        });
        let len = shard.command_queue_len();
        let is_full = shard.is_queue_full();
        kani::assert(is_full == len == capacity, "assertion failed");
    }

    #[kani::proof]
    fn verify_runtime_state_transitions_produce_valid_states() {
        let state: u8 = kani::any();
        kani::assume(state < 5);
        let runtime_state = match state {
            0 => super::RuntimeState::Initial,
            1 => super::RuntimeState::Running,
            2 => super::RuntimeState::Resumable,
            3 => super::RuntimeState::Resuming,
            4 => super::RuntimeState::Failed,
            _ => { kani::assume(false); return; },
        };
        match runtime_state {
            super::RuntimeState::Initial => {
                kani::assert(!runtime_state.is_resumable(), "kani harness assertion");
            }
            super::RuntimeState::Running => {
                kani::assert(!runtime_state.is_resumable(, "assertion failed"), "kani harness assertion");
            }
            super::RuntimeState::Resumable => {
                kani::assert(runtime_state.is_resumable(, "assertion failed"), "kani harness assertion");
            }
            super::RuntimeState::Resuming => {
                kani::assert(!runtime_state.is_resumable(, "assertion failed"), "kani harness assertion");
            }
            super::RuntimeState::Failed => {
                kani::assert(!runtime_state.is_resumable(, "assertion failed"), "kani harness assertion");
            }
        }
    }

    #[kani::proof]
    fn verify_runtime_event_terminal_state_produces_correct_classification() {
        let event_idx: u8 = kani::any();
        kani::assume(event_idx < 10);
        let event = match event_idx {
            0 => super::RuntimeEvent::Submit,
            1 => super::RuntimeEvent::Resume,
            2 => super::RuntimeEvent::ResumeRollback,
            3 => super::RuntimeEvent::DriveContinue,
            4 => super::RuntimeEvent::DriveFinished,
            5 => super::RuntimeEvent::AwaitAction,
            6 => super::RuntimeEvent::AwaitTimer,
            7 => super::RuntimeEvent::Fail,
            8 => super::RuntimeEvent::TerminalRemove,
            _ => super::RuntimeEvent::Submit,
        };
        let terminal = event.is_terminal();
        let resumable = event.is_resumable();
        kani::assert(terminal != resumable || (!terminal && !resumable, "assertion failed"), "event cannot be both terminal and resumable: {event:?}");
        if terminal {
            kani::assert(matches!(
                event, super::RuntimeEvent::Fail
                    | super::RuntimeEvent::TerminalRemove
                    | super::RuntimeEvent::DriveFinished, "assertion failed"));
        }
    }

    #[kani::proof]
    fn verify_is_valid_command_queue_capacity_within_domain() {
        let cap: usize = kani::any();
        let valid = super::is_valid_command_queue_capacity(cap);
        if cap == 0 {
            );
        }
    }

    #[kani::proof]
    fn verify_is_valid_command_queue_capacity_within_domain() {
        let cap: usize = kani::any();
        let valid = super::is_valid_command_queue_capacity(cap);
        if cap == 0 {
            kani::assert(!valid, "kani harness assertion");
        } else if cap > super::MAX_COMMAND_QUEUE_CAPACITY {
            kani::assert(!valid, "kani harness assertion");
        } else {
            kani::assert(valid, "kani harness assertion");
        }
    }

    #[kani::proof]
    fn verify_shard_config_new_zero_capacity_always_rejected() {
        let policy: u8 = kani::any();
        kani::assume(policy < 5);
        let pol = match policy {
            0 => vb_core::policy::RuntimePolicy::Strict,
            1 => vb_core::policy::RuntimePolicy::Relaxed,
            2 => vb_core::policy::RuntimePolicy::Journaled,
            _ => vb_core::policy::RuntimePolicy::Strict,
        };
        let result = ShardConfig::new(0, 4096, 1000, 1, pol);
        kani::assert(matches!(
                result, Err(RuntimeError::CommandQueueCapacityExceeded { .. })
            ));
    }

    #[kani::proof]
    fn verify_pending_timer_matches_authority_with_matching_fields() {
        let generation: u64 = kani::any();
        let deadline_ms: u64 = kani::any();
        kani::assume(deadline_ms < u64::MAX / 2);
        let kind: u8 = kani::any();
        kani::assume(kind < 2);
        let kind = match kind {
            0 => super::PendingTimerKind::Wait,
            _ => super::PendingTimerKind::Ask,
        };
        let kani_deadline = std::time::Instant::now();
        let timer = super::PendingTimer {
            step: vb_core::ids::StepIdx::new(0),
            kind,
            generation,
            deadline: kani_deadline,
        };
        kani::assert(timer.matches_authority(generation, kani_deadline, kind, "assertion failed"));
        if generation > 0 {
            kani::assert(!timer.matches_authority(generation.wrapping_sub(1), kani_deadline, kind, "assertion failed"));
        }
    }
}


/// Submit a finished workflow, then inspect it -- counters correct.
#[test]
fn shard_submit_finish_then_inspect_counters() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = super::RunId::new(1030);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: wf,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_completed, 1);

    // Inspect the finished run
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run,
            correlation: 5,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run,
            correlation: 5
        })
    );
}

// =======================================================================
// Edge-case tests for ShardConfig, PendingTimerKind, PendingTimer,
// AskTicket, AskAnswer, InspectSnapshot, and InspectResponse
// =======================================================================

#[test]
fn shard_config_default_uses_strict_policy() {
    let config = ShardConfig::default();
    assert_eq!(config.policy, vb_core::policy::RuntimePolicy::Strict);
}

#[test]
fn shard_config_copy_preserves_independent_snapshot() {
    let original = ShardConfig::default();
    let copy = original;
    // Mutating a derived config must not affect the original;
    // since ShardConfig is Copy, both are independent values.
    assert_eq!(copy.command_queue_capacity, original.command_queue_capacity);
    assert_eq!(copy.trace_capacity, original.trace_capacity);
    assert_eq!(copy.step_budget_per_tick, original.step_budget_per_tick);
    assert_eq!(copy.max_active_runs, original.max_active_runs);
    assert_eq!(copy.policy, original.policy);
}

#[test]
fn shard_config_debug_format_contains_field_names() {
    let config = ShardConfig::default();
    let debug_str = format!("{config:?}");
    // Debug output should contain the struct name and field identifiers.
    assert!(
        debug_str.contains("ShardConfig"),
        "Debug output should contain struct name: {debug_str}"
    );
    assert!(
        debug_str.contains("command_queue_capacity"),
        "Debug output should contain command_queue_capacity: {debug_str}"
    );
    assert!(
        debug_str.contains("trace_capacity"),
        "Debug output should contain trace_capacity: {debug_str}"
    );
    assert!(
        debug_str.contains("step_budget_per_tick"),
        "Debug output should contain step_budget_per_tick: {debug_str}"
    );
    assert!(
        debug_str.contains("max_active_runs"),
        "Debug output should contain max_active_runs: {debug_str}"
    );
}

#[test]
fn shard_config_new_rejects_zero_trace_capacity_in_lifecycle_chunk() {
    let result = ShardConfig::new(1, 0, 1, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "trace_capacity_zero"
        })
    );
}

#[test]
fn shard_config_new_rejects_zero_step_budget_in_lifecycle_chunk() {
    let result = ShardConfig::new(1, 1, 0, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::UnsupportedOperation {
            operation: "step_budget_per_tick_zero"
        })
    );
}

#[test]
fn shard_config_new_accepts_max_step_budget() {
    let result = ShardConfig::new(1, 1, u64::MAX, 1, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Ok(ShardConfig {
            command_queue_capacity: 1,
            trace_capacity: 1,
            step_budget_per_tick: u64::MAX,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        })
    );
}

#[test]
fn pending_timer_kind_equality_and_inequality() {
    assert_eq!(
        super::types::PendingTimerKind::Wait,
        super::types::PendingTimerKind::Wait
    );
    assert_eq!(
        super::types::PendingTimerKind::Ask,
        super::types::PendingTimerKind::Ask
    );
    assert_ne!(
        super::types::PendingTimerKind::Wait,
        super::types::PendingTimerKind::Ask
    );
}

#[test]
fn pending_timer_kind_debug_format() {
    let wait = super::types::PendingTimerKind::Wait;
    let ask = super::types::PendingTimerKind::Ask;
    let wait_debug = format!("{wait:?}");
    let ask_debug = format!("{ask:?}");
    assert!(
        wait_debug.contains("Wait"),
        "Wait debug should contain 'Wait': {wait_debug}"
    );
    assert!(
        ask_debug.contains("Ask"),
        "Ask debug should contain 'Ask': {ask_debug}"
    );
}

#[test]
fn pending_timer_equality_same_fields() {
    let deadline = std::time::Instant::now();
    let a = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(3),
        kind: super::types::PendingTimerKind::Wait,
        generation: 1,
        deadline,
    };
    let b = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(3),
        kind: super::types::PendingTimerKind::Wait,
        generation: 1,
        deadline,
    };
    assert_eq!(a, b);
}

#[test]
fn pending_timer_inequality_different_step() {
    let a = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(1),
        kind: super::types::PendingTimerKind::Ask,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let b = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(2),
        kind: super::types::PendingTimerKind::Ask,
        generation: 1,
        deadline: a.deadline,
    };
    assert_ne!(a, b);
}

#[test]
fn pending_timer_inequality_different_kind() {
    let a = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(5),
        kind: super::types::PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let b = super::types::PendingTimer {
        step: vb_core::ids::StepIdx::new(5),
        kind: super::types::PendingTimerKind::Ask,
        generation: 1,
        deadline: a.deadline,
    };
    assert_ne!(a, b);
}

#[test]
fn ask_ticket_equality_and_inequality() {
    let a = AskTicket {
        run: super::RunId::new(10),
        ask_step: vb_core::ids::StepIdx::new(1),
        resume_step: vb_core::ids::StepIdx::new(2),
    };
    let b = AskTicket {
        run: super::RunId::new(10),
        ask_step: vb_core::ids::StepIdx::new(1),
        resume_step: vb_core::ids::StepIdx::new(2),
    };
    assert_eq!(a, b);

    // Different run
    let c = AskTicket {
        run: super::RunId::new(11),
        ask_step: vb_core::ids::StepIdx::new(1),
        resume_step: vb_core::ids::StepIdx::new(2),
    };
    assert_ne!(a, c);

    // Different ask_step
    let d = AskTicket {
        run: super::RunId::new(10),
        ask_step: vb_core::ids::StepIdx::new(99),
        resume_step: vb_core::ids::StepIdx::new(2),
    };
    assert_ne!(a, d);

    // Different resume_step
    let e = AskTicket {
        run: super::RunId::new(10),
        ask_step: vb_core::ids::StepIdx::new(1),
        resume_step: vb_core::ids::StepIdx::new(99),
    };
    assert_ne!(a, e);
}

#[test]
fn inspect_snapshot_equality_and_debug() {
    let snap = InspectSnapshot {
        run: super::RunId::new(42),
        correlation: 7,
        pc: vb_core::ids::StepIdx::new(3),
        executed: 100,
    };
    let snap2 = InspectSnapshot {
        run: super::RunId::new(42),
        correlation: 7,
        pc: vb_core::ids::StepIdx::new(3),
        executed: 100,
    };
    assert_eq!(snap, snap2);

    let debug_str = format!("{snap:?}");
    assert!(
        debug_str.contains("InspectSnapshot"),
        "Debug should contain InspectSnapshot: {debug_str}"
    );
}

#[test]
fn max_command_queue_capacity_is_65536() {
    assert_eq!(MAX_COMMAND_QUEUE_CAPACITY, 65_536);
}

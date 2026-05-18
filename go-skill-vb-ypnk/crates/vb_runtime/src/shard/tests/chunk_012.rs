#[test]
fn shard_remaining_capacity_decrements_on_enqueue() {
    // Given a shard with capacity 4
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    assert_eq!(shard.remaining_capacity(), 4);
    // When enqueuing commands
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 3);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 2);
}

#[test]
fn shard_remaining_capacity_is_zero_when_full() {
    // Given a shard with capacity 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // Fill the queue
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then remaining capacity is 0
    assert_eq!(shard.remaining_capacity(), 0);
}

#[test]
fn shard_is_queue_full_returns_false_initially() {
    // Given a fresh shard
    let config = small_config();
    let shard = Shard::new(config);
    // Then queue is not full
    assert_eq!(shard.is_queue_full(), false);
}

#[test]
fn shard_is_queue_full_returns_true_when_at_capacity() {
    // Given a shard with capacity 2
    let config = ShardConfig {
        command_queue_capacity: 2,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // Fill the queue
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    // Then queue is full
    assert_eq!(shard.is_queue_full(), true);
}

#[test]
fn shard_command_queue_capacity_returns_configured_value() {
    // Given a shard configured with capacity 512
    let config = ShardConfig {
        command_queue_capacity: 512,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let shard = Shard::new(config);
    // Then the capacity method returns 512
    assert_eq!(shard.command_queue_capacity(), 512);
}

#[test]
fn shard_remaining_capacity_after_pop() {
    // Given a shard with capacity 4 and 2 commands enqueued
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.remaining_capacity(), 2);
    // When popping one command
    assert_eq!(shard.tick(), Ok(false)); // Shutdown causes tick to return false
}

#[test]
fn shard_queue_len_decrements_after_tick() {
    // Given a shard with a Cancel command queued
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    // Cancel for a non-existent run succeeds silently
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: super::RunId::new(999),
            reason: None
        }),
        Ok(())
    );
    assert_eq!(shard.command_queue_len(), 1);
    // When ticking
    assert_eq!(shard.tick(), Ok(true));
    // Then queue length is 0
    assert_eq!(shard.command_queue_len(), 0);
}

#[test]
fn shard_config_new_rejects_zero_command_queue_capacity() {
    let result = ShardConfig::new(0, 16, 4, 4, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: 0,
            max: MAX_COMMAND_QUEUE_CAPACITY
        })
    );
}

#[test]
fn shard_config_new_rejects_excessive_command_queue_capacity() {
    let result = ShardConfig::new(
        MAX_COMMAND_QUEUE_CAPACITY + 1,
        16,
        4,
        4,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert_eq!(
        result,
        Err(RuntimeError::CommandQueueCapacityExceeded {
            capacity: MAX_COMMAND_QUEUE_CAPACITY + 1,
            max: MAX_COMMAND_QUEUE_CAPACITY
        })
    );
}

#[test]
fn command_queue_capacity_predicate_matches_config_boundary() {
    assert_eq!(
        crate::shard::types::is_valid_command_queue_capacity(0),
        false
    );
    assert_eq!(
        crate::shard::types::is_valid_command_queue_capacity(1),
        true
    );
    assert_eq!(
        crate::shard::types::is_valid_command_queue_capacity(MAX_COMMAND_QUEUE_CAPACITY),
        true
    );
    assert_eq!(
        crate::shard::types::is_valid_command_queue_capacity(MAX_COMMAND_QUEUE_CAPACITY + 1),
        false
    );
}

#[test]
fn shard_config_new_rejects_zero_max_active_runs() {
    let result = ShardConfig::new(16, 16, 4, 0, vb_core::policy::RuntimePolicy::Relaxed);
    assert_eq!(result, Err(RuntimeError::ActiveRunCapacityZero));
}

#[test]
fn shard_config_new_accepts_valid_parameters() {
    let result = ShardConfig::new(
        1024,
        4096,
        1000,
        512,
        vb_core::policy::RuntimePolicy::Relaxed,
    );
    assert_eq!(
        result,
        Ok(ShardConfig {
            command_queue_capacity: 1024,
            trace_capacity: 4096,
            step_budget_per_tick: 1000,
            max_active_runs: 512,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        })
    );
}

#[test]
fn runtime_error_command_queue_capacity_exceeded_has_diagnostic_code() {
    let error = RuntimeError::CommandQueueCapacityExceeded {
        capacity: 100000,
        max: MAX_COMMAND_QUEUE_CAPACITY,
    };
    assert_eq!(
        error.diagnostic_code(),
        RuntimeError::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE
    );
}

#[test]
fn runtime_error_active_run_capacity_zero_has_diagnostic_code() {
    let error = RuntimeError::ActiveRunCapacityZero;
    assert_eq!(
        error.diagnostic_code(),
        RuntimeError::ACTIVE_RUN_CAPACITY_ZERO_CODE
    );
}

// =========================================================================
// Additional lifecycle tests — expanded coverage per handle_* method
// =========================================================================

/// Workflow: SetConst(slot1=2) -> Do(action=0, input=slot0) -> RetryCheck(policy_slot=slot1, body=step1, exhausted=step3) -> Finish(result=slot0)
/// Layout:
///   [0] SetConst(slot1 = const[0] = I64(2))
///   [1] Do(action=0, input=slot0)
///   [2] RetryCheck(policy_slot=slot1, body=step1, exhausted=step3)
///   [3] Finish(result=slot0)
fn do_with_retry_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_policy = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let action = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let retry_check = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(1),
            body: vb_core::ids::StepIdx::new(1),
            exhausted: vb_core::ids::StepIdx::new(3),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(3),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("do_with_retry"),
        digest: WorkflowDigest::from_bytes([6; 32]),
        nodes: Box::from([set_policy, action, retry_check, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(2)]),
        slot_count: 2,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

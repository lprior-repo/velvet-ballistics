
#[test]
fn shard_rejects_active_run_capacity_overflow() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };

    let first = shard.enqueue(ShardCommand::Submit {
        run: RunId::new(1),
        workflow: workflow.clone(),
        caps: vb_core::capability::CapabilitySet::empty(),
    });
    assert_eq!(first, Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    let second = shard.enqueue(ShardCommand::Submit {
        run: RunId::new(2),
        workflow,
        caps: vb_core::capability::CapabilitySet::empty(),
    });
    assert_eq!(second, Ok(()));
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
    );
}

#[test]
fn inspect_command_stores_retrievable_snapshot() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(7);

    let submitted = shard.enqueue(ShardCommand::Submit {
        run,
        workflow,
        caps: vb_core::capability::CapabilitySet::empty(),
    });
    assert_eq!(submitted, Ok(()));
    assert_eq!(shard.tick(), Ok(true));
    let inspected = shard.enqueue(ShardCommand::Inspect {
        run,
        correlation: 99,
    });
    assert_eq!(inspected, Ok(()));
    assert_eq!(shard.tick(), Ok(true));

    match shard.take_inspect_response() {
        Some(InspectResponse::Found(snapshot)) => {
            assert_eq!(snapshot.run, run);
            assert_eq!(snapshot.correlation, 99);
        }
        other => assert_eq!(other, None),
    }
}

#[test]
fn enqueue_shutdown_sets_shutting_down_flag() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(shard.is_shutting_down(), false);
    assert_eq!(shard.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(shard.tick(), Ok(false));
    assert_eq!(shard.is_shutting_down(), true);
}

#[test]
fn tick_returns_true_when_queue_is_empty() {
    let config = ShardConfig::default();
    let mut shard = Shard::new(config);
    assert_eq!(shard.tick(), Ok(true));
}

#[test]
fn cancel_nonexistent_run_returns_run_not_found() {
    let config = ShardConfig::default();
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(999),
        reason: None}),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
}

#[test]
fn counters_reflect_submitted_after_submit_tick() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(1);
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
}

#[test]
fn inspect_nonexistent_run_returns_not_found() {
    let config = ShardConfig {
        command_queue_capacity: 4,
        trace_capacity: 4,
        step_budget_per_tick: 4,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    assert_eq!(
        shard.enqueue(ShardCommand::Inspect {
            run: RunId::new(999),
            correlation: 42,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard.take_inspect_response(),
        Some(InspectResponse::NotFound {
            run: RunId::new(999),
            correlation: 42,
        })
    );
}

// Helper: workflow that finishes immediately (SetConst -> Finish).
fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn timed_wait_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: vb_core::ids::StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("timed_wait_then_finish"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: Box::from([set_deadline, wait, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

// Tests for frame release on submit error path (PO-vb-pymh-006).
//
// These tests verify that when `drive_run` fails during submit handling,
// the frame is properly released back to the pool.

// suspended_workflow() is defined in chunk_001.rs
// small_config() is defined in chunk_003.rs

/// Workflow that will cause drive_run to fail due to step budget exhaustion.
/// Uses max_step_budget_per_tick = 1 so a simple workflow exceeds it.
fn exhausted_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    // Create a chain of SetConst nodes that will exceed step budget
    let mut nodes = Vec::new();
    for i in 0..20 {
        nodes.push(CompiledNode {
            id: vb_core::ids::StepIdx::new(i),
            output: Some(SlotIdx::new(i)),
            next: if i < 19 {
                Some(vb_core::ids::StepIdx::new(i + 1))
            } else {
                None
            },
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
    }
    let parts = WorkflowParts {
        name: Box::from("exhausted"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from(nodes),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(false)]),
        slot_count: 20,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract {
            max_steps: 10_000,
            max_slots: 1_024,
            max_constants: u16::MAX,
            max_accessors: 8_192,
            max_expressions: 4_096,
            max_expr_stack: 64,
            max_step_budget_per_tick: 1, // Very low budget to trigger exhaustion
            max_transitions_per_tick: 10_000,
            max_input_bytes: 1_048_576,
            max_output_bytes: 262_144,
            max_blob_bytes: 16_777_216,
            max_ipc_payload_bytes: 1_048_576,
            max_retry_attempts: 3,
            max_fanout: 64,
            max_collect_items: 100,
            max_queue_depth: 100,
            max_journal_batch_bytes: 65_536,
            allows_secret_results: false,
        },
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

/// Test that submit_error_path_frame_release: when drive_run fails,
/// the frame is released back to the pool (PO-vb-pymh-006).
#[test]
fn submit_error_path_releases_frame_on_drive_failure() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 1, // Very low budget to trigger step exhaustion
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);
    let Some(workflow) = exhausted_workflow() else {
        return;
    };
    let run = RunId::new(100);

    // Before submit: frame pool dimension (1, 1) should have 0 available
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(FramePool::available),
        Some(0)
    );

    // Submit the workflow that will exhaust its step budget
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // Tick processes the submit - drive_run will fail due to budget
    // but the frame should still be released
    let result = shard.tick();
    // The exact error depends on implementation; we care that frame is released

    // After tick: frame should be released back to pool
    assert_eq!(
        shard.frame_pools.get(&(1, 1)).map(FramePool::available),
        Some(1)
    );

    // Run should not be in active runs (either completed or failed)
    assert_eq!(shard.run_state_contains(run), false);
}

/// Test that a workflow that causes EngineDriveFailed releases its frame.
#[test]
fn submit_engine_failure_releases_frame() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let Some(workflow) = exhausted_workflow() else {
        return;
    };
    let run = RunId::new(101);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // Before tick
    let before_available = shard.frame_pools.get(&(1, 1)).map(FramePool::available);

    shard.tick();

    // After tick: frame should be released
    let after_available = shard.frame_pools.get(&(1, 1)).map(FramePool::available);
    assert_eq!(after_available, Some(before_available.unwrap_or(0) + 1));
}

/// Test that multiple concurrent submit failures release all frames.
#[test]
fn multiple_submit_failures_release_all_frames() {
    let config = ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 1,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    let mut shard = Shard::new(config);

    let runs = [RunId::new(110), RunId::new(111), RunId::new(112)];

    for run in runs {
        let Some(workflow) = exhausted_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
                caps: vb_core::capability::CapabilitySet::empty()
            }),
            Ok(())
        );
    }

    // Process all submits
    for _ in 0..3 {
        shard.tick();
    }

    // All frames should be released back to pool
    let available = shard.frame_pools.get(&(1, 1)).map(FramePool::available);
    assert_eq!(available, Some(3));
}

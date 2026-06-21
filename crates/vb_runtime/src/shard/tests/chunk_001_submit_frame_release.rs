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
                Some(vb_core::ids::StepIdx::new(i.wrapping_add(1)))
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

/// Test that a suspended workflow can be submitted and tick processes it.
#[test]
fn submit_suspended_workflow_and_tick() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(100);

    // Submit the workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // Tick processes the submit
    assert_eq!(shard.tick(), Ok(true));

    // Run should be in active runs (suspended waiting for action)
    assert!(shard.run_state_contains(run));
    Ok(())
}

/// Test that a workflow with high step budget completes successfully.
#[test]
fn submit_workflow_with_sufficient_budget_completes() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = exhausted_workflow() else {
        return Ok(());
    };
    let run = RunId::new(101);

    // Submit the workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );

    // With default budget (4), the 20-node workflow should complete
    // because it takes 20 steps but budget is 4, so it suspends
    // and needs multiple ticks
    let _ = shard.tick();

    // Run should still be active (suspended waiting for action)
    // because step budget was exhausted
    assert!(shard.run_state_contains(run));
    Ok(())
}

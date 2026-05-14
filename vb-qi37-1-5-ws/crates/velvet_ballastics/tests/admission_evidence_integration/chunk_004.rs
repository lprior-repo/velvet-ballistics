
// ===========================================================================
// Test 5: budget validation rejects oversized workflow
// ===========================================================================

#[test]
fn budget_validation_rejects_oversized_workflow() {
    // Given: a BoundednessPolicy with very tight limits
    let tight_policy = vb_core::BoundednessPolicy {
        max_total_steps: 2,
        max_total_slots: 10,
        max_fanout: 1,
        max_nesting_depth: 1,
        absolute_max_action_tickets: 1,
        absolute_max_parallel: 1,
        absolute_max_run_time_seconds: 60,
        absolute_max_result_bytes: 1024,
        absolute_max_steps_executable: 2,
    };

    // When: creating a 3-node workflow that exceeds the step limit
    let budget = vb_core::WholeWorkflowBudget {
        max_total_steps: 3,
        max_total_slots: 5,
        max_fanout: 0,
        max_nesting_depth: 0,
        max_steps_executable: 3,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
    };

    // Then: validation rejects the budget
    match tight_policy.validate(&budget) {
        Err(vb_core::BudgetError::TotalStepsExceeded { actual, limit }) => {
            assert_eq!(actual, 3, "actual should be 3");
            assert_eq!(limit, 2, "limit should be 2");
        }
        Err(other) => {
            fail_assert!("expected TotalStepsExceeded, got {other:?}");
        }
        Ok(()) => {
            fail_assert!("3-step workflow should exceed a 2-step limit");
        }
    }

    // Also verify that compile_workflow rejects a workflow exceeding ResourceContract limits
    // by constructing a WorkflowParts with max_steps=1 but 2 nodes
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let mut tight_contract = ResourceContract::DEFAULT;
    tight_contract.max_steps = 1;
    let oversized_parts = WorkflowParts {
        name: Box::from("oversized"),
        digest: WorkflowDigest::from_bytes([5u8; 32]),
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: tight_contract,
        step_names: Box::default(),
    };

    // Then: construction should fail because 2 nodes > max_steps(1)
    let result = CompiledWorkflow::try_from_parts(oversized_parts);
    assert!(
        result.is_err(),
        "workflow with 2 nodes exceeding max_steps=1 should be rejected at construction"
    );

    // Verify slot limit rejection too
    let single_node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let mut slot_contract = ResourceContract::DEFAULT;
    slot_contract.max_slots = 0;
    let slot_parts = WorkflowParts {
        name: Box::from("slot_exceeded"),
        digest: WorkflowDigest::from_bytes([6u8; 32]),
        nodes: Box::from([single_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: slot_contract,
        step_names: Box::default(),
    };
    let slot_result = CompiledWorkflow::try_from_parts(slot_parts);
    assert!(
        slot_result.is_err(),
        "workflow with slot_count=1 exceeding max_slots=0 should be rejected"
    );
}

// ===========================================================================
// Test 6: taint propagates through expression eval
// ===========================================================================

#[test]
fn taint_propagates_through_expression_eval() {
    // Given: a workflow with an EvalExpr node that loads slot 0 into an expression
    let digest = WorkflowDigest::from_bytes([7u8; 32]);
    let Some(workflow) = eval_expr_taint_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };

    // Create a run frame, write a Secret-tainted value to slot 0
    let run_id = RunId::new(7);
    let mut frame = match vb_core::engine::new_run_frame(run_id, &workflow) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame creation failed: {err:?}");
            return;
        }
    };

    // Write a Secret-tainted value into slot 0 (the expression input)
    match frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(41), Taint::Secret) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("write_slot_with_taint failed: {err:?}");
            return;
        }
    }

    // Verify slot 0 is Secret
    match frame.read_taint(SlotIdx::new(0)) {
        Ok(Taint::Secret) => {}
        Ok(other) => {
            fail_assert!("slot 0 should be Secret, got {other:?}");
            return;
        }
        Err(err) => {
            fail_assert!("read_taint failed: {err:?}");
            return;
        }
    }

    // When: evaluating the expression that loads slot 0
    let mut store = vb_core::value_store::ValueStore::new();
    let eval_result =
        vb_core::engine::eval_expr_with_store(&workflow, &frame, &mut store, ExprIdx::new(0));

    // Then: the result value is correct and the taint has propagated
    match eval_result {
        Ok((value, taint)) => {
            // 41 (slot 0) + 1 (const 0) = 42
            assert_eq!(
                value,
                SlotValue::I64(42),
                "expression should compute 41 + 1 = 42"
            );
            assert_eq!(
                taint,
                Taint::Secret,
                "Secret taint from slot 0 should propagate through expression evaluation"
            );
        }
        Err(err) => {
            fail_assert!("eval_expr failed: {err:?}");
        }
    }

    // Also run through the engine to verify taint reaches the Finish signal
    // Reinitialize frame with the tainted slot
    let mut frame2 = match vb_core::engine::new_run_frame(run_id, &workflow) {
        Ok(f) => f,
        Err(err) => {
            fail_assert!("frame2 creation failed: {err:?}");
            return;
        }
    };
    match frame2.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(41), Taint::Secret) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("write_slot_with_taint failed: {err:?}");
            return;
        }
    }
    // Set the PC to step 0 and start execution
    let mut budget = vb_core::engine::StepBudget::new(100);
    let mut store2 = vb_core::value_store::ValueStore::new();
    let signal =
        vb_core::engine::drive_deterministic(&workflow, &mut frame2, &mut budget, &mut store2);

    match signal {
        Ok(vb_core::engine::EngineSignal::Finished(value, taint)) => {
            assert_eq!(value, SlotValue::I64(42));
            assert_eq!(
                taint,
                Taint::Secret,
                "finished signal taint should be Secret from expression propagation"
            );
        }
        Ok(other) => {
            fail_assert!("expected Finished signal, got {other:?}");
        }
        Err(err) => {
            fail_assert!("drive_deterministic failed: {err:?}");
        }
    }
}

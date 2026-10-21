// vb-3l40a: Admission contract validation tests with real Do workflows.
// Validates action contract IDs against actual workflow Do actions.
// Uses imports from chunk_001.rs via shared module scope.

fn contract_validation_test_shard() -> Shard {
    let config = ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    Shard::new(config)
}

fn contract_validation_make_contract(
    id: u16,
    name: &str,
) -> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
    let action_name = vb_core::action::ActionName::new(name)?;
    Ok(vb_core::action::ActionContract {
        id: ActionId::new(id),
        name: action_name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

/// Builds a SetConst->Do->Finish workflow that seeds the Do input from a constant.
fn contract_validation_setconst_do_workflow(action_id: u16) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let do_node = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(action_id),
            input: SlotIdx::ZERO,
        },
    };
    let finish_node = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("setconst-do-workflow"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([set_const, do_node, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| -> Box<dyn std::error::Error> {
            format!("test workflow must compile: {}", e).into()
        })
}

/// Builds a simple Do workflow for validation-only tests (no SetConst).
fn contract_validation_do_workflow(action_id: u16) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(action_id),
            input: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("test-do-workflow"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| -> Box<dyn std::error::Error> {
            format!("test workflow must compile: {}", e).into()
        })
}

#[test]
fn validate_contracts_accepts_valid_do_contract() -> Result<(), Box<dyn std::error::Error>> {
    let shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(1);
    let caps = CapabilitySet::empty();

    let contract = contract_validation_make_contract(0, "real_action")?;
    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract],
    );
    assert!(
        result.is_ok(),
        "valid Do contract must be accepted: {:?}",
        result
    );
    Ok(())
}

#[test]
fn validate_contracts_rejects_fabricated_action_id() -> Result<(), Box<dyn std::error::Error>> {
    let shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(1);
    let caps = CapabilitySet::empty();

    // Contract with ID 999 at index 0 - fails ID matches index check
    let contract = contract_validation_make_contract(999, "not_in_workflow")?;
    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(999), "error should mention fabricated action ID");
            assert!(error.contains("999"), "error should mention fabricated ID: {}", error);
        }
        other => {
            return Err(format!(
                "expected ContractValidationError, got: {:?}",
                other
            ).into());
        }
    }
    Ok(())
}

#[test]
fn validate_contracts_rejects_duplicate_action_ids() -> Result<(), Box<dyn std::error::Error>> {
    let shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(1);
    let caps = CapabilitySet::empty();

    // Two contracts with ID 0 - second one has ID != index
    let contract = contract_validation_make_contract(0, "foo")?;
    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract.clone(), contract.clone()],
    );
    match result {
        Err(RuntimeError::ContractValidationError { .. }) => {}
        other => {
            return Err(format!(
                "expected ContractValidationError for duplicates, got: {:?}",
                other
            ).into());
        }
    }
    Ok(())
}

#[test]
fn validate_contracts_rejects_missing_capability() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::capability::Capability;

    let shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(1);
    let caps = CapabilitySet::empty();

    let required = Capability::new("needed".into(), ActionId::new(0));
    let contract = vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("needs_cap")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::from([required]),
    };

    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract],
    );
    match result {
        Err(RuntimeError::AdmissionCapabilityDenied { required, .. }) => {
            assert_eq!(required.name(), "needed");
        }
        other => {
            return Err(format!(
                "expected AdmissionCapabilityDenied, got: {:?}",
                other
            ).into());
        }
    }
    Ok(())
}

#[test]
fn validate_contracts_accepts_with_capability() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::capability::Capability;

    let shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(1);

    let cap = Capability::new("needed".into(), ActionId::new(0));
    let caps = CapabilitySet::from_grants(Box::from([cap]));

    let contract = vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("needs_cap")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::from([
            Capability::new("needed".into(), ActionId::new(0)),
        ]),
    };

    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract],
    );
    assert!(
        result.is_ok(),
        "contract with satisfied capability must be accepted: {:?}",
        result
    );
    Ok(())
}

#[test]
fn validate_contracts_rejection_is_non_mutating() -> Result<(), Box<dyn std::error::Error>> {
    let shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(1);
    let caps = CapabilitySet::empty();

    // Contract references non-existent action 999
    let contract = contract_validation_make_contract(999, "not_in_workflow")?;

    // Rejection should not create any run state
    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract],
    );
    assert!(result.is_err());

    // Run should not exist in shard after rejection
    assert!(!shard.run_state_contains(run), "rejection must not mutate run state");
    Ok(())
}

#[test]
fn validate_contracts_accepts_reused_action_ids() -> Result<(), Box<dyn std::error::Error>> {
    // Workflow with same action appearing twice
    let node1 = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let node2 = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("reused-action"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([node1, node2]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts)?;

    let shard = contract_validation_test_shard();
    let run = RunId::new(1);
    let caps = CapabilitySet::empty();

    // Single contract for action 0 covers both occurrences
    let contract = contract_validation_make_contract(0, "reused")?;
    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        caps,
        &[contract],
    );
    assert!(
        result.is_ok(),
        "reused action IDs must be accepted: {:?}",
        result
    );
    Ok(())
}

// vb-3l40a: Direct shard boundary validation tests.
// Proves that contracts are validated at the shard entry methods before
// any run admission, header, or journal mutation when enqueued directly.

#[test]
fn shard_boundary_validates_fabricated_id_before_admission() -> Result<(), Box<dyn std::error::Error>> {
    use crate::shard::ShardCommand;

    let mut shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(100);
    let caps = CapabilitySet::empty();

    // Contract with ID 999 at index 0 - fabricated ID.
    let contract = contract_validation_make_contract(999, "fabricated")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    // Enqueue directly to shard, bypassing Runtime preflight.
    shard.enqueue(ShardCommand::SubmitWithContracts {
        run,
        workflow,
        caps,
        action_contracts,
    })?;

    // Drive the shard to process the command. Tick returns Err with validation error.
    let tick_result = shard.tick();
    match tick_result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(999), "error should mention fabricated action ID");
            assert!(error.contains("999"), "error should mention fabricated ID: {}", error);
        }
        other => {
            return Err(format!(
                "expected Err(ContractValidationError), got: {:?}",
                other
            ).into());
        }
    }

    // Run should NOT exist - validation must reject before admission.
    assert!(
        !shard.run_state_contains(run),
        "fabricated contract must be rejected before run admission"
    );

    Ok(())
}

#[test]
fn shard_boundary_validates_fabricated_id_with_inputs_before_admission() -> Result<(), Box<dyn std::error::Error>> {
    use crate::shard::ShardCommand;

    let mut shard = contract_validation_test_shard();
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(102);
    let caps = CapabilitySet::empty();
    let inputs: Box<[(SlotIdx, SlotValue)]> = Box::from([]);

    // Contract with ID 777 at index 0 - fabricated ID.
    let contract = contract_validation_make_contract(777, "fabricated")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    // Enqueue directly to shard with inputs variant.
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        inputs,
        caps,
        action_contracts,
    })?;

    // Tick returns Err with validation error.
    let tick_result = shard.tick();
    match tick_result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(777), "error should mention fabricated action ID");
            assert!(error.contains("777"), "error should mention fabricated ID: {}", error);
        }
        other => {
            return Err(format!(
                "expected Err(ContractValidationError) for inputs variant, got: {:?}",
                other
            ).into());
        }
    }

    // Run should NOT exist - validation must reject before admission.
    assert!(
        !shard.run_state_contains(run),
        "fabricated contract must be rejected before run admission (inputs variant)"
    );

    Ok(())
}

#[test]
fn shard_boundary_accepts_valid_contract_and_executes_do() -> Result<(), Box<dyn std::error::Error>> {
    use crate::shard::ShardCommand;
    use vb_core::action::ActionOutputReady;

    let mut shard = contract_validation_test_shard();
    // Use SetConst->Do->Finish workflow so Do has real input.
    let workflow = contract_validation_setconst_do_workflow(0)?;

    let run = RunId::new(101);
    let caps = CapabilitySet::empty();

    // Valid contract with ID 0 at index 0.
    let contract = contract_validation_make_contract(0, "valid_action")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    // Enqueue directly to shard.
    shard.enqueue(ShardCommand::SubmitWithContracts {
        run,
        workflow,
        caps,
        action_contracts,
    })?;

    // Tick processes SetConst, then Do schedules an action.
    let tick1 = shard.tick()?;
    assert!(tick1, "first tick should make progress");

    // Action should be pending.
    let ticket = shard
        .pending_action_get(run)
        .ok_or("Do action must be scheduled after tick")?;
    assert_eq!(ticket.run, run);
    assert_eq!(ticket.step, StepIdx::new(1));
    assert_eq!(ticket.action, ActionId::new(0));

    // Complete the action with a real result.
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(100),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 3,
    };
    shard.enqueue(ShardCommand::ActionCompleted {
        ticket,
        output,
    })?;

    // Tick processes completion and Finish.
    let tick2 = shard.tick()?;
    assert!(tick2, "tick after completion should make progress");

    // Run should be terminal (finished successfully).
    assert!(
        shard.terminal_runs_contains(run),
        "run should be terminal after action completion and finish"
    );
    assert!(
        !shard.run_state_contains(run),
        "run should be removed from active state after terminal completion"
    );

    Ok(())
}

#[test]
fn shard_boundary_accepts_valid_contract_with_seeded_input() -> Result<(), Box<dyn std::error::Error>> {
    use crate::shard::ShardCommand;
    use vb_core::action::ActionOutputReady;

    let mut shard = contract_validation_test_shard();
    // Use simple Do workflow, seed input via inputs parameter.
    let workflow = contract_validation_do_workflow(0)?;

    let run = RunId::new(103);
    let caps = CapabilitySet::empty();

    // Seed the Do input slot with a real value.
    let inputs: Box<[(SlotIdx, SlotValue)]> = Box::from([(
        SlotIdx::ZERO,
        SlotValue::I64(55),
    )]);

    let contract = contract_validation_make_contract(0, "valid_action")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    // Enqueue directly to shard with inputs variant.
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        inputs,
        caps,
        action_contracts,
    })?;

    // Tick processes Do and schedules action.
    let tick1 = shard.tick()?;
    assert!(tick1, "first tick should make progress");

    // Action should be pending.
    let ticket = shard
        .pending_action_get(run)
        .ok_or("Do action must be scheduled after tick with seeded input")?;
    assert_eq!(ticket.run, run);
    assert_eq!(ticket.step, StepIdx::ZERO);
    assert_eq!(ticket.action, ActionId::new(0));

    // Complete the action with a real result.
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(200),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 3,
    };
    shard.enqueue(ShardCommand::ActionCompleted {
        ticket,
        output,
    })?;

    // Tick processes completion.
    let tick2 = shard.tick()?;
    assert!(tick2, "tick after completion should make progress");

    // Run should be terminal (finished).
    assert!(
        shard.terminal_runs_contains(run),
        "run should be terminal after action completion with seeded input"
    );

    Ok(())
}

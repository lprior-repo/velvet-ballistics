// vb-3l40a: Admission contract validation tests with real Do workflows.
// Validates action contract IDs against actual workflow Do actions.
// Uses imports from chunk_001.rs via shared module scope.

use vb_core::policy::RuntimePolicy;
use vb_core::workflow::CompiledInputSlot;
use vb_core::workflow::InputSlotKind;

fn test_shard() -> Shard {
    let config = ShardConfig {
        command_queue_capacity: 64,
        trace_capacity: 64,
        step_budget_per_tick: 4,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    };
    Shard::new(config)
}

fn make_contract(
    id: u16,
    name: &str,
) -> Result<vb_core::action::ActionContract, Box<dyn std::error::Error>> {
    Ok(vb_core::action::ActionContract {
        id: ActionId::new(id),
        name: vb_core::action::ActionName::new(name)?,
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

fn do_workflow(action_id: u16) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
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
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("test-do"),
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
    })
    .map_err(|e| format!("workflow must compile: {}", e).into())
}

/// SetConst->Do->Finish workflow with real input for Do.
fn setconst_do_workflow(action_id: u16) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let set = CompiledNode {
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
    let finish = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("setconst-do"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set, do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    })
    .map_err(|e| format!("workflow must compile: {}", e).into())
}

/// Do->Finish workflow with typed input slot for seeded input tests.
fn input_slot_do_workflow(action_id: u16) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(action_id),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("input-do"),
        digest: WorkflowDigest::from_bytes([5; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::I64,
        }]),
    })
    .map_err(|e| format!("workflow must compile: {}", e).into())
}

#[test]
fn validate_contracts_accepts_valid_do_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract = make_contract(0, "real_action")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
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
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract = make_contract(999, "not_in_workflow")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(999));
            assert_eq!(error, "contract at index 0 has ID 999 (expected 0)");
        }
        other => return Err(format!("expected ContractValidationError, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn validate_contracts_rejects_duplicate_action_ids() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract = make_contract(0, "foo")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract.clone(), contract.clone()],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(0));
            assert_eq!(error, "duplicate action ID");
        }
        other => return Err(format!("expected ContractValidationError, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn validate_contracts_rejects_missing_capability() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let required = vb_core::capability::Capability::new("needed".into(), ActionId::new(0));
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
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract],
    );
    match result {
        Err(RuntimeError::AdmissionCapabilityDenied { required, .. }) => {
            assert_eq!(required.name(), "needed");
        }
        other => {
            return Err(format!("expected AdmissionCapabilityDenied, got: {:?}", other).into())
        }
    }
    Ok(())
}

#[test]
fn validate_contracts_accepts_with_capability() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let cap = vb_core::capability::Capability::new("needed".into(), ActionId::new(0));
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
        required_capabilities: Box::from([vb_core::capability::Capability::new(
            "needed".into(),
            ActionId::new(0),
        )]),
    };
    let result =
        shard.validate_submit_admission_with_contracts(RunId::new(1), &workflow, caps, &[contract]);
    assert!(
        result.is_ok(),
        "contract with satisfied capability must be accepted: {:?}",
        result
    );
    Ok(())
}

#[test]
fn validate_contracts_rejection_is_non_mutating() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract = make_contract(999, "not_in_workflow")?;
    let run = RunId::new(1);
    let result = shard.validate_submit_admission_with_contracts(
        run,
        &workflow,
        CapabilitySet::empty(),
        &[contract],
    );
    assert!(result.is_err());
    assert!(
        !shard.run_state_contains(run),
        "rejection must not mutate run state"
    );
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
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("reused"),
        digest: WorkflowDigest::from_bytes([3; 32]),
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
    })?;

    let mut shard = test_shard();
    let contract = make_contract(0, "reused")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract],
    );
    assert!(
        result.is_ok(),
        "reused action IDs must be accepted: {:?}",
        result
    );
    Ok(())
}

#[test]
fn validate_contracts_rejects_empty_action_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    // Use postcard to deserialize an empty ActionName (constructor rejects "")
    let empty_bytes = b"\x00\x00";
    let empty_name: vb_core::action::ActionName = postcard::from_bytes(empty_bytes)?;
    let contract = vb_core::action::ActionContract {
        id: ActionId::new(0),
        name: empty_name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    };
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(0));
            assert_eq!(error, "action name is empty");
        }
        other => return Err(format!("expected ContractValidationError, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn validate_contracts_accepts_multiple_distinct_action_ids(
) -> Result<(), Box<dyn std::error::Error>> {
    // Workflow with two distinct actions
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
            action: ActionId::new(1),
            input: SlotIdx::new(1),
        },
    };
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("multi-action"),
        digest: WorkflowDigest::from_bytes([4; 32]),
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
    })?;

    let mut shard = test_shard();
    let contract0 = make_contract(0, "first")?;
    let contract1 = make_contract(1, "second")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract0, contract1],
    );
    assert!(
        result.is_ok(),
        "multiple distinct action IDs must be accepted: {:?}",
        result
    );
    Ok(())
}

#[test]
fn validate_contracts_rejects_missing_referenced_action() -> Result<(), Box<dyn std::error::Error>>
{
    let mut shard = test_shard();
    let workflow = do_workflow(10)?;
    let contract = make_contract(0, "only_action")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(10));
            assert_eq!(error, "no contract for workflow Do action");
        }
        other => return Err(format!("expected ContractValidationError, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn validate_contracts_accepts_unused_dense_entries() -> Result<(), Box<dyn std::error::Error>> {
    // Registry with 3 contracts, workflow only uses action 0
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract0 = make_contract(0, "used")?;
    let contract1 = make_contract(1, "unused1")?;
    let contract2 = make_contract(2, "unused2")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract0, contract1, contract2],
    );
    assert!(
        result.is_ok(),
        "unused dense entries must be accepted: {:?}",
        result
    );
    Ok(())
}

#[test]
fn validate_contracts_rejects_sparse_registry() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract0 = make_contract(0, "ok")?;
    let contract2 = make_contract(2, "sparse")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract0, contract2],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(2));
            assert_eq!(error, "contract at index 1 has ID 2 (expected 1)");
        }
        other => return Err(format!("expected ContractValidationError, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn validate_contracts_duplicate_precedes_index_mismatch() -> Result<(), Box<dyn std::error::Error>>
{
    // Two contracts with ID 3, at indices 0 and 1 — duplicate must be detected
    // before the index mismatch at index 0
    let mut shard = test_shard();
    let workflow = do_workflow(0)?;
    let contract_a = make_contract(3, "dup")?;
    let contract_b = make_contract(3, "dup2")?;
    let result = shard.validate_submit_admission_with_contracts(
        RunId::new(1),
        &workflow,
        CapabilitySet::empty(),
        &[contract_a, contract_b],
    );
    match result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(3));
            assert_eq!(error, "duplicate action ID");
        }
        other => return Err(format!("expected duplicate error, got: {:?}", other).into()),
    }
    Ok(())
}

// Direct shard boundary validation tests with journal immutability checks.

#[test]
fn shard_boundary_validates_fabricated_id_before_admission(
) -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(
        ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: RuntimePolicy::Relaxed,
        },
        journal.clone(),
    );
    let workflow = do_workflow(0)?;
    let run = RunId::new(100);
    let contract = make_contract(999, "fabricated")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    let snapshot = journal.snapshot()?;
    shard.enqueue(ShardCommand::SubmitWithContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts,
    })?;
    let tick_result = shard.tick();
    match tick_result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(999));
            assert_eq!(error, "contract at index 0 has ID 999 (expected 0)");
        }
        other => {
            return Err(format!("expected Err(ContractValidationError), got: {:?}", other).into())
        }
    }
    assert!(!shard.run_state_contains(run), "run must not be admitted");
    assert!(
        !shard.terminal_runs_contains(run),
        "run must not be terminal"
    );
    let after = journal.snapshot()?;
    assert_eq!(
        snapshot, after,
        "journal must be unchanged after validation failure"
    );
    Ok(())
}

#[test]
fn shard_boundary_validates_fabricated_id_with_inputs_before_admission(
) -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(
        ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: RuntimePolicy::Relaxed,
        },
        journal.clone(),
    );
    let workflow = do_workflow(0)?;
    let run = RunId::new(102);
    let contract = make_contract(777, "fabricated")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);
    let inputs: Box<[(SlotIdx, SlotValue)]> = Box::from([]);

    let snapshot = journal.snapshot()?;
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        inputs,
        caps: CapabilitySet::empty(),
        action_contracts,
    })?;
    let tick_result = shard.tick();
    match tick_result {
        Err(RuntimeError::ContractValidationError { action, error }) => {
            assert_eq!(action, ActionId::new(777));
            assert_eq!(error, "contract at index 0 has ID 777 (expected 0)");
        }
        other => {
            return Err(format!("expected Err(ContractValidationError), got: {:?}", other).into())
        }
    }
    assert!(!shard.run_state_contains(run), "run must not be admitted");
    assert!(
        !shard.terminal_runs_contains(run),
        "run must not be terminal"
    );
    let after = journal.snapshot()?;
    assert_eq!(
        snapshot, after,
        "journal must be unchanged after validation failure"
    );
    Ok(())
}

#[test]
fn shard_boundary_accepts_valid_contract_and_executes_do() -> Result<(), Box<dyn std::error::Error>>
{
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(
        ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: RuntimePolicy::Relaxed,
        },
        journal.clone(),
    );
    let workflow = setconst_do_workflow(0)?;
    let run = RunId::new(101);
    let contract = make_contract(0, "valid_action")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    shard.enqueue(ShardCommand::SubmitWithContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts,
    })?;

    let tick1 = shard.tick()?;
    assert!(tick1, "first tick should make progress");

    let ticket = shard
        .pending_action_get(run)
        .ok_or("Do action must be scheduled after tick")?;
    assert_eq!(ticket.run, run);
    assert_eq!(ticket.step, StepIdx::new(1));
    assert_eq!(ticket.action, ActionId::new(0));
    assert_eq!(ticket.attempt, 1, "first attempt must be 1");
    let run_state = shard
        .run_state_get(run)
        .ok_or("run state must exist after tick")?;
    assert_eq!(
        run_state.frame.read_slot(SlotIdx::ZERO)?,
        &SlotValue::I64(42),
        "Do input slot must hold the SetConst value 42"
    );
    assert_eq!(
        run_state.frame.read_taint(SlotIdx::ZERO)?,
        vb_core::value::Taint::Clean,
        "Do input slot must be Clean-tainted"
    );

    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(100),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 3,
    };
    shard.enqueue(ShardCommand::ActionCompleted { ticket, output })?;

    let tick2 = shard.tick()?;
    assert!(tick2, "tick after completion should make progress");

    assert!(shard.terminal_runs_contains(run), "run must be terminal");
    assert!(
        !shard.run_state_contains(run),
        "run must be removed from active state"
    );

    // Verify journal evidence: RunFinished, no RunFailed
    let events = journal.snapshot()?;
    let run_finished: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFinished { .. }))
        .collect();
    assert_eq!(
        run_finished.len(),
        1,
        "exactly one RunFinished journal event expected"
    );
    assert!(matches!(run_finished.first(),
        Some(crate::journal::RuntimeJournalEvent::RunFinished { run: r, result })
            if *r == run && *result == SlotIdx::new(1)));

    let run_failed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFailed { .. }))
        .collect();
    assert_eq!(run_failed.len(), 0, "no RunFailed journal events expected");

    // Verify ActionCompletedEnvelope with correct output value (100)
    let completed: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            crate::journal::RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                ..
            } => Some((*ticket, *output, value.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        completed.len(),
        1,
        "exactly one ActionCompletedEnvelope expected"
    );
    if let Some((env_ticket, env_output, env_value)) = completed.first() {
        assert_eq!(*env_ticket, ticket);
        assert_eq!(*env_output, SlotIdx::new(1));
        let expected_value = postcard::to_allocvec(&SlotValue::I64(100))?;
        assert_eq!(
            env_value, &expected_value,
            "ActionCompletedEnvelope value must be 100"
        );
    }

    Ok(())
}

#[test]
fn shard_boundary_accepts_valid_contract_with_seeded_input(
) -> Result<(), Box<dyn std::error::Error>> {
    let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(
        ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            policy: RuntimePolicy::Relaxed,
        },
        journal.clone(),
    );
    let workflow = input_slot_do_workflow(0)?;
    let run = RunId::new(103);
    let inputs: Box<[(SlotIdx, SlotValue)]> = Box::from([(SlotIdx::ZERO, SlotValue::I64(55))]);
    let contract = make_contract(0, "valid_action")?;
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::from([contract]);

    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        inputs,
        caps: CapabilitySet::empty(),
        action_contracts,
    })?;

    let tick1 = shard.tick()?;
    assert!(tick1, "first tick should make progress");

    let ticket = shard
        .pending_action_get(run)
        .ok_or("Do action must be scheduled after tick with seeded input")?;
    assert_eq!(ticket.run, run);
    assert_eq!(ticket.step, StepIdx::ZERO);
    assert_eq!(ticket.action, ActionId::new(0));
    assert_eq!(ticket.attempt, 1, "first attempt must be 1");
    let run_state = shard
        .run_state_get(run)
        .ok_or("run state must exist after tick")?;
    assert_eq!(
        run_state.frame.read_slot(SlotIdx::ZERO)?,
        &SlotValue::I64(55),
        "Do input slot must hold the seeded value 55"
    );
    assert_eq!(
        run_state.frame.read_taint(SlotIdx::ZERO)?,
        vb_core::value::Taint::Clean,
        "Do input slot must be Clean-tainted"
    );

    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(200),
        taint: vb_core::value::Taint::Clean,
        encoded_len: 3,
    };
    shard.enqueue(ShardCommand::ActionCompleted { ticket, output })?;

    let tick2 = shard.tick()?;
    assert!(tick2, "tick after completion should make progress");
    assert!(shard.terminal_runs_contains(run), "run must be terminal");

    // Verify journal evidence: RunFinished, no RunFailed
    let events = journal.snapshot()?;
    let run_finished: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFinished { .. }))
        .collect();
    assert_eq!(
        run_finished.len(),
        1,
        "exactly one RunFinished journal event expected"
    );
    assert!(matches!(run_finished.first(),
        Some(crate::journal::RuntimeJournalEvent::RunFinished { run: r, result })
            if *r == run && *result == SlotIdx::new(1)));

    let run_failed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, crate::journal::RuntimeJournalEvent::RunFailed { .. }))
        .collect();
    assert_eq!(run_failed.len(), 0, "no RunFailed journal events expected");

    // Verify ActionCompletedEnvelope with correct output value (200)
    let completed: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            crate::journal::RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket,
                output,
                value,
                ..
            } => Some((*ticket, *output, value.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        completed.len(),
        1,
        "exactly one ActionCompletedEnvelope expected"
    );
    if let Some((env_ticket, env_output, env_value)) = completed.first() {
        assert_eq!(*env_ticket, ticket);
        assert_eq!(*env_output, SlotIdx::new(1));
        let expected_value = postcard::to_allocvec(&SlotValue::I64(200))?;
        assert_eq!(
            env_value, &expected_value,
            "ActionCompletedEnvelope value must be 200"
        );
    }

    Ok(())
}

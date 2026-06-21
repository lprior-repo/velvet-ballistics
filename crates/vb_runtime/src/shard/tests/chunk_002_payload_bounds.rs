// Tests for ask answer payload boundary conditions (PO-vb-pymh-009).
//
// These tests verify that AskAnswer encoded_len boundary conditions are enforced:
// - ask_answer_boundary_max_payload: encoded_len == max_ipc_payload_bytes → Ok
// - ask_answer_boundary_zero_payload: encoded_len == 0 → Ok
// - ask_answer_boundary_max_plus_one: encoded_len == max + 1 → Err(IpcPayloadSizeExceeded)
// - ask_answer_payload_size_exceeded_error: encoded_len > max_ipc_payload_bytes → Err

// suspended_workflow() is defined in chunk_001.rs
// small_config() is defined in chunk_003.rs

/// Workflow with an Ask.
fn ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: None,
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
        name: Box::from("ask"),
        digest: WorkflowDigest::from_bytes([9; 32]),
        nodes: Box::from([set_prompt, ask, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1))]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

/// Workflow with a custom max_ipc_payload_bytes.
fn ask_wait_with_max_payload(max_bytes: u32) -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: None,
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
        name: Box::from("ask_wait_custom_payload"),
        digest: WorkflowDigest::from_bytes([10; 32]),
        nodes: Box::from([set_prompt, ask, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1))]),
        slot_count: 1,
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
            max_step_budget_per_tick: 10_000,
            max_transitions_per_tick: 10_000,
            max_input_bytes: 1_048_576,
            max_output_bytes: 262_144,
            max_blob_bytes: 16_777_216,
            max_ipc_payload_bytes: max_bytes, // Custom limit
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

/// Workflow with allows_secret_results = true.
fn ask_wait_with_secret_results_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let ask = CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: None,
        next: Some(vb_core::ids::StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: None,
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
        name: Box::from("ask_wait_secret"),
        digest: WorkflowDigest::from_bytes([11; 32]),
        nodes: Box::from([set_prompt, ask, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1))]),
        slot_count: 1,
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
            max_step_budget_per_tick: 10_000,
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
            allows_secret_results: true, // Allow secret results
        },
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

/// Test ask_answer_boundary_max_payload: encoded_len == max_ipc_payload_bytes → Ok.
#[test]
fn ask_answer_payload_at_max_boundary_is_accepted() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    // Create workflow with max_ipc_payload_bytes = 100
    let Some(workflow) = ask_wait_with_max_payload(100) else {
        return;
    };
    let run = RunId::new(200);

    // Submit the workflow
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    // Get the pending timer to build a valid AskAnswer
    let pending_timer = shard.pending_timer_get(run).expect("should have pending ask timer");
    assert_eq!(pending_timer.kind, PendingTimerKind::Ask);

    // Create answer with encoded_len == max (100)
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: pending_timer.step,
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Clean,
        100, // Exactly at max
    );

    // Enqueue the answer
    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );

    // Tick should process successfully
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

/// Test ask_answer_boundary_zero_payload: encoded_len == 0 → Ok.
#[test]
fn ask_answer_payload_of_zero_is_accepted() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(201);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let pending_timer = shard.pending_timer_get(run).expect("should have pending ask timer");

    // Create answer with encoded_len == 0
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: pending_timer.step,
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Clean,
        0, // Zero length
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

/// Test ask_answer_boundary_max_plus_one: encoded_len == max + 1 → Err(IpcPayloadSizeExceeded).
#[test]
fn ask_answer_payload_exceeding_max_by_one_is_rejected() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    // Create workflow with max_ipc_payload_bytes = 100
    let Some(workflow) = ask_wait_with_max_payload(100) else {
        return;
    };
    let run = RunId::new(202);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let pending_timer = shard.pending_timer_get(run).expect("should have pending ask timer");

    // Create answer with encoded_len == max + 1 (101)
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: pending_timer.step,
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Clean,
        101, // One over max
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );

    // Tick should return IpcPayloadSizeExceeded error
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::IpcPayloadSizeExceeded {
            size: 101,
            max: 100
        })
    );
    Ok(())
}

/// Test ask_answer_payload_size_exceeded_error: encoded_len > max_ipc_payload_bytes → Err.
#[test]
fn ask_answer_payload_significantly_over_max_is_rejected() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    // Create workflow with max_ipc_payload_bytes = 100
    let Some(workflow) = ask_wait_with_max_payload(100) else {
        return;
    };
    let run = RunId::new(203);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let pending_timer = shard.pending_timer_get(run).expect("should have pending ask timer");

    // Create answer with encoded_len way over max (1000 > 100)
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: pending_timer.step,
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Clean,
        1000, // Way over max
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );

    assert_eq!(
        shard.tick(),
        Err(RuntimeError::IpcPayloadSizeExceeded {
            size: 1000,
            max: 100
        })
    );
    Ok(())
}

/// Test that secret taint is rejected when allows_secret_results is false.
#[test]
fn ask_answer_secret_taint_rejected_when_not_allowed() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;
    let Some(workflow) = ask_workflow() else {
        return;
    };
    let run = RunId::new(204);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let pending_timer = shard.pending_timer_get(run).expect("should have pending ask timer");

    // Create answer with Secret taint but contract doesn't allow it
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: pending_timer.step,
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Secret, // Secret taint
        10,
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );

    // Should be rejected because allows_secret_results = false by default
    assert_eq!(shard.tick(), Err(RuntimeError::SecretResultNotAllowed));
    Ok(())
}

/// Test that secret taint is accepted when allows_secret_results is true.
#[test]
fn ask_answer_secret_taint_accepted_when_explicitly_allowed() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    // Create workflow with allows_secret_results = true
    let custom_workflow = ask_wait_with_secret_results_workflow();
    let Some(workflow) = custom_workflow else {
        return;
    };
    let run = RunId::new(205);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty()
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let pending_timer = shard.pending_timer_get(run).expect("should have pending ask timer");

    // Create answer with Secret taint
    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run,
            ask_step: pending_timer.step,
            resume_step: vb_core::ids::StepIdx::new(2),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Secret,
        10,
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );

    // Should be accepted because allows_secret_results = true
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

/// Test ask_answer_run_not_found_error: Answer for vanished run → Err(RunNotFound).
#[test]
fn ask_answer_for_nonexistent_run_returns_not_found() -> Result<(), RuntimeError> {
    let config = small_config();
    let mut shard = Shard::new(config)?;

    let answer = AskAnswer::with_encoded_len(
        AskTicket {
            run: RunId::new(999), // Non-existent run
            ask_step: vb_core::ids::StepIdx::ZERO,
            resume_step: vb_core::ids::StepIdx::new(1),
        },
        SlotIdx::ZERO,
        vb_core::value::SlotValue::Symbol(vb_core::ids::SymbolId::new(1)),
        vb_core::value::Taint::Clean,
        10,
    );

    assert_eq!(
        shard.enqueue(ShardCommand::AskAnswered { answer }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}
